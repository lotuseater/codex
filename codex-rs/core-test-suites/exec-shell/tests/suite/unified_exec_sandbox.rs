use anyhow::Result;
use codex_core_test_runtime::assert_regex_match;
use codex_core_test_runtime::managed_network_requirements_loader;
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
use codex_core_test_runtime::test_codex::TestCodex;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::wait_for_event;
use codex_features::Feature;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ExecCommandStatus;
use codex_protocol::protocol::SandboxPolicy;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

use crate::unified_exec_support::collect_tool_outputs;
use crate::unified_exec_support::submit_unified_exec_turn;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_network_denial_emits_failed_background_end_event() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_windows!(Ok(()));

    let server = start_mock_server().await;
    let (test, sandbox_policy) = unified_exec_network_denial_test(&server).await?;

    let call_id = "uexec-network-denied";
    let args = json!({
        "cmd": "python3 -c \"import os, socket, time, urllib.parse; time.sleep(0.3); proxy = urllib.parse.urlparse(os.environ['HTTP_PROXY']); sock = socket.create_connection((proxy.hostname, proxy.port), timeout=2); sock.sendall(b'GET http://codex-network-denied.invalid/ HTTP/1.1\\r\\nHost: codex-network-denied.invalid\\r\\n\\r\\n'); sock.recv(1024); time.sleep(5)\"",
        "yield_time_ms": 50,
    });
    let response_mock =
        mount_unified_exec_network_denial_responses(&server, call_id, &args).await?;

    submit_unified_exec_turn(&test, "exercise network denial", sandbox_policy).await?;

    let (end_event, turn_completed) =
        wait_for_unified_exec_end(&test, call_id, &response_mock).await;

    assert_eq!(end_event.status, ExecCommandStatus::Failed);
    assert_eq!(end_event.exit_code, -1);
    assert!(
        end_event.aggregated_output.contains("Network access"),
        "expected network denial message in aggregated output: {:?}",
        end_event.aggregated_output
    );
    assert!(
        end_event.process_id.is_some(),
        "background denial should end the stored unified exec process"
    );

    if !turn_completed {
        wait_for_event(&test.codex, |event| {
            matches!(event, EventMsg::TurnComplete(_))
        })
        .await;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_short_lived_network_denial_emits_failed_end_event() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_windows!(Ok(()));

    let server = start_mock_server().await;
    let (test, sandbox_policy) = unified_exec_network_denial_test(&server).await?;

    let call_id = "uexec-short-network-denied";
    let args = json!({
        "cmd": "python3 -c \"import os, socket, urllib.parse; proxy = urllib.parse.urlparse(os.environ['HTTP_PROXY']); sock = socket.create_connection((proxy.hostname, proxy.port), timeout=2); sock.sendall(b'GET http://codex-short-network-denied.invalid/ HTTP/1.1\\r\\nHost: codex-short-network-denied.invalid\\r\\n\\r\\n'); sock.recv(1024)\"",
        "yield_time_ms": 1000,
    });
    let response_mock =
        mount_unified_exec_network_denial_responses(&server, call_id, &args).await?;

    submit_unified_exec_turn(&test, "exercise short network denial", sandbox_policy).await?;

    let (end_event, turn_completed) =
        wait_for_unified_exec_end(&test, call_id, &response_mock).await;

    assert_eq!(end_event.status, ExecCommandStatus::Failed);
    assert_eq!(end_event.exit_code, -1);
    assert!(
        end_event.aggregated_output.contains("Network access"),
        "expected network denial message in aggregated output: {:?}",
        end_event.aggregated_output
    );
    assert!(
        end_event.process_id.is_some(),
        "short-lived denial should still emit an end event for the command"
    );

    if !turn_completed {
        wait_for_event(&test.codex, |event| {
            matches!(event, EventMsg::TurnComplete(_))
        })
        .await;
    }
    Ok(())
}

