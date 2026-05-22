#![allow(clippy::unwrap_used, clippy::expect_used)]

pub(crate) use anyhow::Result;
pub(crate) use codex_core::config::Constrained;
pub(crate) use codex_core::sandboxing::SandboxPermissions;
pub(crate) use codex_core_test_runtime::responses::ev_assistant_message;
pub(crate) use codex_core_test_runtime::responses::ev_completed;
pub(crate) use codex_core_test_runtime::responses::ev_function_call;
pub(crate) use codex_core_test_runtime::responses::ev_response_created;
pub(crate) use codex_core_test_runtime::responses::mount_sse_once;
pub(crate) use codex_core_test_runtime::responses::mount_sse_sequence;
pub(crate) use codex_core_test_runtime::responses::sse;
pub(crate) use codex_core_test_runtime::responses::start_mock_server;
pub(crate) use codex_core_test_runtime::skip_if_no_network;
pub(crate) use codex_core_test_runtime::skip_if_sandbox;
pub(crate) use codex_core_test_runtime::test_codex::TestCodex;
pub(crate) use codex_core_test_runtime::test_codex::test_codex;
pub(crate) use codex_core_test_runtime::wait_for_event;
pub(crate) use codex_features::Feature;
pub(crate) use codex_protocol::config_types::ApprovalsReviewer;
pub(crate) use codex_protocol::models::AdditionalPermissionProfile as PermissionProfile;
pub(crate) use codex_protocol::models::FileSystemPermissions;
pub(crate) use codex_protocol::protocol::AskForApproval;
pub(crate) use codex_protocol::protocol::EventMsg;
pub(crate) use codex_protocol::protocol::ExecApprovalRequestEvent;
pub(crate) use codex_protocol::protocol::GranularApprovalConfig;
pub(crate) use codex_protocol::protocol::Op;
pub(crate) use codex_protocol::protocol::ReviewDecision;
pub(crate) use codex_protocol::protocol::SandboxPolicy;
pub(crate) use codex_protocol::request_permissions::PermissionGrantScope;
pub(crate) use codex_protocol::request_permissions::RequestPermissionProfile;
pub(crate) use codex_protocol::request_permissions::RequestPermissionsResponse;
pub(crate) use codex_protocol::user_input::UserInput;
pub(crate) use codex_utils_absolute_path::AbsolutePathBuf;
pub(crate) use pretty_assertions::assert_eq;
pub(crate) use regex_lite::Regex;
pub(crate) use serde_json::Value;
pub(crate) use serde_json::json;
pub(crate) use std::fs;
pub(crate) use std::path::Path;

pub(crate) fn absolute_path(path: &Path) -> AbsolutePathBuf {
    AbsolutePathBuf::try_from(path).expect("absolute path")
}

pub(crate) struct CommandResult {
    pub(crate) exit_code: Option<i64>,
    pub(crate) stdout: String,
}

pub(crate) fn parse_result(item: &Value) -> CommandResult {
    let output_str = item
        .get("output")
        .and_then(Value::as_str)
        .expect("shell output payload");
    match serde_json::from_str::<Value>(output_str) {
        Ok(parsed) => {
            let exit_code = parsed["metadata"]["exit_code"].as_i64();
            let stdout = parsed["output"].as_str().unwrap_or_default().to_string();
            CommandResult { exit_code, stdout }
        }
        Err(_) => {
            let structured = Regex::new(r"(?s)^Exit code:\s*(-?\d+).*?Output:\n(.*)$").unwrap();
            let regex =
                Regex::new(r"(?s)^.*?Process exited with code (\d+)\n.*?Output:\n(.*)$").unwrap();
            if let Some(captures) = structured.captures(output_str) {
                let exit_code = captures.get(1).unwrap().as_str().parse::<i64>().unwrap();
                let output = captures.get(2).unwrap().as_str();
                CommandResult {
                    exit_code: Some(exit_code),
                    stdout: output.to_string(),
                }
            } else if let Some(captures) = regex.captures(output_str) {
                let exit_code = captures.get(1).unwrap().as_str().parse::<i64>().unwrap();
                let output = captures.get(2).unwrap().as_str();
                CommandResult {
                    exit_code: Some(exit_code),
                    stdout: output.to_string(),
                }
            } else {
                CommandResult {
                    exit_code: None,
                    stdout: output_str.to_string(),
                }
            }
        }
    }
}

pub(crate) fn shell_event_with_request_permissions<S: serde::Serialize>(
    call_id: &str,
    command: &str,
    additional_permissions: &S,
) -> Result<Value> {
    let args = json!({
        "command": command,
        "timeout_ms": 1_000_u64,
        "sandbox_permissions": SandboxPermissions::WithAdditionalPermissions,
        "additional_permissions": additional_permissions,
    });
    let args_str = serde_json::to_string(&args)?;
    Ok(ev_function_call(call_id, "shell_command", &args_str))
}

pub(crate) fn request_permissions_tool_event(
    call_id: &str,
    reason: &str,
    permissions: &RequestPermissionProfile,
) -> Result<Value> {
    let args = json!({
        "reason": reason,
        "permissions": permissions,
    });
    let args_str = serde_json::to_string(&args)?;
    Ok(ev_function_call(call_id, "request_permissions", &args_str))
}

