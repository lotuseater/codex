use crate::unified_exec_support::submit_unified_exec_turn;
use crate::unified_exec_support::wait_for_raw_unified_exec_output;
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
use serde_json::Value;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_emits_terminal_interaction_for_write_stdin() -> Result<()> {
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

    let open_call_id = "uexec-open";
    let open_args = json!({
        "cmd": "/bin/bash -i",
        "yield_time_ms": 200,
        "tty": true,
    });

    let stdin_call_id = "uexec-stdin-delta";
    let stdin_args = json!({
        "chars": "echo WSTDIN-MARK\\n",
        "session_id": 1000,
        "yield_time_ms": 800,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(
                open_call_id,
                "exec_command",
                &serde_json::to_string(&open_args)?,
            ),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_function_call(
                stdin_call_id,
                "write_stdin",
                &serde_json::to_string(&stdin_args)?,
            ),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_response_created("resp-3"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-3"),
        ]),
    ];
    mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(&test, "stdin delta", SandboxPolicy::DangerFullAccess).await?;

    let mut terminal_interaction = None;

    loop {
        let msg = wait_for_event(&test.codex, |_| true).await;
        match msg {
            EventMsg::TerminalInteraction(ev) if ev.call_id == open_call_id => {
                terminal_interaction = Some(ev);
            }
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    let delta = terminal_interaction.expect("expected TerminalInteraction event");
    assert_eq!(delta.process_id, "1000");
    let expected_stdin = stdin_args
        .get("chars")
        .and_then(Value::as_str)
        .expect("stdin chars");
    assert_eq!(delta.stdin, expected_stdin);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_terminal_interaction_captures_delayed_output() -> Result<()> {
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

    let open_call_id = "uexec-delayed-open";
    let open_args = json!({
        "cmd": "sleep 3 && echo MARKER1 && sleep 3 && echo MARKER2",
        "yield_time_ms": 10,
        "tty": true,
    });

    // Poll stdin three times: first for no output, second after the first marker,
    // and a final long poll to capture the second marker.
    let first_poll_call_id = "uexec-delayed-poll-1";
    let first_poll_args = json!({
        "chars": "x",
        "session_id": 1000,
        "yield_time_ms": 10,
    });

    let second_poll_call_id = "uexec-delayed-poll-2";
    let second_poll_args = json!({
        "chars": "x",
        "session_id": 1000,
        "yield_time_ms": 4000,
    });

    let third_poll_call_id = "uexec-delayed-poll-3";
    let third_poll_args = json!({
        "chars": "x",
        "session_id": 1000,
        "yield_time_ms": 6000,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(
                open_call_id,
                "exec_command",
                &serde_json::to_string(&open_args)?,
            ),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_function_call(
                first_poll_call_id,
                "write_stdin",
                &serde_json::to_string(&first_poll_args)?,
            ),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_response_created("resp-3"),
            ev_function_call(
                second_poll_call_id,
                "write_stdin",
                &serde_json::to_string(&second_poll_args)?,
            ),
            ev_completed("resp-3"),
        ]),
        sse(vec![
            ev_response_created("resp-4"),
            ev_function_call(
                third_poll_call_id,
                "write_stdin",
                &serde_json::to_string(&third_poll_args)?,
            ),
            ev_completed("resp-4"),
        ]),
        sse(vec![
            ev_response_created("resp-5"),
            ev_assistant_message("msg-1", "complete"),
            ev_completed("resp-5"),
        ]),
    ];
    mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(
        &test,
        "delayed terminal interaction output",
        SandboxPolicy::DangerFullAccess,
    )
    .await?;

    let mut begin_event = None;
    let mut end_event = None;
    let mut task_completed = false;
    let mut terminal_events = Vec::new();
    let mut delta_text = String::new();

    // Consume all events for this turn so we can assert on each stage.
    loop {
        let msg = wait_for_event(&test.codex, |_| true).await;
        match msg {
            EventMsg::ExecCommandBegin(ev) if ev.call_id == open_call_id => {
                begin_event = Some(ev);
            }
            EventMsg::ExecCommandOutputDelta(ev) if ev.call_id == open_call_id => {
                delta_text.push_str(&String::from_utf8_lossy(&ev.chunk));
            }
            EventMsg::TerminalInteraction(ev) if ev.call_id == open_call_id => {
                terminal_events.push(ev);
            }
            EventMsg::ExecCommandEnd(ev) if ev.call_id == open_call_id => {
                end_event = Some(ev);
            }
            EventMsg::TurnComplete(_) => {
                task_completed = true;
            }
            _ => {}
        };
        if task_completed && end_event.is_some() {
            break;
        }
    }

    let begin_event = begin_event.expect("expected ExecCommandBegin event");
    assert!(
        begin_event.process_id.is_some(),
        "begin event should include process_id for a live session"
    );

    // We expect three terminal interactions matching the three write_stdin calls.
    assert_eq!(
        terminal_events.len(),
        3,
        "expected three terminal interactions; got {terminal_events:?}"
    );

    for event in &terminal_events {
        assert_eq!(event.call_id, open_call_id);
        assert_eq!(event.process_id, "1000");
    }
    assert_eq!(
        terminal_events
            .iter()
            .map(|ev| ev.stdin.as_str())
            .collect::<Vec<_>>(),
        vec!["x", "x", "x"],
        "terminal interactions should reflect the three stdin polls"
    );

    assert!(
        delta_text.contains("MARKER1") && delta_text.contains("MARKER2"),
        "streamed deltas should contain both markers; got {delta_text:?}"
    );

    let end_event = end_event.expect("expected ExecCommandEnd event");
    assert_eq!(end_event.call_id, open_call_id);
    assert_eq!(end_event.exit_code, 0);
    assert!(
        end_event.process_id.is_some(),
        "end event should include the process_id"
    );
    assert!(
        end_event.aggregated_output.contains("MARKER1")
            && end_event.aggregated_output.contains("MARKER2"),
        "aggregated output should include both markers in order; got {:?}",
        end_event.aggregated_output
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_stdin_clamps_model_requested_max_output_tokens_to_policy() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_windows!(Ok(()));

    let server = start_mock_server().await;

    let mut builder = test_codex().with_model("gpt-5.4").with_config(|config| {
        config.use_experimental_unified_exec_tool = true;
        config.tool_output_token_limit = Some(50);
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
    });
    let test = builder.build_remote_aware(&server).await?;

    let start_call_id = "uexec-stdin-clamp-start";
    let start_args = serde_json::json!({
        "cmd": "printf 'READY\\n'; read trigger; line_number=1; while [ \"$line_number\" -le 999 ]; do printf 'STDIN-LINE-%04d yyyyyyyyyyyyyyyyyyyy\\n' \"$line_number\"; line_number=$((line_number + 1)); done",
        "yield_time_ms": 500,
        "tty": true,
    });

    let stdin_call_id = "uexec-stdin-clamped-max-output";
    let stdin_args = serde_json::json!({
        "chars": "go\n",
        "session_id": 1000,
        "yield_time_ms": 3_000,
        "max_output_tokens": 70_000,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(
                start_call_id,
                "exec_command",
                &serde_json::to_string(&start_args)?,
            ),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_function_call(
                stdin_call_id,
                "write_stdin",
                &serde_json::to_string(&stdin_args)?,
            ),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_response_created("resp-3"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-3"),
        ]),
    ];
    mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(
        &test,
        "run clamped write_stdin output test",
        SandboxPolicy::DangerFullAccess,
    )
    .await?;

    let start_output = wait_for_raw_unified_exec_output(&test, start_call_id).await?;
    assert!(
        start_output.process_id.is_some(),
        "start command should leave a running process for write_stdin"
    );

    let stdin_output = wait_for_raw_unified_exec_output(&test, stdin_call_id).await?;
    assert_eq!(stdin_output.original_token_count, Some(9_492));
    let stdin_output_text = stdin_output.output.replace("\r\n", "\n");
    assert_regex_match(
        r"^Total output lines: 1000\n\ngo\nSTDIN-LINE-0001 y{20}\nSTDIN-LINE-0002 y{20}\nSTDIN-LINE-0003 yyyy…9442 tokens truncated…7 y{20}\nSTDIN-LINE-0998 y{20}\nSTDIN-LINE-0999 y{20}\n$",
        &stdin_output_text,
    );

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    Ok(())
}
