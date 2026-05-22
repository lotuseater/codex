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
use codex_core_test_runtime::responses::ev_apply_patch_custom_tool_call;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::mount_sse_sequence;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::wait_for_event;
use codex_core_test_runtime::wait_for_event_with_timeout;
use codex_protocol::protocol::EventMsg;
use pretty_assertions::assert_eq;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_aggregates_diff_across_multiple_tool_calls() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;
    let test = harness.test();
    let codex = test.codex.clone();

    let call1 = "agg-1";
    let call2 = "agg-2";
    let patch1 = "*** Begin Patch\n*** Add File: agg/a.txt\n+v1\n*** End Patch";
    let patch2 = "*** Begin Patch\n*** Update File: agg/a.txt\n@@\n-v1\n+v2\n*** Add File: agg/b.txt\n+B\n*** End Patch";

    let s1 = sse(vec![
        ev_response_created("resp-1"),
        ev_apply_patch_custom_tool_call(call1, patch1),
        ev_completed("resp-1"),
    ]);
    let s2 = sse(vec![
        ev_response_created("resp-2"),
        ev_apply_patch_custom_tool_call(call2, patch2),
        ev_completed("resp-2"),
    ]);
    let s3 = sse(vec![
        ev_assistant_message("msg-1", "ok"),
        ev_completed("resp-3"),
    ]);
    mount_sse_sequence(harness.server(), vec![s1, s2, s3]).await;

    submit_without_wait(&harness, "aggregate diffs").await?;

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

    let diff = last_diff.expect("expected TurnDiff after two patches");
    assert!(diff.contains("agg/a.txt"), "diff missing a.txt");
    assert!(diff.contains("agg/b.txt"), "diff missing b.txt");
    // Final content reflects v2 for a.txt
    assert!(diff.contains("+v2\n") || diff.contains("v2\n"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_aggregates_diff_preserves_success_after_failure() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;
    let test = harness.test();
    let codex = test.codex.clone();

    let call_success = "agg-success";
    let call_failure = "agg-failure";
    let patch_success = "*** Begin Patch\n*** Add File: partial/success.txt\n+ok\n*** End Patch";
    let patch_failure =
        "*** Begin Patch\n*** Update File: partial/success.txt\n@@\n-missing\n+new\n*** End Patch";

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_apply_patch_custom_tool_call(call_success, patch_success),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_apply_patch_custom_tool_call(call_failure, patch_failure),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "failed"),
            ev_completed("resp-3"),
        ]),
    ];
    mount_sse_sequence(harness.server(), responses).await;

    submit_without_wait(&harness, "apply patch twice with failure").await?;

    let mut last_diff: Option<String> = None;
    wait_for_event_with_timeout(
        &codex,
        |event| match event {
            EventMsg::TurnDiff(ev) => {
                last_diff = Some(ev.unified_diff.clone());
                false
            }
            EventMsg::TurnComplete(_) => true,
            _ => false,
        },
        Duration::from_secs(30),
    )
    .await;

    let diff = last_diff.expect("expected TurnDiff after failed patch");
    assert!(
        diff.contains("partial/success.txt"),
        "diff should still include the successful addition: {diff}"
    );
    assert!(
        diff.contains("+ok"),
        "diff should include contents from successful patch: {diff}"
    );

    let failure_out = harness.custom_tool_call_output(call_failure).await;
    assert!(
        failure_out.contains("apply_patch verification failed"),
        "expected verification failure output: {failure_out}"
    );
    assert!(
        failure_out.contains("Failed to find expected lines in"),
        "expected missing context diagnostics: {failure_out}"
    );

    assert_eq!(harness.read_file_text("partial/success.txt").await?, "ok\n");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_clears_aggregated_diff_after_inexact_delta() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness_with(|builder| {
        builder.with_workspace_setup(|cwd, fs| async move {
            fs.write_file(
                &cwd.join("binary.dat"),
                vec![0xff, 0xfe, 0xfd],
                /*sandbox*/ None,
            )
            .await?;
            Ok(())
        })
    })
    .await?;
    let test = harness.test();
    let codex = test.codex.clone();

    let call_success = "agg-success";
    let call_inexact = "agg-inexact";
    let patch_success = "*** Begin Patch\n*** Add File: partial/success.txt\n+ok\n*** End Patch";
    let patch_inexact = "*** Begin Patch\n*** Add File: binary.dat\n+text\n*** End Patch";

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_apply_patch_custom_tool_call(call_success, patch_success),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_apply_patch_custom_tool_call(call_inexact, patch_inexact),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-3"),
        ]),
    ];
    mount_sse_sequence(harness.server(), responses).await;

    submit_without_wait(&harness, "apply patch twice with inexact delta").await?;

    let mut last_diff: Option<String> = None;
    wait_for_event_with_timeout(
        &codex,
        |event| match event {
            EventMsg::TurnDiff(ev) => {
                last_diff = Some(ev.unified_diff.clone());
                false
            }
            EventMsg::TurnComplete(_) => true,
            _ => false,
        },
        Duration::from_secs(30),
    )
    .await;

    assert_eq!(
        last_diff.as_deref(),
        Some(""),
        "inexact delta should clear the aggregate diff"
    );
    assert_eq!(harness.read_file_text("partial/success.txt").await?, "ok\n");
    assert_eq!(harness.read_file_text("binary.dat").await?, "text\n");
    Ok(())
}
