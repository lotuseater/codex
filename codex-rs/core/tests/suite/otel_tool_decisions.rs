use codex_core::config::Constrained;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_custom_tool_call;
use codex_core_test_runtime::responses::ev_function_call;
use codex_core_test_runtime::responses::ev_message_item_added;
use codex_core_test_runtime::responses::ev_output_text_delta;
use codex_core_test_runtime::responses::ev_reasoning_item;
use codex_core_test_runtime::responses::ev_reasoning_item_added;
use codex_core_test_runtime::responses::ev_reasoning_summary_text_delta;
use codex_core_test_runtime::responses::ev_reasoning_text_delta;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::mount_response_once;
use codex_core_test_runtime::responses::mount_sse_once;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::responses::sse_response;
use codex_core_test_runtime::responses::start_mock_server;
use codex_core_test_runtime::test_codex::TestCodex;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::wait_for_event;
use codex_features::Feature;
use codex_protocol::models::PermissionProfile;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::ReviewDecision;
use codex_protocol::user_input::UserInput;
use std::sync::Mutex;
use tracing::Level;
use tracing_test::traced_test;

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_test::internal::MockWriter;

fn extract_log_field(line: &str, key: &str) -> Option<String> {
    let quoted_prefix = format!("{key}=\"");
    if let Some(start) = line.find(&quoted_prefix) {
        let value_start = start + quoted_prefix.len();
        if let Some(end_rel) = line[value_start..].find('"') {
            return Some(line[value_start..value_start + end_rel].to_string());
        }
    }

    let bare_prefix = format!("{key}=");
    for token in line.split_whitespace() {
        let trimmed = token.trim_end_matches(',');
        if let Some(value) = trimmed.strip_prefix(&bare_prefix) {
            return Some(value.to_string());
        }
    }

    None
}

fn assert_empty_mcp_tool_fields(line: &str) -> Result<(), String> {
    let mcp_server = extract_log_field(line, "mcp_server")
        .ok_or_else(|| "missing mcp_server field".to_string())?;
    if !mcp_server.is_empty() {
        return Err(format!("expected empty mcp_server, got {mcp_server}"));
    }

    let mcp_server_origin = extract_log_field(line, "mcp_server_origin")
        .ok_or_else(|| "missing mcp_server_origin field".to_string())?;
    if !mcp_server_origin.is_empty() {
        return Err(format!(
            "expected empty mcp_server_origin, got {mcp_server_origin}"
        ));
    }

    Ok(())
}

fn shell_command_call(call_id: &str, command: &str) -> serde_json::Value {
    let args = serde_json::json!({ "command": command }).to_string();
    ev_function_call(call_id, "shell_command", &args)
}

fn touch_command(path: &str) -> String {
    if cfg!(windows) {
        format!("New-Item -ItemType File -Path {path} -Force | Out-Null")
    } else {
        format!("/usr/bin/touch {path}")
    }
}

