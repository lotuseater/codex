use crate::unified_exec_support::collect_tool_outputs;
use crate::unified_exec_support::submit_unified_exec_turn;
use anyhow::Result;
use codex_core_test_runtime::process::process_is_alive;
use codex_core_test_runtime::process::wait_for_pid_file;
use codex_core_test_runtime::process::wait_for_process_exit;
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
use codex_core_test_runtime::wait_for_event_match;
use codex_features::Feature;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::SandboxPolicy;
use pretty_assertions::assert_eq;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_stdin_returns_exit_metadata_and_clears_session() -> Result<()> {
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

    let start_call_id = "uexec-cat-start";
    let send_call_id = "uexec-cat-send";
    let exit_call_id = "uexec-cat-exit";

    let start_args = serde_json::json!({
        "cmd": "/bin/cat",
        "yield_time_ms": 500,
        "tty": true,
    });
    let send_args = serde_json::json!({
        "chars": "hello unified exec\n",
        "session_id": 1000,
        "yield_time_ms": 500,
    });
    let exit_args = serde_json::json!({
        "chars": "\u{0004}",
        "session_id": 1000,
        "yield_time_ms": 500,
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
                send_call_id,
                "write_stdin",
                &serde_json::to_string(&send_args)?,
            ),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_response_created("resp-3"),
            ev_function_call(
                exit_call_id,
                "write_stdin",
                &serde_json::to_string(&exit_args)?,
            ),
            ev_completed("resp-3"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "all done"),
            ev_completed("resp-4"),
        ]),
    ];
    let request_log = mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(
        &test,
        "test write_stdin exit behavior",
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

    let start_output = outputs
        .get(start_call_id)
        .expect("missing start output for exec_command");
    let process_id = start_output
        .process_id
        .clone()
        .expect("expected process id from exec_command");
    assert!(
        process_id.len() > 3,
        "process_id should be at least 4 digits, got {process_id}"
    );
    assert!(
        start_output.exit_code.is_none(),
        "initial exec_command should not include exit_code while session is running"
    );

    let send_output = outputs
        .get(send_call_id)
        .expect("missing write_stdin echo output");
    let echoed = send_output.output.as_str();
    assert!(
        echoed.contains("hello unified exec"),
        "expected echoed output from cat, got {echoed:?}"
    );
    let echoed_session = send_output
        .process_id
        .clone()
        .expect("write_stdin should return process id while process is running");
    assert_eq!(
        echoed_session, process_id,
        "write_stdin should reuse existing process id"
    );
    assert!(
        send_output.exit_code.is_none(),
        "write_stdin should not include exit_code while process is running"
    );

    let exit_output = outputs
        .get(exit_call_id)
        .expect("missing exit metadata output");
    assert!(
        exit_output.process_id.is_none(),
        "process_id should be omitted once the process exits"
    );
    let exit_code = exit_output
        .exit_code
        .expect("expected exit_code after sending EOF");
    assert_eq!(exit_code, 0, "cat should exit cleanly after EOF");

    let exit_chunk = exit_output
        .chunk_id
        .as_ref()
        .expect("missing chunk id for exit output");
    assert!(
        exit_chunk.chars().all(|c| c.is_ascii_hexdigit()),
        "chunk id should be hexadecimal: {exit_chunk}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_emits_end_event_when_session_dies_via_stdin() -> Result<()> {
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

    let start_call_id = "uexec-end-on-exit-start";
    let start_args = serde_json::json!({
        "cmd": "/bin/cat",
        "yield_time_ms": 200,
        "tty": true,
    });

    let echo_call_id = "uexec-end-on-exit-echo";
    let echo_args = serde_json::json!({
        "chars": "bye-END\n",
        "session_id": 1000,
        "yield_time_ms": 300,
    });

    let exit_call_id = "uexec-end-on-exit";
    let exit_args = serde_json::json!({
        "chars": "\u{0004}",
        "session_id": 1000,
        "yield_time_ms": 500,
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
                echo_call_id,
                "write_stdin",
                &serde_json::to_string(&echo_args)?,
            ),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_response_created("resp-3"),
            ev_function_call(
                exit_call_id,
                "write_stdin",
                &serde_json::to_string(&exit_args)?,
            ),
            ev_completed("resp-3"),
        ]),
        sse(vec![
            ev_response_created("resp-4"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-4"),
        ]),
    ];
    mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(&test, "end on exit", SandboxPolicy::DangerFullAccess).await?;

    // We expect the ExecCommandEnd event to match the initial exec_command call_id.
    let end_event = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::ExecCommandEnd(ev) if ev.call_id == start_call_id => Some(ev.clone()),
        _ => None,
    })
    .await;

    assert_eq!(end_event.exit_code, 0);

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_keeps_long_running_session_after_turn_end() -> Result<()> {
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
    let TestCodex {
        codex,
        cwd,
        session_configured,
        ..
    } = builder.build(&server).await?;

    let temp_dir = tempfile::tempdir()?;
    let pid_path = temp_dir.path().join("uexec_pid");
    let pid_path_str = pid_path.to_string_lossy();

    let call_id = "uexec-long-running";
    let command = format!("printf '%s' $$ > '{pid_path_str}' && exec sleep 3000");
    let args = json!({
        "cmd": command,
        "yield_time_ms": 250,
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
    mount_sse_sequence(&server, responses).await;

    let session_model = session_configured.model.clone();

    codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "keep unified exec process after turn end".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd.path().to_path_buf(),
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            permission_profile: None,
            model: session_model,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    let begin_event = wait_for_event_match(&codex, |msg| match msg {
        EventMsg::ExecCommandBegin(ev) if ev.call_id == call_id => Some(ev.clone()),
        _ => None,
    })
    .await;

    let _begin_process_id = begin_event
        .process_id
        .clone()
        .expect("expected process_id for long-running unified exec process");

    let pid = wait_for_pid_file(&pid_path).await?;
    assert!(
        pid.chars().all(|ch| ch.is_ascii_digit()),
        "expected numeric pid, got {pid:?}"
    );

    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    assert!(
        process_is_alive(&pid)?,
        "expected unified exec process to remain alive after turn completion"
    );

    codex.submit(Op::Shutdown).await?;
    wait_for_event(&codex, |event| matches!(event, EventMsg::ShutdownComplete)).await;
    wait_for_process_exit(&pid).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_interrupt_preserves_long_running_session() -> Result<()> {
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
    let TestCodex {
        codex,
        cwd,
        session_configured,
        ..
    } = builder.build(&server).await?;

    let temp_dir = tempfile::tempdir()?;
    let pid_path = temp_dir.path().join("uexec_pid_interrupt");
    let pid_path_str = pid_path.to_string_lossy();

    let call_id = "uexec-long-running-interrupt";
    let command = format!("printf '%s' $$ > '{pid_path_str}' && exec sleep 3000");
    let args = json!({
        "cmd": command,
        "yield_time_ms": 30000,
    });

    let responses = vec![sse(vec![
        ev_response_created("resp-1"),
        ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
        ev_completed("resp-1"),
    ])];
    mount_sse_sequence(&server, responses).await;

    let session_model = session_configured.model.clone();

    codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "interrupt long-running unified exec".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd.path().to_path_buf(),
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            permission_profile: None,
            model: session_model,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    let _begin_event = wait_for_event_match(&codex, |msg| match msg {
        EventMsg::ExecCommandBegin(ev) if ev.call_id == call_id => Some(ev.clone()),
        _ => None,
    })
    .await;

    let pid = wait_for_pid_file(&pid_path).await?;
    assert!(
        pid.chars().all(|ch| ch.is_ascii_digit()),
        "expected numeric pid, got {pid:?}"
    );

    codex.submit(Op::Interrupt).await?;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnAborted(_))).await;

    assert!(
        process_is_alive(&pid)?,
        "expected unified exec process to remain alive after interrupt"
    );

    codex.submit(Op::CleanBackgroundTerminals).await?;
    wait_for_process_exit(&pid).await?;

    Ok(())
}