#[allow(clippy::expect_used)]
async fn unified_exec_network_denial_test(
    server: &wiremock::MockServer,
) -> Result<(TestCodex, SandboxPolicy)> {
    use codex_config::Constrained;
    use std::sync::Arc;
    use tempfile::TempDir;

    let home = Arc::new(TempDir::new()?);
    fs::write(
        home.path().join("config.toml"),
        r#"default_permissions = "workspace"

[permissions.workspace.filesystem]
":minimal" = "read"

[permissions.workspace.network]
enabled = true
mode = "limited"
allow_local_binding = true
"#,
    )?;
    let mut sandbox_policy = SandboxPolicy::new_workspace_write_policy();
    if let SandboxPolicy::WorkspaceWrite { network_access, .. } = &mut sandbox_policy {
        *network_access = true;
    }
    let sandbox_policy_for_config = sandbox_policy.clone();
    let mut builder = test_codex()
        .with_home(home)
        .with_cloud_requirements(managed_network_requirements_loader())
        .with_config(move |config| {
            config.use_experimental_unified_exec_tool = true;
            config
                .features
                .enable(Feature::UnifiedExec)
                .expect("test config should allow feature update");
            config.permissions.approval_policy = Constrained::allow_any(AskForApproval::Never);
            config.permissions.permission_profile = Constrained::allow_any(
                PermissionProfile::from_legacy_sandbox_policy(&sandbox_policy_for_config),
            );
        });
    let test = builder.build_remote_aware(server).await?;
    assert!(
        test.config.permissions.network.is_some(),
        "expected managed network proxy config to be present"
    );

    Ok((test, sandbox_policy))
}

async fn mount_unified_exec_network_denial_responses(
    server: &wiremock::MockServer,
    call_id: &str,
    args: &Value,
) -> Result<codex_core_test_runtime::responses::ResponseMock> {
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
    Ok(mount_sse_sequence(server, responses).await)
}

