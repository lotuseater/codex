pub(crate) use anyhow::Context;
pub(crate) use anyhow::Result;
pub(crate) use codex_config::types::ApprovalsReviewer;
pub(crate) use codex_core::config::Constrained;
pub(crate) use codex_core_test_runtime::PathBufExt;
pub(crate) use codex_core_test_runtime::PathExt;
pub(crate) use codex_core_test_runtime::get_remote_test_env;
pub(crate) use codex_core_test_runtime::responses::ev_apply_patch_custom_tool_call;
pub(crate) use codex_core_test_runtime::responses::ev_assistant_message;
pub(crate) use codex_core_test_runtime::responses::ev_completed;
pub(crate) use codex_core_test_runtime::responses::ev_function_call;
pub(crate) use codex_core_test_runtime::responses::ev_response_created;
pub(crate) use codex_core_test_runtime::responses::mount_sse_sequence;
pub(crate) use codex_core_test_runtime::responses::sse;
pub(crate) use codex_core_test_runtime::responses::start_mock_server;
pub(crate) use codex_core_test_runtime::skip_if_no_network;
pub(crate) use codex_core_test_runtime::test_codex::TestCodex;
pub(crate) use codex_core_test_runtime::test_codex::test_codex;
pub(crate) use codex_core_test_runtime::test_codex::test_env;
pub(crate) use codex_core_test_runtime::wait_for_event;
pub(crate) use codex_exec_server::CopyOptions;
pub(crate) use codex_exec_server::CreateDirectoryOptions;
pub(crate) use codex_exec_server::FileSystemSandboxContext;
pub(crate) use codex_exec_server::LOCAL_ENVIRONMENT_ID;
pub(crate) use codex_exec_server::REMOTE_ENVIRONMENT_ID;
pub(crate) use codex_exec_server::RemoveOptions;
pub(crate) use codex_features::Feature;
pub(crate) use codex_protocol::models::PermissionProfile;
pub(crate) use codex_protocol::permissions::FileSystemAccessMode;
pub(crate) use codex_protocol::permissions::FileSystemPath;
pub(crate) use codex_protocol::permissions::FileSystemSandboxEntry;
pub(crate) use codex_protocol::permissions::FileSystemSandboxPolicy;
pub(crate) use codex_protocol::permissions::NetworkSandboxPolicy;
pub(crate) use codex_protocol::protocol::ApplyPatchApprovalRequestEvent;
pub(crate) use codex_protocol::protocol::AskForApproval;
pub(crate) use codex_protocol::protocol::EventMsg;
pub(crate) use codex_protocol::protocol::Op;
pub(crate) use codex_protocol::protocol::ReviewDecision;
pub(crate) use codex_protocol::protocol::SandboxPolicy;
pub(crate) use codex_protocol::protocol::TurnEnvironmentSelection;
pub(crate) use codex_protocol::user_input::UserInput;
pub(crate) use codex_utils_absolute_path::AbsolutePathBuf;
pub(crate) use pretty_assertions::assert_eq;
pub(crate) use serde_json::Value;
pub(crate) use serde_json::json;
pub(crate) use std::fs;
pub(crate) use std::path::PathBuf;
pub(crate) use std::process::Command;
pub(crate) use std::time::SystemTime;
pub(crate) use std::time::UNIX_EPOCH;
pub(crate) use tempfile::TempDir;

pub(crate) async fn unified_exec_test(server: &wiremock::MockServer) -> Result<TestCodex> {
    let mut builder = test_codex().with_config(|config| {
        config.use_experimental_unified_exec_tool = true;
        let result = config.features.enable(Feature::UnifiedExec);
        assert!(
            result.is_ok(),
            "unified exec should enable for test: {result:?}",
        );
    });
    builder.build_remote_aware(server).await
}

