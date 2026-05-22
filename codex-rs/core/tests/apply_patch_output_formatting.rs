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
use codex_core_test_runtime::responses::ev_apply_patch_custom_tool_call;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_function_call;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::mount_sse_sequence;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::test_codex::ApplyPatchModelOutput;
use codex_core_test_runtime::wait_for_event;
use codex_exec_server::CreateDirectoryOptions;
use codex_features::Feature;
use codex_protocol::protocol::EventMsg;
use pretty_assertions::assert_eq;
use serde_json::json;
use test_case::test_case;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_custom_tool_streaming_emits_updated_changes() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness_with(|builder| {
        builder.with_config(|config| {
            config
                .features
                .enable(Feature::ApplyPatchStreamingEvents)
                .expect("enable apply_patch streaming events");
        })
    })
    .await?;
    let test = harness.test();
    let codex = test.codex.clone();
    let call_id = "apply-patch-streaming";
    let patch = "*** Begin Patch\n*** Add File: streamed.txt\n+hello\n+world\n*** End Patch";
    mount_sse_sequence(
        harness.server(),
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                json!({
                    "type": "response.output_item.added",
                    "item": {
                        "type": "custom_tool_call",
                        "call_id": call_id,
                        "name": "apply_patch",
                        "input": "",
                    }
                }),
                json!({
                    "type": "response.custom_tool_call_input.delta",
                    "call_id": call_id,
                    "delta": "*** Begin Patch\n",
                }),
                json!({
                    "type": "response.custom_tool_call_input.delta",
                    "call_id": call_id,
                    "delta": "*** Add File: streamed.txt\n+hello",
                }),
                json!({
                    "type": "response.custom_tool_call_input.delta",
                    "call_id": call_id,
                    "delta": "\n+world\n*** End Patch",
                }),
                ev_apply_patch_custom_tool_call(call_id, patch),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    submit_without_wait(&harness, "create streamed file").await?;

    let mut updates = Vec::new();
    wait_for_event(&codex, |event| match event {
        EventMsg::PatchApplyUpdated(update) => {
            updates.push(update.clone());
            false
        }
        EventMsg::TurnComplete(_) => true,
        _ => false,
    })
    .await;

    assert_eq!(
        updates
            .iter()
            .map(|update| update.call_id.as_str())
            .collect::<Vec<_>>(),
        vec![call_id, call_id]
    );
    assert_eq!(
        updates
            .first()
            .expect("first update")
            .changes
            .get(&std::path::PathBuf::from("streamed.txt")),
        Some(&codex_protocol::protocol::FileChange::Add {
            content: String::new(),
        })
    );
    assert_eq!(
        updates
            .last()
            .expect("last update")
            .changes
            .get(&std::path::PathBuf::from("streamed.txt")),
        Some(&codex_protocol::protocol::FileChange::Add {
            content: "hello\nworld\n".to_string(),
        })
    );
    assert_eq!(
        harness.read_file_text("streamed.txt").await?,
        "hello\nworld\n"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_shell_command_heredoc_with_cd_emits_turn_diff() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness_with(|builder| builder.with_model("gpt-5.4")).await?;
    let test = harness.test();
    let codex = test.codex.clone();

    // Prepare a file inside a subdir; update it via cd && apply_patch heredoc form.
    harness.write_file("sub/in_sub.txt", "before\n").await?;

    let script = "cd sub && apply_patch <<'EOF'\n*** Begin Patch\n*** Update File: in_sub.txt\n@@\n-before\n+after\n*** End Patch\nEOF\n";
    let call_id = "shell-heredoc-cd";
    let args = json!({ "command": script, "timeout_ms": 30_000 });
    let bodies = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "shell_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "ok"),
            ev_completed("resp-2"),
        ]),
    ];
    mount_sse_sequence(harness.server(), bodies).await;

    submit_without_wait(&harness, "apply via shell heredoc with cd").await?;

    let mut saw_turn_diff = None;
    let mut saw_patch_begin = false;
    let mut patch_end_success = None;
    wait_for_event(&codex, |event| match event {
        EventMsg::PatchApplyBegin(begin) => {
            saw_patch_begin = true;
            assert_eq!(begin.call_id, call_id);
            false
        }
        EventMsg::PatchApplyEnd(end) => {
            assert_eq!(end.call_id, call_id);
            patch_end_success = Some(end.success);
            false
        }
        EventMsg::TurnDiff(ev) => {
            saw_turn_diff = Some(ev.unified_diff.clone());
            false
        }
        EventMsg::TurnComplete(_) => true,
        _ => false,
    })
    .await;

    assert!(saw_patch_begin, "expected PatchApplyBegin event");
    let patch_end_success =
        patch_end_success.expect("expected PatchApplyEnd event to capture success flag");
    assert!(patch_end_success);

    let diff = saw_turn_diff.expect("expected TurnDiff event");
    assert!(diff.contains("diff --git"), "diff header missing: {diff:?}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_turn_diff_paths_stay_repo_relative_when_session_cwd_is_nested() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness_with(|builder| {
        builder
            .with_model("gpt-5.4")
            .with_config(|config| {
                config.cwd = config.cwd.join("subdir");
            })
            .with_workspace_setup(|cwd, fs| async move {
                fs.create_directory(
                    &cwd,
                    CreateDirectoryOptions { recursive: true },
                    /*sandbox*/ None,
                )
                .await?;
                let repo_root = cwd.parent().expect("nested cwd should have parent");
                fs.write_file(
                    &repo_root.join(".git"),
                    b"gitdir: /tmp/fake-worktree\n".to_vec(),
                    /*sandbox*/ None,
                )
                .await?;
                fs.write_file(
                    &repo_root.join("repo.txt"),
                    b"before\n".to_vec(),
                    /*sandbox*/ None,
                )
                .await?;
                Ok(())
            })
    })
    .await?;
    let test = harness.test();
    let codex = test.codex.clone();
    let repo_root = harness
        .test()
        .config
        .cwd
        .parent()
        .expect("nested cwd should have parent");

    let call_id = "apply-nested-cwd-repo-relative";
    let patch = "*** Begin Patch\n*** Update File: ../repo.txt\n@@\n-before\n+after\n*** End Patch";
    mount_apply_patch(
        &harness,
        call_id,
        patch,
        "updated repo-relative path",
        ApplyPatchModelOutput::Freeform,
    )
    .await;

    submit_without_wait(&harness, "update file outside nested cwd but inside repo").await?;

    let mut last_diff: Option<String> = None;
    wait_for_event(&codex, |event| match event {
        EventMsg::TurnDiff(ev) => {
            last_diff = Some(ev.unified_diff.clone());
            false
        }
        EventMsg::TurnComplete(_) => true,
        _ => false,
    })
    .await;

    let diff = last_diff.expect("expected TurnDiff event after update");
    assert!(
        diff.contains("diff --git a/repo.txt b/repo.txt"),
        "diff should stay repo-relative: {diff:?}"
    );
    assert!(
        !diff.contains(repo_root.as_path().to_string_lossy().as_ref()),
        "diff should not leak absolute repo paths: {diff:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_shell_command_failure_propagates_error_and_skips_diff() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness_with(|builder| builder.with_model("gpt-5.4")).await?;
    let test = harness.test();
    let codex = test.codex.clone();

    harness.write_file("invalid.txt", "ok\n").await?;

    let script = "apply_patch <<'EOF'\n*** Begin Patch\n*** Update File: invalid.txt\n@@\n-nope\n+changed\n*** End Patch\nEOF\n";
    let call_id = "shell-apply-failure";
    let args = json!({ "command": script, "timeout_ms": 5_000 });
    let bodies = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "shell_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "fail"),
            ev_completed("resp-2"),
        ]),
    ];
    mount_sse_sequence(harness.server(), bodies).await;

    submit_without_wait(&harness, "apply patch via shell").await?;

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

    assert!(
        !saw_turn_diff,
        "turn diff should not be emitted when shell apply_patch fails verification"
    );

    let out = harness.function_call_stdout(call_id).await;
    assert!(
        out.contains("Failed to find expected lines in"),
        "expected failure diagnostics: {out}"
    );
    assert!(
        out.contains("invalid.txt"),
        "expected file path in output: {out}"
    );
    assert_eq!(harness.read_file_text("invalid.txt").await?, "ok\n");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_emits_turn_diff_event_with_unified_diff(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;
    let test = harness.test();
    let codex = test.codex.clone();

    let call_id = "apply-diff-event";
    let file = "udiff.txt";
    let patch = format!("*** Begin Patch\n*** Add File: {file}\n+hello\n*** End Patch\n");
    mount_apply_patch(&harness, call_id, patch.as_str(), "ok", model_output).await;

    submit_without_wait(&harness, "emit diff").await?;

    let mut saw_turn_diff = None;
    wait_for_event(&codex, |event| match event {
        EventMsg::TurnDiff(ev) => {
            saw_turn_diff = Some(ev.unified_diff.clone());
            false
        }
        EventMsg::TurnComplete(_) => true,
        _ => false,
    })
    .await;

    let diff = saw_turn_diff.expect("expected TurnDiff event");
    // Basic markers of a unified diff with file addition
    assert!(diff.contains("diff --git"), "diff header missing: {diff:?}");
    assert!(diff.contains("--- /dev/null") || diff.contains("--- a/"));
    assert!(diff.contains("+++ b/"));
    Ok(())
}
