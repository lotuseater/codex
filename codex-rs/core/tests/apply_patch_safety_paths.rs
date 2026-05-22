#![allow(clippy::expect_used)]

mod support;

#[path = "suite/apply_patch_cli.rs"]
mod apply_patch_cli;
#[path = "suite/apply_patch_harness.rs"]
mod apply_patch_harness;

use anyhow::Result;
use apply_patch_cli::create_file_symlink;
use apply_patch_cli::restrictive_workspace_write_profile;
use apply_patch_cli::workspace_write_with_read_only_root;
#[cfg(unix)]
use apply_patch_cli::workspace_write_with_unreadable_path;
use apply_patch_harness::apply_patch_harness;
use apply_patch_harness::apply_patch_harness_with;
use apply_patch_harness::mount_apply_patch;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::skip_if_remote;
use codex_core_test_runtime::test_codex::ApplyPatchModelOutput;
use codex_features::Feature;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use test_case::test_case;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_delete_directory_reports_verification_error(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    harness.create_dir_all("dir").await?;

    let patch = "*** Begin Patch\n*** Delete File: dir\n*** End Patch";
    let call_id = "apply-delete-dir";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("delete a directory via apply_patch").await?;

    let out = harness.apply_patch_output(call_id, model_output).await;
    assert!(out.contains("apply_patch verification failed"));
    assert!(out.contains("Failed to read"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_rejects_path_traversal_outside_workspace(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    let escape_path = harness
        .test()
        .config
        .cwd
        .parent()
        .expect("cwd should have parent")
        .join("escape.txt");
    harness.remove_abs_path(&escape_path).await?;

    let patch = "*** Begin Patch\n*** Add File: ../escape.txt\n+outside\n*** End Patch";
    let call_id = "apply-path-traversal";
    mount_apply_patch(&harness, call_id, patch, "fail", model_output).await;

    harness
        .submit_with_permission_profile(
            "attempt to escape workspace via apply_patch",
            restrictive_workspace_write_profile(),
        )
        .await?;

    let out = harness.apply_patch_output(call_id, model_output).await;
    assert!(
        out.contains(
            "patch rejected: writing outside of the project; rejected by user approval settings"
        ),
        "expected rejection message for path traversal: {out}"
    );
    assert!(
        !harness.abs_path_exists(&escape_path).await?,
        "path traversal should be rejected; tool output: {out}"
    );
    Ok(())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc ; "shell_command_heredoc")]
async fn intercepted_apply_patch_verification_uses_local_sandbox(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_remote!(Ok(()), "symlink setup needs local filesystem link creation");

    let harness = apply_patch_harness().await?;
    let denied_target = harness.path("denied-target.txt");
    std::fs::write(&denied_target, "outside content\n")?;

    let link_rel = "soft-link.txt";
    create_file_symlink(&denied_target, &harness.path(link_rel))?;

    let patch = format!(
        r#"*** Begin Patch
*** Update File: {link_rel}
@@
-outside content
+pwned
*** End Patch"#
    );
    let call_id = "apply-sandboxed-read";
    mount_apply_patch(&harness, call_id, &patch, "fail", model_output).await;

    harness
        .submit_with_permission_profile(
            "attempt to read denied target via intercepted apply_patch",
            workspace_write_with_unreadable_path(AbsolutePathBuf::try_from(denied_target.clone())?),
        )
        .await?;

    let out = harness.apply_patch_output(call_id, model_output).await;
    assert!(
        out.contains("apply_patch verification failed"),
        "expected sandboxed verification failure: {out}"
    );
    assert!(
        out.contains("Failed to read"),
        "expected read failure: {out}"
    );
    assert_eq!(
        std::fs::read_to_string(&denied_target)?,
        "outside content\n",
        "verification failure should leave the denied target unchanged"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform ; "freeform")]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc ; "shell_command_heredoc")]
async fn apply_patch_cli_does_not_write_through_symlink_escape_outside_workspace(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_remote!(
        Ok(()),
        "link escape setup needs local filesystem link creation"
    );

    let test_root = tempfile::tempdir_in(std::env::current_dir()?)?;
    let work_dir = AbsolutePathBuf::try_from(test_root.path().join("work"))?;
    let outside_dir = AbsolutePathBuf::try_from(test_root.path().join("outside"))?;
    std::fs::create_dir_all(work_dir.as_path())?;
    std::fs::create_dir_all(outside_dir.as_path())?;

    let harness_work_dir = work_dir.clone();
    let harness = apply_patch_harness_with(move |builder| {
        builder.with_config(move |config| {
            config.cwd = harness_work_dir;
        })
    })
    .await?;
    let original_contents = "original outside content\n";
    let outside_file = outside_dir.join("victim.txt");
    std::fs::write(&outside_file, original_contents)?;

    let link_rel = "soft-link.txt";
    let link_path = harness.path(link_rel);
    match create_file_symlink(&outside_file, &link_path) {
        Ok(()) => {}
        Err(error) if cfg!(windows) => {
            eprintln!("Skipping Windows symlink apply_patch sandbox test: {error}");
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    }

    let patch = format!(
        r#"*** Begin Patch
*** Update File: {link_rel}
@@
-original outside content
+pwned
*** End Patch"#
    );
    let call_id = "apply-symlink-escape";
    mount_apply_patch(&harness, call_id, &patch, "fail", model_output).await;

    harness
        .submit_with_permission_profile(
            "attempt to escape workspace via apply_patch link",
            workspace_write_with_read_only_root(outside_dir.clone()),
        )
        .await?;

    let out = harness.apply_patch_output(call_id, model_output).await;
    assert_eq!(
        std::fs::read_to_string(&outside_file)?,
        original_contents,
        "symlink escape should not modify the outside victim; tool output: {out}",
    );
    let metadata = std::fs::symlink_metadata(&link_path)?;
    assert!(metadata.file_type().is_symlink());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform ; "freeform")]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc ; "shell_command_heredoc")]