pub(crate) async fn submit_turn_with_approval_and_environments(
    test: &TestCodex,
    prompt: &str,
    environments: Vec<TurnEnvironmentSelection>,
) -> Result<()> {
    test.codex
        .submit(Op::UserTurn {
            environments: Some(environments),
            items: vec![UserInput::Text {
                text: prompt.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            context_budget_mode: None,
            cwd: test.cwd.path().to_path_buf(),
            approval_policy: AskForApproval::OnRequest,
            approvals_reviewer: Some(ApprovalsReviewer::User),
            sandbox_policy: SandboxPolicy::new_workspace_write_policy(),
            permission_profile: None,
            model: test.session_configured.model.clone(),
            effort: None,
            summary: None,
            service_tier: None,
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    Ok(())
}

pub(crate) async fn expect_patch_approval(
    test: &TestCodex,
    expected_call_id: &str,
) -> ApplyPatchApprovalRequestEvent {
    let event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::ApplyPatchApprovalRequest(_) | EventMsg::TurnComplete(_)
        )
    })
    .await;

    match event {
        EventMsg::ApplyPatchApprovalRequest(approval) => {
            assert_eq!(approval.call_id, expected_call_id);
            approval
        }
        EventMsg::TurnComplete(_) => panic!("expected patch approval request before completion"),
        other => panic!("unexpected event: {other:?}"),
    }
}

pub(crate) async fn wait_for_completion_without_patch_approval(test: &TestCodex) {
    let event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::ApplyPatchApprovalRequest(_) | EventMsg::TurnComplete(_)
        )
    })
    .await;

    match event {
        EventMsg::TurnComplete(_) => {}
        EventMsg::ApplyPatchApprovalRequest(event) => {
            panic!("unexpected patch approval request: {:?}", event.call_id)
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

pub(crate) fn absolute_path(path: PathBuf) -> AbsolutePathBuf {
    match AbsolutePathBuf::try_from(path) {
        Ok(path) => path,
        Err(error) => panic!("path should be absolute: {error}"),
    }
}

pub(crate) fn read_only_sandbox(readable_root: PathBuf) -> FileSystemSandboxContext {
    let readable_root = absolute_path(readable_root);
    FileSystemSandboxContext::from_permission_profile(PermissionProfile::from_runtime_permissions(
        &FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
            path: FileSystemPath::Path {
                path: readable_root,
            },
            access: FileSystemAccessMode::Read,
        }]),
        NetworkSandboxPolicy::Restricted,
    ))
}

pub(crate) fn workspace_write_sandbox(writable_root: PathBuf) -> FileSystemSandboxContext {
    let writable_root = absolute_path(writable_root);
    FileSystemSandboxContext::from_permission_profile(PermissionProfile::from_runtime_permissions(
        &FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
            path: FileSystemPath::Path {
                path: writable_root,
            },
            access: FileSystemAccessMode::Write,
        }]),
        NetworkSandboxPolicy::Restricted,
    ))
}

pub(crate) fn assert_normalized_path_rejected(error: &std::io::Error) {
    match error.kind() {
        std::io::ErrorKind::NotFound => assert!(
            error.to_string().contains("No such file or directory"),
            "unexpected not-found message: {error}",
        ),
        std::io::ErrorKind::InvalidInput | std::io::ErrorKind::PermissionDenied => {
            let message = error.to_string();
            assert!(
                message.contains("is not permitted")
                    || message.contains("Operation not permitted")
                    || message.contains("Permission denied"),
                "unexpected rejection message: {message}",
            );
        }
        other => panic!("unexpected normalized-path error kind: {other:?}: {error:?}"),
    }
}

pub(crate) fn remote_exec(script: &str) -> Result<()> {
    let remote_env = get_remote_test_env().context("remote env should be configured")?;
    let output = Command::new("docker")
        .args(["exec", &remote_env.container_name, "sh", "-lc", script])
        .output()?;
    assert!(
        output.status.success(),
        "remote exec failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim(),
    );
    Ok(())
}

pub(crate) async fn exec_command_routing_output(
    test: &TestCodex,
    server: &wiremock::MockServer,
    call_id: &str,
    arguments: Value,
    environments: Option<Vec<TurnEnvironmentSelection>>,
) -> Result<String> {
    let response_mock = mount_sse_sequence(
        server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(call_id, "exec_command", &serde_json::to_string(&arguments)?),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    test.submit_turn_with_environments("route exec command", environments)
        .await?;

    response_mock
        .function_call_output_text(call_id)
        .with_context(|| format!("missing function_call_output for {call_id}"))
}

pub(crate) fn remote_test_file_path() -> PathBuf {
    let nanos = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    };
    PathBuf::from(format!(
        "/tmp/codex-remote-test-env-{}-{nanos}.txt",
        std::process::id()
    ))
}
