#![allow(clippy::expect_used)]

mod support;

#[path = "suite/apply_patch_cli.rs"]
mod apply_patch_cli;
#[path = "suite/apply_patch_harness.rs"]
mod apply_patch_harness;

use anyhow::Result;
use apply_patch_cli::submit_without_wait;
use apply_patch_harness::apply_patch_harness;
use apply_patch_harness::apply_patch_harness_with;
use apply_patch_harness::mount_apply_patch;
use codex_core_test_runtime::assert_regex_match;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::test_codex::ApplyPatchModelOutput;
use codex_core_test_runtime::wait_for_event;
use codex_protocol::protocol::EventMsg;
use pretty_assertions::assert_eq;
use test_case::test_case;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
async fn apply_patch_cli_multiple_operations_integration(
    output_type: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness_with(|builder| builder.with_model("gpt-5.4")).await?;

    // Seed workspace state
    harness.write_file("modify.txt", "line1\nline2\n").await?;
    harness.write_file("delete.txt", "obsolete\n").await?;

    let patch = "*** Begin Patch\n*** Add File: nested/new.txt\n+created\n*** Delete File: delete.txt\n*** Update File: modify.txt\n@@\n-line2\n+changed\n*** End Patch";

    let call_id = "apply-multi-ops";
    mount_apply_patch(&harness, call_id, patch, "done", output_type).await;

    harness.submit("please apply multi-ops patch").await?;

    let out = harness.apply_patch_output(call_id, output_type).await;

    let expected = r"(?s)^Exit code: 0
Wall time: [0-9]+(?:\.[0-9]+)? seconds
Output:
Success. Updated the following files:
A nested/new.txt
M modify.txt
D delete.txt
?$";
    assert_regex_match(expected, &out);

    assert_eq!(harness.read_file_text("nested/new.txt").await?, "created\n");
    assert_eq!(
        harness.read_file_text("modify.txt").await?,
        "line1\nchanged\n"
    );
    assert!(!harness.path_exists("delete.txt").await?);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_multiple_chunks(model_output: ApplyPatchModelOutput) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    harness
        .write_file("multi.txt", "line1\nline2\nline3\nline4\n")
        .await?;

    let patch = "*** Begin Patch\n*** Update File: multi.txt\n@@\n-line2\n+changed2\n@@\n-line4\n+changed4\n*** End Patch";
    let call_id = "apply-multi-chunks";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("apply multi-chunk patch").await?;

    assert_eq!(
        harness.read_file_text("multi.txt").await?,
        "line1\nchanged2\nline3\nchanged4\n"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_moves_file_to_new_directory(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    harness.write_file("old/name.txt", "old content\n").await?;

    let patch = "*** Begin Patch\n*** Update File: old/name.txt\n*** Move to: renamed/dir/name.txt\n@@\n-old content\n+new content\n*** End Patch";
    let call_id = "apply-move";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("apply move patch").await?;

    assert!(!harness.path_exists("old/name.txt").await?);
    assert_eq!(
        harness.read_file_text("renamed/dir/name.txt").await?,
        "new content\n"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_updates_file_appends_trailing_newline(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    harness
        .write_file("no_newline.txt", "no newline at end")
        .await?;

    let patch = "*** Begin Patch\n*** Update File: no_newline.txt\n@@\n-no newline at end\n+first line\n+second line\n*** End Patch";
    let call_id = "apply-append-nl";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("apply newline patch").await?;

    let contents = harness.read_file_text("no_newline.txt").await?;
    assert!(contents.ends_with('\n'));
    assert_eq!(contents, "first line\nsecond line\n");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_insert_only_hunk_modifies_file(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    harness
        .write_file("insert_only.txt", "alpha\nomega\n")
        .await?;

    let patch = "*** Begin Patch\n*** Update File: insert_only.txt\n@@\n alpha\n+beta\n omega\n*** End Patch";
    let call_id = "apply-insert-only";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("insert lines via apply_patch").await?;

    assert_eq!(
        harness.read_file_text("insert_only.txt").await?,
        "alpha\nbeta\nomega\n"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_move_overwrites_existing_destination(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    harness.write_file("old/name.txt", "from\n").await?;
    harness
        .write_file("renamed/dir/name.txt", "existing\n")
        .await?;

    let patch = "*** Begin Patch\n*** Update File: old/name.txt\n*** Move to: renamed/dir/name.txt\n@@\n-from\n+new\n*** End Patch";
    let call_id = "apply-move-overwrite";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("apply move overwrite patch").await?;

    assert!(!harness.path_exists("old/name.txt").await?);
    assert_eq!(
        harness.read_file_text("renamed/dir/name.txt").await?,
        "new\n"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_move_without_content_change_has_no_turn_diff(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;
    let test = harness.test();
    let codex = test.codex.clone();

    harness.write_file("old/name.txt", "same\n").await?;

    let patch = "*** Begin Patch\n*** Update File: old/name.txt\n*** Move to: renamed/name.txt\n@@\n same\n*** End Patch";
    let call_id = "apply-move-no-change";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    submit_without_wait(&harness, "rename without content change").await?;

    let mut saw_turn_diff = false;
    wait_for_event(&codex, |event| match event {
        EventMsg::TurnDiff(_) => {
            saw_turn_diff = true;
            false
        }
        EventMsg::TurnComplete(_) => true,
        _ => false,
    })
    .await;

    assert!(!saw_turn_diff, "pure rename should not emit a turn diff");
    assert!(!harness.path_exists("old/name.txt").await?);
    assert_eq!(harness.read_file_text("renamed/name.txt").await?, "same\n");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_add_overwrites_existing_file(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    harness.write_file("duplicate.txt", "old content\n").await?;

    let patch = "*** Begin Patch\n*** Add File: duplicate.txt\n+new content\n*** End Patch";
    let call_id = "apply-add-overwrite";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("apply add overwrite patch").await?;

    assert_eq!(
        harness.read_file_text("duplicate.txt").await?,
        "new content\n"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_end_of_file_anchor(model_output: ApplyPatchModelOutput) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    harness.write_file("tail.txt", "alpha\nlast\n").await?;

    let patch = "*** Begin Patch\n*** Update File: tail.txt\n@@\n-last\n+end\n*** End of File\n*** End Patch";
    let call_id = "apply-eof";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("apply EOF-anchored patch").await?;
    assert_eq!(harness.read_file_text("tail.txt").await?, "alpha\nend\n");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_change_context_disambiguates_target(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    harness
        .write_file("multi_ctx.txt", "fn a\nx=10\ny=2\nfn b\nx=10\ny=20\n")
        .await?;

    let patch =
        "*** Begin Patch\n*** Update File: multi_ctx.txt\n@@ fn b\n-x=10\n+x=11\n*** End Patch";
    let call_id = "apply-ctx";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("apply with change_context").await?;

    let contents = harness.read_file_text("multi_ctx.txt").await?;
    assert_eq!(contents, "fn a\nx=10\ny=2\nfn b\nx=11\ny=20\n");
    Ok(())
}
