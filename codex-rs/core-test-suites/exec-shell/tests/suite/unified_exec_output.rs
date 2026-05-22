use std::ffi::OsStr;
use std::fs;

use anyhow::Context;
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
use codex_core_test_runtime::test_codex::TestCodexHarness;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::wait_for_event;
use codex_core_test_runtime::wait_for_event_match;
use codex_features::Feature;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ExecCommandSource;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::SandboxPolicy;
use pretty_assertions::assert_eq;
use serde_json::json;

use crate::unified_exec_support::collect_tool_outputs;
use crate::unified_exec_support::create_workspace_directory;
use crate::unified_exec_support::submit_unified_exec_turn;
use crate::unified_exec_support::wait_for_raw_unified_exec_output;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_intercepts_apply_patch_exec_command() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_windows!(Ok(()));

    let builder = test_codex().with_config(|config| {
        config.include_apply_patch_tool = true;
        config.use_experimental_unified_exec_tool = true;
        if let Err(err) = config.features.enable(Feature::UnifiedExec) {
            panic!("test config should allow feature update: {err}");
        }
    });
    let harness = TestCodexHarness::with_builder(builder).await?;

    let patch =
        "*** Begin Patch\n*** Add File: uexec_apply.txt\n+hello from unified exec\n*** End Patch";
    let command = format!("apply_patch <<'EOF'\n{patch}\nEOF\n");
    let call_id = "uexec-apply-patch";
    let args = json!({
        "cmd": command,
        // The intercepted apply_patch path spawns a helper process, which can
        // take longer than a tiny unified-exec yield deadline on CI.
        "yield_time_ms": 5_000,
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
    mount_sse_sequence(harness.server(), responses).await;

    let test = harness.test();
    let codex = test.codex.clone();
    let cwd = test.cwd_path().to_path_buf();
    let session_model = test.session_configured.model.clone();

    codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "apply patch via unified exec".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd,
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

    let mut saw_patch_begin = false;
    let mut patch_end = None;
    let mut saw_exec_begin = false;
    let mut saw_exec_end = false;
    wait_for_event(&codex, |event| match event {
        EventMsg::PatchApplyBegin(begin) if begin.call_id == call_id => {
            saw_patch_begin = true;
            assert!(
                begin
                    .changes
                    .keys()
                    .any(|path| path.file_name() == Some(OsStr::new("uexec_apply.txt"))),
                "expected apply_patch changes to target uexec_apply.txt",
            );
            false
        }
        EventMsg::PatchApplyEnd(end) if end.call_id == call_id => {
            patch_end = Some(end.clone());
            false
        }
        EventMsg::ExecCommandBegin(event) if event.call_id == call_id => {
            saw_exec_begin = true;
            false
        }
        EventMsg::ExecCommandEnd(event) if event.call_id == call_id => {
            saw_exec_end = true;
            false
        }
        EventMsg::TurnComplete(_) => true,
        _ => false,
    })
    .await;

    assert!(
        saw_patch_begin,
        "expected apply_patch to emit PatchApplyBegin"
    );
    let patch_end = patch_end.expect("expected apply_patch to emit PatchApplyEnd");
    assert!(
        patch_end.success,
        "expected apply_patch to finish successfully: stdout={:?} stderr={:?}",
        patch_end.stdout, patch_end.stderr,
    );
    assert!(
        !saw_exec_begin,
        "apply_patch should be intercepted before exec_command begin"
    );
    assert!(
        !saw_exec_end,
        "apply_patch should not emit exec_command end events"
    );

    let output = harness.function_call_stdout(call_id).await;
    assert!(
        output.contains("Success. Updated the following files:"),
        "expected apply_patch output, got: {output:?}"
    );
    assert!(
        output.contains("A uexec_apply.txt"),
        "expected apply_patch file summary, got: {output:?}"
    );
    assert_eq!(
        fs::read_to_string(harness.path("uexec_apply.txt"))?,
        "hello from unified exec\n"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_emits_exec_command_begin_event() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_windows!(Ok(()));

    let server = start_mock_server().await;

    let mut builder = test_codex().with_model("gpt-5.2").with_config(|config| {
        config.use_experimental_unified_exec_tool = true;
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
    });
    let test = builder.build_remote_aware(&server).await?;
    let cwd = test.config.cwd.to_path_buf();

    let call_id = "uexec-begin-event";
    let args = json!({
        "shell": "bash".to_string(),
        "cmd": "/bin/echo hello unified exec".to_string(),
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
            ev_assistant_message("msg-1", "finished"),
            ev_completed("resp-2"),
        ]),
    ];
    mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(&test, "emit begin event", SandboxPolicy::DangerFullAccess).await?;

    let begin_event = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::ExecCommandBegin(event) if event.call_id == call_id => Some(event.clone()),
        _ => None,
    })
    .await;

    assert_command(&begin_event.command, "-lc", "/bin/echo hello unified exec");

    assert_eq!(begin_event.cwd.as_path(), cwd.as_path());

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_resolves_relative_workdir() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_windows!(Ok(()));

    let server = start_mock_server().await;

    let mut builder = test_codex().with_model("gpt-5.2").with_config(|config| {
        config.use_experimental_unified_exec_tool = true;
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
    });
    let test = builder.build_remote_aware(&server).await?;

    let workdir_rel = std::path::PathBuf::from("uexec_relative_workdir");
    let workdir = create_workspace_directory(&test, &workdir_rel).await?;

    let call_id = "uexec-workdir-relative";
    let args = json!({
        "cmd": "pwd",
        "yield_time_ms": 250,
        "workdir": workdir_rel.to_string_lossy().to_string(),
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_assistant_message("msg-1", "finished"),
            ev_completed("resp-2"),
        ]),
    ];
    mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(
        &test,
        "run relative workdir test",
        SandboxPolicy::DangerFullAccess,
    )
    .await?;

    let begin_event = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::ExecCommandBegin(event) if event.call_id == call_id => Some(event.clone()),
        _ => None,
    })
    .await;

    assert_eq!(
        begin_event.cwd.as_path(),
        workdir.as_path(),
        "exec_command cwd should resolve relative workdir against turn cwd",
    );

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "flaky"]
async fn unified_exec_respects_workdir_override() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_windows!(Ok(()));

    let server = start_mock_server().await;

    let mut builder = test_codex().with_model("gpt-5.2").with_config(|config| {
        config.use_experimental_unified_exec_tool = true;
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
    });
    let test = builder.build_remote_aware(&server).await?;

    let workdir = create_workspace_directory(&test, "uexec_workdir_test").await?;

    let call_id = "uexec-workdir";
    let args = json!({
        "cmd": "pwd",
        "yield_time_ms": 250,
        "workdir": workdir.to_string_lossy().to_string(),
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_assistant_message("msg-1", "finished"),
            ev_completed("resp-2"),
        ]),
    ];
    let request_log = mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(&test, "run workdir test", SandboxPolicy::DangerFullAccess).await?;

    let begin_event = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::ExecCommandBegin(event) if event.call_id == call_id => Some(event.clone()),
        _ => None,
    })
    .await;

    assert_eq!(
        begin_event.cwd.as_path(),
        workdir.as_path(),
        "exec_command cwd should reflect the requested workdir override"
    );

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = request_log.requests();
    assert!(!requests.is_empty(), "expected at least one POST request");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_emits_exec_command_end_event() -> Result<()> {
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

    let call_id = "uexec-end-event";
    let args = json!({
        "cmd": "/bin/echo END-EVENT".to_string(),
        "yield_time_ms": 250,
    });
    let poll_call_id = "uexec-end-event-poll";
    let poll_args = json!({
        "chars": "",
        "session_id": 1000,
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
            ev_function_call(
                poll_call_id,
                "write_stdin",
                &serde_json::to_string(&poll_args)?,
            ),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_response_created("resp-3"),
            ev_assistant_message("msg-1", "finished"),
            ev_completed("resp-3"),
        ]),
    ];
    mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(&test, "emit end event", SandboxPolicy::DangerFullAccess).await?;

    let end_event = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::ExecCommandEnd(ev) if ev.call_id == call_id => Some(ev.clone()),
        _ => None,
    })
    .await;

    assert_eq!(end_event.exit_code, 0);
    assert!(
        end_event.aggregated_output.contains("END-EVENT"),
        "expected aggregated output to contain marker"
    );

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_emits_output_delta_for_exec_command() -> Result<()> {
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

    let call_id = "uexec-delta-1";
    let args = json!({
        "cmd": "printf 'HELLO-UEXEC'",
        "yield_time_ms": 1000,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_assistant_message("msg-1", "finished"),
            ev_completed("resp-2"),
        ]),
    ];
    mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(&test, "emit delta", SandboxPolicy::DangerFullAccess).await?;

    let event = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::ExecCommandEnd(ev) if ev.call_id == call_id => Some(ev.clone()),
        _ => None,
    })
    .await;

    let text = event.stdout;
    assert!(
        text.contains("HELLO-UEXEC"),
        "delta chunk missing expected text: {text:?}",
    );

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_full_lifecycle_with_background_end_event() -> Result<()> {
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

    let call_id = "uexec-full-lifecycle";
    // This timing force the long-standing PTY
    let args = json!({
        "cmd": "sleep 0.5; printf 'HELLO-FULL-LIFECYCLE'",
        "yield_time_ms": 1000,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_assistant_message("msg-1", "finished"),
            ev_completed("resp-2"),
        ]),
    ];
    mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(
        &test,
        "exercise full unified exec lifecycle",
        SandboxPolicy::DangerFullAccess,
    )
    .await?;

    let mut begin_event = None;
    let mut end_event = None;
    let mut task_completed = false;

    loop {
        let msg = wait_for_event(&test.codex, |_| true).await;
        match msg {
            EventMsg::ExecCommandBegin(ev) if ev.call_id == call_id => begin_event = Some(ev),
            EventMsg::ExecCommandEnd(ev) if ev.call_id == call_id => {
                assert!(
                    end_event.is_none(),
                    "expected a single ExecCommandEnd event for this call id"
                );
                end_event = Some(ev);
                if task_completed && end_event.is_some() {
                    break;
                }
            }
            EventMsg::TurnComplete(_) => {
                task_completed = true;
                if task_completed && end_event.is_some() {
                    break;
                }
            }
            _ => {}
        }
    }

    let begin_event = begin_event.expect("expected ExecCommandBegin event");
    assert_eq!(begin_event.call_id, call_id);
    assert!(
        begin_event.process_id.is_some(),
        "begin event should include a process_id for a long-lived session"
    );

    let end_event = end_event.expect("expected ExecCommandEnd event");
    assert_eq!(end_event.call_id, call_id);
    assert_eq!(end_event.exit_code, 0);
    assert!(
        end_event.process_id.is_some(),
        "end event should include process_id emitted by background watcher"
    );
    assert!(
        end_event.aggregated_output.contains("HELLO-FULL-LIFECYCLE"),
        "aggregated_output should contain the full PTY transcript; got {:?}",
        end_event.aggregated_output
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_emits_one_begin_and_one_end_event() -> Result<()> {
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

    let open_call_id = "uexec-open-session";
    let open_args = json!({
        "shell": "bash".to_string(),
        "cmd": "sleep 0.1".to_string(),
        "yield_time_ms": 10,
    });

    let poll_call_id = "uexec-poll-empty";
    let poll_args = json!({
        "chars": "",
        "session_id": 1000,
        "yield_time_ms": 150,
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
                poll_call_id,
                "write_stdin",
                &serde_json::to_string(&poll_args)?,
            ),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_response_created("resp-3"),
            ev_assistant_message("msg-1", "complete"),
            ev_completed("resp-3"),
        ]),
    ];
    mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(
        &test,
        "check poll event behavior",
        SandboxPolicy::DangerFullAccess,
    )
    .await?;

    let mut begin_events = Vec::new();
    let mut end_events = Vec::new();
    let mut task_completed = false;
    loop {
        let event_msg = wait_for_event(&test.codex, |_| true).await;
        match event_msg {
            EventMsg::ExecCommandBegin(event) if event.call_id == open_call_id => {
                begin_events.push(event);
            }
            EventMsg::ExecCommandEnd(event) if event.call_id == open_call_id => {
                end_events.push(event);
            }
            EventMsg::TurnComplete(_) => {
                task_completed = true;
            }
            _ => {}
        }
        if task_completed && !end_events.is_empty() {
            break;
        }
    }

    assert_eq!(
        begin_events.len(),
        1,
        "expected begin events for the startup command"
    );

    assert_eq!(
        end_events.len(),
        1,
        "expected end event for the write_stdin call"
    );

    let open_event = &begin_events[0];

    assert_command(&open_event.command, "-lc", "sleep 0.1");

    assert!(
        open_event.interaction_input.is_none(),
        "startup begin events should not include interaction input"
    );
    assert_eq!(open_event.source, ExecCommandSource::UnifiedExecStartup);

    let end_event = &end_events[0];
    assert_eq!(end_event.call_id, open_call_id);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_reports_chunk_and_exit_metadata() -> Result<()> {
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

    let call_id = "uexec-metadata";
    let args = serde_json::json!({
        "cmd": "printf 'token one token two token three token four token five token six token seven'",
        "yield_time_ms": 500,
        "max_output_tokens": 6,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    ];
    let request_log = mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(&test, "run metadata test", SandboxPolicy::DangerFullAccess).await?;

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
    let metadata = outputs
        .get(call_id)
        .expect("missing exec_command metadata output");

    let chunk_id = metadata.chunk_id.as_ref().expect("missing chunk_id");
    assert_eq!(chunk_id.len(), 6, "chunk id should be 6 hex characters");
    assert!(
        chunk_id.chars().all(|c| c.is_ascii_hexdigit()),
        "chunk id should be hexadecimal: {chunk_id}"
    );

    let wall_time = metadata.wall_time_seconds;
    assert!(
        wall_time >= 0.0,
        "wall_time_seconds should be non-negative, got {wall_time}"
    );

    assert!(
        metadata.process_id.is_none(),
        "exec_command for a completed process should not include process_id"
    );

    let exit_code = metadata.exit_code.expect("expected exit_code");
    assert_eq!(exit_code, 0, "expected successful exit");

    let output_text = &metadata.output;
    assert!(
        output_text.contains("tokens truncated"),
        "expected truncation notice in output: {output_text:?}"
    );

    let original_tokens = metadata
        .original_token_count
        .expect("missing original_token_count") as usize;
    assert!(
        original_tokens > 6,
        "original token count should exceed max_output_tokens"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_clamps_model_requested_max_output_tokens_to_policy() -> Result<()> {
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

    let call_id = "uexec-clamped-max-output";
    let args = serde_json::json!({
        "cmd": "line_number=1; while [ \"$line_number\" -le 999 ]; do printf 'EXEC-LINE-%04d xxxxxxxxxxxxxxxxxxxx\\n' \"$line_number\"; line_number=$((line_number + 1)); done",
        "yield_time_ms": 3_000,
        "max_output_tokens": 70_000,
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

    submit_unified_exec_turn(
        &test,
        "run clamped max output test",
        SandboxPolicy::DangerFullAccess,
    )
    .await?;

    let output = wait_for_raw_unified_exec_output(&test, call_id).await?;
    assert_eq!(output.original_token_count, Some(8_991));
    let output_text = output.output.replace("\r\n", "\n");
    assert_regex_match(
        r"^Total output lines: 999\n\nEXEC-LINE-0001 x{20}\nEXEC-LINE-0002 x{20}\nEXEC-LINE-0003 x{13}…8941 tokens truncated…E-0997 x{20}\nEXEC-LINE-0998 x{20}\nEXEC-LINE-0999 x{20}\n$",
        &output_text,
    );

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_respects_early_exit_notifications() -> Result<()> {
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

    let call_id = "uexec-early-exit";
    let args = serde_json::json!({
        "cmd": "sleep 0.05",
        "yield_time_ms": 31415,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    ];
    let request_log = mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(
        &test,
        "watch early exit timing",
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
        .expect("missing early exit unified_exec output");

    assert!(
        output.process_id.is_none(),
        "short-lived process should not keep a session alive"
    );
    assert_eq!(
        output.exit_code,
        Some(0),
        "short-lived process should exit successfully"
    );

    let wall_time = output.wall_time_seconds;
    assert!(
        wall_time < 0.75,
        "wall_time should reflect early exit rather than the full yield time; got {wall_time}"
    );
    assert!(
        output.output.is_empty(),
        "sleep command should not emit output, got {:?}",
        output.output
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// Skipped on arm because the ctor logic to handle arg0 doesn't work on ARM
#[cfg(not(target_arch = "arm"))]
async fn unified_exec_formats_large_output_summary() -> Result<()> {
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

    let script = r#"python3 - <<'PY'
import sys
sys.stdout.write("token token \n" * 5000)
PY
"#;

    let call_id = "uexec-large-output";
    let args = serde_json::json!({
        "cmd": script,
        "max_output_tokens": 100,
        "yield_time_ms": 500,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    ];
    let request_log = mount_sse_sequence(&server, responses).await;

    submit_unified_exec_turn(
        &test,
        "summarize large output",
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
    let large_output = outputs.get(call_id).expect("missing large output summary");

    let output_text = large_output.output.replace("\r\n", "\n");
    let truncated_pattern = r"(?s)^Total output lines: \d+\n\n(token token \n){5,}.*…\d+ tokens truncated….*(token token \n){5,}$";
    assert_regex_match(truncated_pattern, &output_text);

    let original_tokens = large_output
        .original_token_count
        .expect("missing original_token_count for large output summary");
    assert!(original_tokens > 0);

    Ok(())
}

fn assert_command(command: &[String], expected_args: &str, expected_cmd: &str) {
    assert_eq!(command.len(), 3);
    let shell_path = &command[0];
    assert!(
        shell_path == "/bin/bash"
            || shell_path == "/usr/bin/bash"
            || shell_path == "/usr/local/bin/bash"
            || shell_path.ends_with("/bash"),
        "unexpected bash path: {shell_path}"
    );
    assert_eq!(command[1], expected_args);
    assert_eq!(command[2], expected_cmd);
}