pub(crate) fn shell_command_event(call_id: &str, command: &str) -> Result<Value> {
    let args = json!({
        "command": command,
        "timeout_ms": 1_000_u64,
    });
    let args_str = serde_json::to_string(&args)?;
    Ok(ev_function_call(call_id, "shell_command", &args_str))
}

pub(crate) fn exec_command_event(call_id: &str, command: &str) -> Result<Value> {
    let args = json!({
        "cmd": command,
        "yield_time_ms": 1_000_u64,
    });
    let args_str = serde_json::to_string(&args)?;
    Ok(ev_function_call(call_id, "exec_command", &args_str))
}

pub(crate) fn exec_command_event_with_request_permissions<S: serde::Serialize>(
    call_id: &str,
    command: &str,
    additional_permissions: &S,
) -> Result<Value> {
    let args = json!({
        "cmd": command,
        "yield_time_ms": 1_000_u64,
        "sandbox_permissions": SandboxPermissions::WithAdditionalPermissions,
        "additional_permissions": additional_permissions,
    });
    let args_str = serde_json::to_string(&args)?;
    Ok(ev_function_call(call_id, "exec_command", &args_str))
}

pub(crate) fn exec_command_event_with_missing_additional_permissions(
    call_id: &str,
    command: &str,
) -> Result<Value> {
    let args = json!({
        "cmd": command,
        "yield_time_ms": 1_000_u64,
        "sandbox_permissions": SandboxPermissions::WithAdditionalPermissions,
    });
    let args_str = serde_json::to_string(&args)?;
    Ok(ev_function_call(call_id, "exec_command", &args_str))
}

pub(crate) fn shell_event_with_raw_request_permissions(
    call_id: &str,
    command: &str,
    workdir: Option<&str>,
    additional_permissions: Value,
) -> Result<Value> {
    let args = json!({
        "command": command,
        "workdir": workdir,
        "timeout_ms": 1_000_u64,
        "sandbox_permissions": SandboxPermissions::WithAdditionalPermissions,
        "additional_permissions": additional_permissions,
    });
    let args_str = serde_json::to_string(&args)?;
    Ok(ev_function_call(call_id, "shell_command", &args_str))
}

pub(crate) async fn submit_turn(
    test: &TestCodex,
    prompt: &str,
    approval_policy: AskForApproval,
    sandbox_policy: SandboxPolicy,
) -> Result<()> {
    let session_model = test.session_configured.model.clone();
    test.codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: prompt.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: test.cwd.path().to_path_buf(),
            approval_policy,
            approvals_reviewer: Some(ApprovalsReviewer::User),
            sandbox_policy,
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
    Ok(())
}

pub(crate) async fn wait_for_completion(test: &TestCodex) {
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
}

pub(crate) async fn expect_exec_approval(
    test: &TestCodex,
    expected_command: &str,
) -> ExecApprovalRequestEvent {
    let event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::ExecApprovalRequest(_) | EventMsg::TurnComplete(_)
        )
    })
    .await;

    match event {
        EventMsg::ExecApprovalRequest(approval) => {
            let last_arg = approval
                .command
                .last()
                .map(String::as_str)
                .unwrap_or_default();
            assert_eq!(last_arg, expected_command);
            approval
        }
        EventMsg::TurnComplete(_) => panic!("expected approval request before completion"),
        other => panic!("unexpected event: {other:?}"),
    }
}

pub(crate) async fn wait_for_exec_approval_or_completion(
    test: &TestCodex,
) -> Option<ExecApprovalRequestEvent> {
    let event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::ExecApprovalRequest(_) | EventMsg::TurnComplete(_)
        )
    })
    .await;

    match event {
        EventMsg::ExecApprovalRequest(approval) => Some(approval),
        EventMsg::TurnComplete(_) => None,
        other => panic!("unexpected event: {other:?}"),
    }
}

pub(crate) async fn expect_request_permissions_event(
    test: &TestCodex,
    expected_call_id: &str,
) -> RequestPermissionProfile {
    let event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::RequestPermissions(_) | EventMsg::TurnComplete(_)
        )
    })
    .await;

    match event {
        EventMsg::RequestPermissions(request) => {
            assert_eq!(request.call_id, expected_call_id);
            request.permissions
        }
        EventMsg::TurnComplete(_) => panic!("expected request_permissions before completion"),
        other => panic!("unexpected event: {other:?}"),
    }
}

pub(crate) fn workspace_write_excluding_tmp() -> SandboxPolicy {
    SandboxPolicy::WorkspaceWrite {
        writable_roots: vec![],
        network_access: false,
        exclude_tmpdir_env_var: true,
        exclude_slash_tmp: true,
    }
}

pub(crate) fn requested_directory_write_permissions(path: &Path) -> RequestPermissionProfile {
    RequestPermissionProfile {
        file_system: Some(FileSystemPermissions::from_read_write_roots(
            Some(vec![]),
            Some(vec![absolute_path(path)]),
        )),
        ..RequestPermissionProfile::default()
    }
}

pub(crate) fn normalized_directory_write_permissions(
    path: &Path,
) -> Result<RequestPermissionProfile> {
    Ok(RequestPermissionProfile {
        file_system: Some(FileSystemPermissions::from_read_write_roots(
            Some(vec![]),
            Some(vec![AbsolutePathBuf::try_from(path.canonicalize()?)?]),
        )),
        ..RequestPermissionProfile::default()
    })
}