async fn wait_for_unified_exec_end(
    test: &TestCodex,
    call_id: &str,
    response_mock: &codex_core_test_runtime::responses::ResponseMock,
) -> (codex_protocol::protocol::ExecCommandEndEvent, bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut observed_events = Vec::new();
    let mut turn_completed = false;
    let end_event = loop {
        let remaining = deadline
            .checked_duration_since(std::time::Instant::now())
            .unwrap_or_default();
        if remaining.is_zero() {
            panic!(
                "timed out waiting for network denial end event; observed {observed_events:?}; response requests: {}",
                response_mock.requests().len()
            );
        }
        let event = match tokio::time::timeout(remaining, test.codex.next_event()).await {
            Ok(Ok(event)) => event.msg,
            Ok(Err(err)) => panic!("event stream ended unexpectedly: {err}"),
            Err(_) => panic!(
                "timed out waiting for network denial end event; observed {observed_events:?}; response requests: {}",
                response_mock.requests().len()
            ),
        };
        turn_completed |= matches!(event, EventMsg::TurnComplete(_));
        observed_events.push(format!("{event:?}"));
        if let EventMsg::ExecCommandEnd(ev) = event
            && ev.call_id == call_id
        {
            break ev;
        }
    };
    (end_event, turn_completed)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_runs_under_sandbox() -> Result<()> {
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
    let TestCodex {
        codex,
        cwd,
        session_configured,
        ..
    } = builder.build(&server).await?;

    let call_id = "uexec";
    let args = serde_json::json!({
        "cmd": "echo 'hello'",
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

    let session_model = session_configured.model.clone();

    codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "summarize large output".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd.path().to_path_buf(),
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            // Important!
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
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

    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert!(!requests.is_empty(), "expected at least one POST request");
    let bodies = requests
        .into_iter()
        .map(|request| request.body_json())
        .collect::<Vec<_>>();

    let outputs = collect_tool_outputs(&bodies)?;
    let output = outputs.get(call_id).expect("missing output");

    assert_regex_match("hello[\r\n]+", &output.output);

    Ok(())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_enforces_glob_deny_read_policy() -> Result<()> {
    use codex_config::Constrained;
    use codex_protocol::models::PermissionProfile;
    use codex_protocol::permissions::FileSystemAccessMode;
    use codex_protocol::permissions::FileSystemPath;
    use codex_protocol::permissions::FileSystemSandboxEntry;
    use codex_protocol::permissions::FileSystemSandboxPolicy;
    use codex_protocol::permissions::NetworkSandboxPolicy;

    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = start_mock_server().await;
    let read_only_policy = SandboxPolicy::new_read_only_policy();
    let read_only_policy_for_config = read_only_policy.clone();
    let mut builder = test_codex().with_config(move |config| {
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
        config
            .set_legacy_sandbox_policy(read_only_policy_for_config)
            .expect("set sandbox policy");
        let mut file_system_sandbox_policy = FileSystemSandboxPolicy::default();
        file_system_sandbox_policy
            .entries
            .push(FileSystemSandboxEntry {
                path: FileSystemPath::GlobPattern {
                    pattern: format!("{}/**/*.env", config.cwd.as_path().display()),
                },
                access: FileSystemAccessMode::None,
            });
        config.permissions.permission_profile =
            Constrained::allow_any(PermissionProfile::from_runtime_permissions(
                &file_system_sandbox_policy,
                NetworkSandboxPolicy::Restricted,
            ));
    });
    let TestCodex {
        codex,
        cwd,
        session_configured,
        ..
    } = builder.build(&server).await?;

    let fixture_dir = cwd.path().join("glob-deny-read");
    fs::create_dir_all(&fixture_dir).context("create glob deny-read fixture directory")?;
    let denied_path = fixture_dir.join("secret.env");
    let allowed_path = fixture_dir.join("notes.txt");
    let secret = "unified exec glob deny-read secret";
    let allowed = "unified exec glob deny-read allowed";
    fs::write(&denied_path, format!("{secret}\n")).context("write denied fixture")?;
    fs::write(&allowed_path, format!("{allowed}\n")).context("write allowed fixture")?;

    let call_id = "uexec-glob-deny-read";
    let cmd = format!(
        "read_status=0; cat {denied_path:?} || read_status=$?; cat {allowed_path:?}; exit $read_status"
    );
    let args = serde_json::json!({
        "cmd": cmd,
        "yield_time_ms": 5_000,
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

    let session_model = session_configured.model.clone();
    codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "read the fixture files".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd.path().to_path_buf(),
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy: read_only_policy,
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

    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert!(!requests.is_empty(), "expected at least one POST request");
    let bodies = requests
        .into_iter()
        .map(|request| request.body_json())
        .collect::<Vec<_>>();

    let outputs = collect_tool_outputs(&bodies)?;
    let output = outputs.get(call_id).expect("missing output");

    assert!(
        output.exit_code.is_some_and(|code| code != 0),
        "glob deny-read should surface a non-zero exit code: {output:?}"
    );
    assert!(
        output.output.contains(allowed),
        "expected allowed file contents in unified exec output: {output:?}"
    );
    assert!(
        !output.output.contains(secret),
        "denied file contents leaked into unified exec output: {output:?}"
    );
    let output_lower = output.output.to_lowercase();
    let has_denial = output_lower.contains("permission denied")
        || output_lower.contains("operation not permitted")
        || output_lower.contains("read-only file system");
    assert!(
        has_denial,
        "expected sandbox denial details in unified exec output: {output:?}"
    );

    Ok(())
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_python_prompt_under_seatbelt() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let python = match which::which("python").or_else(|_| which::which("python3")) {
        Ok(path) => path,
        Err(_) => {
            eprintln!("python not found in PATH, skipping test.");
            return Ok(());
        }
    };

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

    let startup_call_id = "uexec-python-seatbelt";
    let startup_args = serde_json::json!({
        "cmd": format!("{} -i", python.display()),
        "yield_time_ms": 1_500,
        "tty": true,
    });

    let exit_call_id = "uexec-python-exit";
    let exit_args = serde_json::json!({
        "chars": "exit()\n",
        "session_id": 1000,
        "yield_time_ms": 1_500,
    });

    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(
                startup_call_id,
                "exec_command",
                &serde_json::to_string(&startup_args)?,
            ),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_function_call(
                exit_call_id,
                "write_stdin",
                &serde_json::to_string(&exit_args)?,
            ),
            ev_completed("resp-2"),
        ]),
        sse(vec![
            ev_response_created("resp-3"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-3"),
        ]),
    ];
    let request_log = mount_sse_sequence(&server, responses).await;

    let session_model = session_configured.model.clone();

    codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "start python under seatbelt".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd.path().to_path_buf(),
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
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

    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert!(!requests.is_empty(), "expected at least one POST request");
    let bodies = requests
        .into_iter()
        .map(|request| request.body_json())
        .collect::<Vec<_>>();

    let outputs = collect_tool_outputs(&bodies)?;
    let startup_output = outputs
        .get(startup_call_id)
        .expect("missing python startup output");

    let output_text = startup_output.output.replace("\r\n", "\n");
    // This assert that we are in a TTY.
    assert!(
        output_text.contains(">>>"),
        "python prompt missing from seatbelt output: {output_text:?}"
    );

    assert_eq!(
        startup_output.process_id.as_deref(),
        Some("1000"),
        "python session should stay alive for follow-up input"
    );

    let exit_output = outputs
        .get(exit_call_id)
        .expect("missing python exit output");

    assert_eq!(
        exit_output.exit_code,
        Some(0),
        "python should exit cleanly after exit()"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_runs_on_all_platforms() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = start_mock_server().await;

    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
    });
    let test = builder.build_remote_aware(&server).await?;

    let call_id = "uexec";
    let args = serde_json::json!({
        "cmd": "echo 'hello crossplat'",
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
    let output = outputs.get(call_id).expect("missing output");

    // TODO: Weaker match because windows produces control characters
    assert_regex_match(".*hello crossplat.*", &output.output);

    Ok(())
}