#[tokio::test]
#[traced_test]
async fn handle_shell_command_autoapprove_from_config_records_tool_decision() {
    let server = start_mock_server().await;
    mount_sse_once(
        &server,
        sse(vec![
            shell_command_call("auto_config_call", "echo local shell"),
            ev_completed("done"),
        ]),
    )
    .await;

    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "shell command done"),
            ev_completed("done"),
        ]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config.permissions.approval_policy = Constrained::allow_any(AskForApproval::OnRequest);
            config
                .permissions
                .set_permission_profile(PermissionProfile::Disabled)
                .expect("set permission profile");
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    logs_assert(tool_decision_assertion(
        "auto_config_call",
        "approved",
        "config",
    ));
}
#[tokio::test]
#[traced_test]
async fn handle_shell_command_user_approved_records_tool_decision() {
    let server = start_mock_server().await;
    let command = touch_command("codex-otel-approval-test");
    mount_sse_once(
        &server,
        sse(vec![
            shell_command_call("user_approved_call", &command),
            ev_completed("done"),
        ]),
    )
    .await;

    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "shell command done"),
            ev_completed("done"),
        ]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config.permissions.approval_policy =
                Constrained::allow_any(AskForApproval::UnlessTrusted);
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "approved".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let approval_event =
        wait_for_event(&codex, |ev| matches!(ev, EventMsg::ExecApprovalRequest(_))).await;
    let EventMsg::ExecApprovalRequest(approval) = approval_event else {
        panic!("expected ExecApprovalRequest event");
    };

    codex
        .submit(Op::ExecApproval {
            id: approval.effective_approval_id(),
            turn_id: None,
            decision: ReviewDecision::Approved,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TokenCount(_))).await;

    logs_assert(tool_decision_assertion(
        "user_approved_call",
        "approved",
        "user",
    ));
}
#[tokio::test]
#[traced_test]
async fn handle_shell_command_user_approved_for_session_records_tool_decision() {
    let server = start_mock_server().await;
    let command = touch_command("codex-otel-approval-test");

    mount_sse_once(
        &server,
        sse(vec![
            shell_command_call("user_approved_session_call", &command),
            ev_completed("done"),
        ]),
    )
    .await;
    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "shell command done"),
            ev_completed("done"),
        ]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config.permissions.approval_policy =
                Constrained::allow_any(AskForApproval::UnlessTrusted);
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "persist".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let approval_event =
        wait_for_event(&codex, |ev| matches!(ev, EventMsg::ExecApprovalRequest(_))).await;
    let EventMsg::ExecApprovalRequest(approval) = approval_event else {
        panic!("expected ExecApprovalRequest event");
    };

    codex
        .submit(Op::ExecApproval {
            id: approval.effective_approval_id(),
            turn_id: None,
            decision: ReviewDecision::ApprovedForSession,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TokenCount(_))).await;

    logs_assert(tool_decision_assertion(
        "user_approved_session_call",
        "approvedforsession",
        "user",
    ));
}
#[tokio::test]
#[traced_test]
async fn handle_sandbox_error_user_approves_retry_records_tool_decision() {
    let server = start_mock_server().await;
    let command = touch_command("codex-otel-approval-test");

    mount_sse_once(
        &server,
        sse(vec![
            shell_command_call("sandbox_retry_call", &command),
            ev_completed("done"),
        ]),
    )
    .await;
    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "shell command done"),
            ev_completed("done"),
        ]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config.permissions.approval_policy =
                Constrained::allow_any(AskForApproval::UnlessTrusted);
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "retry".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let approval_event =
        wait_for_event(&codex, |ev| matches!(ev, EventMsg::ExecApprovalRequest(_))).await;
    let EventMsg::ExecApprovalRequest(approval) = approval_event else {
        panic!("expected ExecApprovalRequest event");
    };

    codex
        .submit(Op::ExecApproval {
            id: approval.effective_approval_id(),
            turn_id: None,
            decision: ReviewDecision::Approved,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TokenCount(_))).await;

    logs_assert(tool_decision_assertion(
        "sandbox_retry_call",
        "approved",
        "user",
    ));
}
#[tokio::test]
#[traced_test]
async fn handle_shell_command_user_denies_records_tool_decision() {
    let server = start_mock_server().await;
    let command = touch_command("codex-otel-approval-test");

    mount_sse_once(
        &server,
        sse(vec![
            shell_command_call("user_denied_call", &command),
            ev_completed("done"),
        ]),
    )
    .await;

    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "shell command done"),
            ev_completed("done"),
        ]),
    )
    .await;
    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config.permissions.approval_policy =
                Constrained::allow_any(AskForApproval::UnlessTrusted);
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "deny".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let approval_event =
        wait_for_event(&codex, |ev| matches!(ev, EventMsg::ExecApprovalRequest(_))).await;
    let EventMsg::ExecApprovalRequest(approval) = approval_event else {
        panic!("expected ExecApprovalRequest event");
    };

    codex
        .submit(Op::ExecApproval {
            id: approval.effective_approval_id(),
            turn_id: None,
            decision: ReviewDecision::Denied,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TokenCount(_))).await;

    logs_assert(tool_decision_assertion(
        "user_denied_call",
        "denied",
        "user",
    ));
}
#[tokio::test]
#[traced_test]
async fn handle_sandbox_error_user_approves_for_session_records_tool_decision() {
    let server = start_mock_server().await;
    let command = touch_command("codex-otel-approval-test");

    mount_sse_once(
        &server,
        sse(vec![
            shell_command_call("sandbox_session_call", &command),
            ev_completed("done"),
        ]),
    )
    .await;
    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "shell command done"),
            ev_completed("done"),
        ]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config.permissions.approval_policy =
                Constrained::allow_any(AskForApproval::UnlessTrusted);
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "persist".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let approval_event =
        wait_for_event(&codex, |ev| matches!(ev, EventMsg::ExecApprovalRequest(_))).await;
    let EventMsg::ExecApprovalRequest(approval) = approval_event else {
        panic!("expected ExecApprovalRequest event");
    };

    codex
        .submit(Op::ExecApproval {
            id: approval.effective_approval_id(),
            turn_id: None,
            decision: ReviewDecision::ApprovedForSession,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TokenCount(_))).await;

    logs_assert(tool_decision_assertion(
        "sandbox_session_call",
        "approvedforsession",
        "user",
    ));
}
#[tokio::test]
#[traced_test]
async fn handle_sandbox_error_user_denies_records_tool_decision() {
    let server = start_mock_server().await;
    let command = touch_command("codex-otel-approval-test");

    mount_sse_once(
        &server,
        sse(vec![
            shell_command_call("sandbox_deny_call", &command),
            ev_completed("done"),
        ]),
    )
    .await;

    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "shell command done"),
            ev_completed("done"),
        ]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config.permissions.approval_policy =
                Constrained::allow_any(AskForApproval::UnlessTrusted);
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "deny".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await
        .unwrap();

    let approval_event =
        wait_for_event(&codex, |ev| matches!(ev, EventMsg::ExecApprovalRequest(_))).await;
    let EventMsg::ExecApprovalRequest(approval) = approval_event else {
        panic!("expected ExecApprovalRequest event");
    };

    codex
        .submit(Op::ExecApproval {
            id: approval.effective_approval_id(),
            turn_id: None,
            decision: ReviewDecision::Denied,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TokenCount(_))).await;

    logs_assert(tool_decision_assertion(
        "sandbox_deny_call",
        "denied",
        "user",
    ));
}