async fn apply_patch_cli_preserves_existing_hard_link_outside_workspace(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_remote!(
        Ok(()),
        "link setup needs local filesystem hard link creation"
    );

    let test_root = tempfile::tempdir_in(std::env::current_dir()?)?;
    let work_dir = AbsolutePathBuf::try_from(test_root.path().join("work"))?;
    let outside_dir = AbsolutePathBuf::try_from(test_root.path().join("outside"))?;
    std::fs::create_dir_all(work_dir.as_path())?;
    std::fs::create_dir_all(outside_dir.as_path())?;

    let harness_work_dir = work_dir.clone();
    let harness = apply_patch_harness_with(move |builder| {
        builder.with_config(move |config| {
            config.cwd = harness_work_dir;
        })
    })
    .await?;
    let outside_file = outside_dir.join("victim.txt");
    std::fs::write(&outside_file, "original outside content\n")?;

    let link_rel = "hard-link.txt";
    let link_path = harness.path(link_rel);
    std::fs::hard_link(&outside_file, &link_path)?;

    let patch = format!(
        r#"*** Begin Patch
*** Update File: {link_rel}
@@
-original outside content
+updated through existing hard link
*** End Patch"#
    );
    let call_id = "apply-hard-link";
    mount_apply_patch(&harness, call_id, &patch, "ok", model_output).await;

    harness
        .submit_with_permission_profile(
            "update existing hard link via apply_patch",
            workspace_write_with_read_only_root(outside_dir.clone()),
        )
        .await?;

    let out = harness.apply_patch_output(call_id, model_output).await;
    if cfg!(windows) {
        assert!(
            out.contains("patch rejected: writing outside of the project"),
            "Windows sandboxing intentionally rejects writes through existing hard links to files outside the workspace; tool output: {out}"
        );
        assert_eq!(
            std::fs::read_to_string(&outside_file)?,
            "original outside content\n",
            "Windows rejection must leave the outside hard-link target unchanged"
        );
        assert_eq!(
            std::fs::read_to_string(&link_path)?,
            "original outside content\n",
            "Windows rejection must leave the workspace hard-link path unchanged"
        );

        std::fs::write(&outside_file, "post-reject outside write\n")?;
        assert_eq!(
            std::fs::read_to_string(&link_path)?,
            "post-reject outside write\n",
            "Windows rejection must not unlink or replace an existing hard link"
        );

        return Ok(());
    }

    assert!(
        out.contains("Success. Updated the following files:"),
        "apply_patch should intentionally allow updates through existing hard links; tool output: {out}"
    );
    assert_eq!(
        std::fs::read_to_string(&outside_file)?,
        "updated through existing hard link\n",
        "apply_patch intentionally preserves existing hard-link semantics; the outside path observes the shared inode update"
    );
    assert_eq!(
        std::fs::read_to_string(&link_path)?,
        "updated through existing hard link\n",
        "apply_patch intentionally preserves existing hard-link semantics; the workspace path observes the same update"
    );

    std::fs::write(&outside_file, "post-apply outside write\n")?;
    assert_eq!(
        std::fs::read_to_string(&link_path)?,
        "post-apply outside write\n",
        "apply_patch must not unlink or replace an existing hard link; later writes through either path should still be visible"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_rejects_move_path_traversal_outside_workspace(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    let escape_path = harness
        .test()
        .config
        .cwd
        .parent()
        .expect("cwd should have parent")
        .join("escape-move.txt");
    harness.remove_abs_path(&escape_path).await?;

    harness.write_file("stay.txt", "from\n").await?;

    let patch = "*** Begin Patch\n*** Update File: stay.txt\n*** Move to: ../escape-move.txt\n@@\n-from\n+to\n*** End Patch";
    let call_id = "apply-move-traversal";
    mount_apply_patch(&harness, call_id, patch, "fail", model_output).await;

    harness
        .submit_with_permission_profile(
            "attempt move traversal via apply_patch",
            restrictive_workspace_write_profile(),
        )
        .await?;

    let out = harness.apply_patch_output(call_id, model_output).await;
    assert!(
        out.contains(
            "patch rejected: writing outside of the project; rejected by user approval settings"
        ),
        "expected rejection message for path traversal: {out}"
    );
    assert!(
        !harness.abs_path_exists(&escape_path).await?,
        "move path traversal should be rejected; tool output: {out}"
    );
    assert_eq!(harness.read_file_text("stay.txt").await?, "from\n");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_verification_failure_has_no_side_effects(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness_with(|builder| {
        builder.with_config(|config| {
            config
                .features
                .enable(Feature::ApplyPatchFreeform)
                .expect("test config should allow feature update");
        })
    })
    .await?;

    // Compose a patch that would create a file, then fail verification on an update.
    let call_id = "apply-partial-no-side-effects";
    let patch = "*** Begin Patch\n*** Add File: created.txt\n+hello\n*** Update File: missing.txt\n@@\n-old\n+new\n*** End Patch";

    mount_apply_patch(&harness, call_id, patch, "failed", model_output).await;

    harness.submit("attempt partial apply patch").await?;

    assert!(
        !harness.path_exists("created.txt").await?,
        "verification failure should prevent any filesystem changes"
    );
    Ok(())
}
