use crate::unified_exec_support::collect_tool_outputs;
use crate::unified_exec_support::submit_unified_exec_turn;
use anyhow::Result;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_function_call;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::mount_sse_sequence;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::responses::start_mock_server;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::skip_if_sandbox;
use codex_core_test_runtime::skip_if_windows;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::wait_for_event;
use codex_features::Feature;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SandboxPolicy;
use pretty_assertions::assert_eq;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_defaults_to_pipe() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_windows!(Ok(()));

    let server = start_mock_server().await;

    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
    });
    let test = builder.build_remote_aware(&server).await?;

    let call_id = "uexec-default-pipe";
    let args = serde_json::json!({
        "cmd": "python3 -c \"import sys; print(sys.stdin.isatty())\"",
        "yield_time_ms": 1500,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    ];
    let request_log = mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(
        &test,
        "check default pipe mode",
        SandboxPolicy::DangerFullAccess,
    )
    .await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = request_log.requests();
    assert!(!requests.is_empty(), "expected at least one POST request");
    let bodies = requests
        .into_iter()
        .map(|request| request.body_json())
        .collect::<Vec<_>>();

    let outputs = collect_tool_outputs(&bodies)?;
    let output = outputs
        .get(call_id)
        .expect("missing default pipe unified exec output");
    let normalized = output.output.replace("\r\n", "\n");

    assert!(
        normalized.contains("False"),
        "stdin should not be a tty by default: {normalized:?}"
    );
    assert_eq!(output.exit_code, Some(0));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_can_enable_tty() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_windows!(Ok(()));

    let server = start_mock_server().await;

    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
    });
    let test = builder.build_remote_aware(&server).await?;

    let call_id = "uexec-tty-enabled";
    let args = serde_json::json!({
        "cmd": "python3 -c \"import sys; print(sys.stdin.isatty())\"",
        "yield_time_ms": 1500,
        "tty": true,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    ];
    let request_log = mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(&test, "check tty enabled", SandboxPolicy::DangerFullAccess).await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = request_log.requests();
    assert!(!requests.is_empty(), "expected at least one POST request");
    let bodies = requests
        .into_iter()
        .map(|request| request.body_json())
        .collect::<Vec<_>>();

    let outputs = collect_tool_outputs(&bodies)?;
    let output = outputs
        .get(call_id)
        .expect("missing tty-enabled unified exec output");
    let normalized = output.output.replace("\r\n", "\n");

    assert!(
        normalized.contains("True"),
        "stdin should be a tty when tty=true: {normalized:?}"
    );
    assert_eq!(output.exit_code, Some(0));
    assert!(output.process_id.is_none(), "process should have exited");
    Ok(())
}
