use crate::unified_exec_support::UNIFIED_EXEC_LAGGED_OUTPUT_TIMEOUT;
use crate::unified_exec_support::collect_tool_outputs;
use crate::unified_exec_support::parse_unified_exec_output;
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
use codex_core_test_runtime::wait_for_event_with_timeout;
use codex_features::Feature;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SandboxPolicy;
use pretty_assertions::assert_eq;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_reuses_session_via_stdin() -> Result<()> {
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

    let first_call_id = "uexec-start";
    let first_args = serde_json::json!({
        "cmd": "/bin/cat",
        "yield_time_ms": 200,
        "tty": true,
    });

    let second_call_id = "uexec-stdin";
    let second_args = serde_json::json!({
        "chars": "hello unified exec\n",
        "session_id": 1000,
        "yield_time_ms": 500,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(
                first_call_id,
                "exec_command",
                &serde_json::to_string(&first_args)?,
            ),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_function_call(
                second_call_id,
                "write_stdin",
                &serde_json::to_string(&second_args)?,
            ),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "all done"),
            ev_completed("resp-3"),
        ]),
    ];
    let request_log = mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(&test, "run unified exec", SandboxPolicy::DangerFullAccess).await?;

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

    let start_output = outputs
        .get(first_call_id)
        .expect("missing first unified_exec output");
    let process_id = start_output.process_id.clone().unwrap_or_default();
    assert!(
        !process_id.is_empty(),
        "expected process id in first unified_exec response"
    );
    assert!(start_output.output.is_empty());

    let reuse_output = outputs
        .get(second_call_id)
        .expect("missing reused unified_exec output");
    assert_eq!(
        reuse_output.process_id.clone().unwrap_or_default(),
        process_id
    );
    let echoed = reuse_output.output.as_str();
    assert!(
        echoed.contains("hello unified exec"),
        "expected echoed output, got {echoed:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_streams_after_lagged_output() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_windows!(Ok(()));

    let server = start_mock_server().await;

    let mut builder = test_codex().with_config(|config| {
        config.use_experimental_unified_exec_tool = true;
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
    });
    let test = builder.build_remote_aware(&server).await?;

    let script = r#"python3 - <<'PY'
import sys
import time

chunk = b'long content here to trigger truncation' * (1 << 10)
for _ in range(4):
    sys.stdout.buffer.write(chunk)
    sys.stdout.flush()

time.sleep(0.2)
for _ in range(5):
    sys.stdout.write("TAIL-MARKER\n")
    sys.stdout.flush()
    time.sleep(0.05)

time.sleep(0.2)
PY
"#;

    let first_call_id = "uexec-lag-start";
    let first_args = serde_json::json!({
        "cmd": script,
        "yield_time_ms": 25,
        "tty": true,
    });

    let second_call_id = "uexec-lag-poll";
    let second_args = serde_json::json!({
        "chars": "",
        "session_id": 1000,
        "yield_time_ms": 2_000,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(
                first_call_id,
                "exec_command",
                &serde_json::to_string(&first_args)?,
            ),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_function_call(
                second_call_id,
                "write_stdin",
                &serde_json::to_string(&second_args)?,
            ),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "lag handled"),
            ev_completed("resp-3"),
        ]),
    ];
    let request_log = mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(
        &test,
        "exercise lag handling",
        SandboxPolicy::DangerFullAccess,
    )
    .await?;
    // This is a worst case scenario for the truncate logic, and CI can spend a
    // while draining the lagged tail before the follow-up tool call completes.
    wait_for_event_with_timeout(
        &test.codex,
        |event| matches!(event, EventMsg::TurnComplete(_)),
        UNIFIED_EXEC_LAGGED_OUTPUT_TIMEOUT,
    )
    .await;

    let requests = request_log.requests();
    assert!(!requests.is_empty(), "expected at least one POST request");
    let bodies = requests
        .into_iter()
        .map(|request| request.body_json())
        .collect::<Vec<_>>();

    let outputs = collect_tool_outputs(&bodies)?;

    let start_output = outputs
        .get(first_call_id)
        .expect("missing initial unified_exec output");
    let process_id = start_output.process_id.clone().unwrap_or_default();
    assert!(
        !process_id.is_empty(),
        "expected session id from initial unified_exec response"
    );

    let poll_output = outputs
        .get(second_call_id)
        .expect("missing poll unified_exec output");
    let poll_text = poll_output.output.as_str();
    assert!(
        poll_text.contains("TAIL-MARKER"),
        "expected poll output to contain tail marker, got {poll_text:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_timeout_and_followup_poll() -> Result<()> {
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

    let first_call_id = "uexec-timeout";
    let first_args = serde_json::json!({
        "cmd": "sleep 0.5; echo ready",
        "yield_time_ms": 10,
    });

    let second_call_id = "uexec-poll";
    let second_args = serde_json::json!({
        "chars": "",
        "session_id": 1000,
        "yield_time_ms": 800,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(
                first_call_id,
                "exec_command",
                &serde_json::to_string(&first_args)?,
            ),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_function_call(
                second_call_id,
                "write_stdin",
                &serde_json::to_string(&second_args)?,
            ),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-3"),
        ]),
    ];
    let request_log = mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(&test, "check timeout", SandboxPolicy::DangerFullAccess).await?;

    loop {
        let event = test.codex.next_event().await.expect("event");
        if matches!(event.msg, EventMsg::TurnComplete(_)) {
            break;
        }
    }

    let requests = request_log.requests();
    assert!(!requests.is_empty(), "expected at least one POST request");
    let bodies = requests
        .into_iter()
        .map(|request| request.body_json())
        .collect::<Vec<_>>();

    let outputs = collect_tool_outputs(&bodies)?;

    let first_output = outputs.get(first_call_id).expect("missing timeout output");
    assert!(first_output.process_id.is_some());
    assert!(first_output.output.is_empty());

    let poll_output = outputs.get(second_call_id).expect("missing poll output");
    let output_text = poll_output.output.as_str();
    assert!(
        output_text.contains("ready"),
        "expected ready output, got {output_text:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn unified_exec_prunes_exited_sessions_first() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_windows!(Ok(()));

    let server = start_mock_server().await;

    let mut builder = test_codex().with_config(|config| {
        config.use_experimental_unified_exec_tool = true;
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
    });
    let test = builder.build_remote_aware(&server).await?;

    const MAX_SESSIONS_FOR_TEST: i32 = 64;
    const FILLER_SESSIONS: i32 = MAX_SESSIONS_FOR_TEST - 1;

    let keep_call_id = "uexec-prune-keep";
    let keep_args = serde_json::json!({
        "cmd": "/bin/cat",
        "yield_time_ms": 250,
        "tty": true,
    });

    let prune_call_id = "uexec-prune-target";
    // Give the sleeper time to exit before the filler sessions trigger pruning.
    let prune_args = serde_json::json!({
        "cmd": "sleep 1",
        "yield_time_ms": 1_250,
        "tty": true,
    });

    let mut events = vec![ev_response_created("resp-prune-1")];
    events.push(ev_function_call(
        keep_call_id,
        "exec_command",
        &serde_json::to_string(&keep_args)?,
    ));
    events.push(ev_function_call(
        prune_call_id,
        "exec_command",
        &serde_json::to_string(&prune_args)?,
    ));

    for idx in 0..FILLER_SESSIONS {
        let filler_args = serde_json::json!({
            "cmd": format!("echo filler {idx}"),
            "yield_time_ms": 250,
        });
        let call_id = format!("uexec-prune-fill-{idx}");
        events.push(ev_function_call(
            &call_id,
            "exec_command",
            &serde_json::to_string(&filler_args)?,
        ));
    }

    let keep_write_call_id = "uexec-prune-keep-write";
    let keep_write_args = serde_json::json!({
        "chars": "still alive\n",
        "session_id": 1000,
        "yield_time_ms": 500,
    });
    events.push(ev_function_call(
        keep_write_call_id,
        "write_stdin",
        &serde_json::to_string(&keep_write_args)?,
    ));

    let probe_call_id = "uexec-prune-probe";
    let probe_args = serde_json::json!({
        "chars": "should fail\n",
        "session_id": 1001,
        "yield_time_ms": 500,
    });
    events.push(ev_function_call(
        probe_call_id,
        "write_stdin",
        &serde_json::to_string(&probe_args)?,
    ));

    events.push(ev_completed("resp-prune-1"));
    let first_response = sse(events);
    let completion_response = sse(vec![
        ev_response_created("resp-prune-2"),
        ev_assistant_message("msg-prune", "done"),
        ev_completed("resp-prune-2"),
    ]);
    let response_mock =
        mount_sse_sequence(&server, vec![first_response, completion_response]).await;

    submit_unified_exec_turn(&test, "fill session cache", SandboxPolicy::DangerFullAccess).await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = response_mock.requests();
    assert!(
        !requests.is_empty(),
        "expected at least one response request"
    );

    let keep_start = requests
        .iter()
        .find_map(|req| req.function_call_output_text(keep_call_id))
        .expect("missing initial keep session output");
    let keep_start_output = parse_unified_exec_output(&keep_start)?;
    assert!(keep_start_output.process_id.is_some());
    assert!(keep_start_output.exit_code.is_none());

    let prune_start = requests
        .iter()
        .find_map(|req| req.function_call_output_text(prune_call_id))
        .expect("missing initial prune process output");
    let prune_start_output = parse_unified_exec_output(&prune_start)?;
    assert!(prune_start_output.process_id.is_some());
    assert!(prune_start_output.exit_code.is_none());

    let keep_write = requests
        .iter()
        .find_map(|req| req.function_call_output_text(keep_write_call_id))
        .expect("missing keep write output");
    let keep_write_output = parse_unified_exec_output(&keep_write)?;
    assert!(keep_write_output.process_id.is_some());
    assert!(
        keep_write_output.output.contains("still alive"),
        "expected cat process to echo input, got {:?}",
        keep_write_output.output
    );

    let pruned_probe = requests
        .iter()
        .find_map(|req| req.function_call_output_text(probe_call_id))
        .expect("missing probe output");
    assert!(
        pruned_probe.contains("UnknownProcessId") || pruned_probe.contains("Unknown process id"),
        "expected probe to fail after pruning, got {pruned_probe:?}"
    );

    Ok(())
}
