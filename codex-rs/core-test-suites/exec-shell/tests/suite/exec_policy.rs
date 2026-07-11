#![allow(clippy::unwrap_used)]

use anyhow::Result;
use codex_config::test_support::CloudConfigBundleFixture;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_function_call;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::mount_sse_once;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::responses::start_mock_server;
use codex_core_test_runtime::skip_if_target_windows;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::test_codex::turn_permission_fields;
use codex_core_test_runtime::wait_for_event;
use codex_features::Feature;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::Settings;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use serde_json::Value;
use serde_json::json;
use std::fs;

fn collaboration_mode_for_model(model: String) -> CollaborationMode {
    CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort: None,
            developer_instructions: Some("exercise approvals in collaboration mode".to_string()),
        },
    }
}

async fn submit_user_turn(
    test: &codex_core_test_runtime::test_codex::TestCodex,
    prompt: &str,
    approval_policy: AskForApproval,
    permission_profile: PermissionProfile,
    collaboration_mode: Option<CollaborationMode>,
) -> Result<()> {
    let session_model = test.session_configured.model.clone();
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(permission_profile, test.config.cwd.as_path());
    test.codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: prompt.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: test.cwd_path().to_path_buf(),
            approval_policy,
            approvals_reviewer: None,
            sandbox_policy,
            permission_profile,
            model: session_model,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode,
            personality: None,
        })
        .await?;
    Ok(())
}

fn assert_no_matched_rules_invariant(output_item: &Value) {
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("function call output should include a string output payload");
    assert!(
        !output.contains("invariant failed: matched_rules must be non-empty"),
        "unexpected invariant panic surfaced in output: {output}"
    );
}

#[cfg(windows)]
#[tokio::test]
async fn unified_exec_disabled_windows_sandbox_rejects_managed_read_only_command() -> Result<()> {
    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
        config
            .features
            .disable(Feature::WindowsSandbox)
            .expect("test config should allow feature update");
        config
            .features
            .disable(Feature::WindowsSandboxElevated)
            .expect("test config should allow feature update");
        config.set_windows_sandbox_enabled(false);
        config.set_windows_elevated_sandbox_enabled(false);
    });
    let test = builder.build(&server).await?;
    let call_id = "unified-exec-disabled-windows-sandbox-read-only";
    let args = json!({
        "cmd": "cmd.exe /c dir",
        "yield_time_ms": 1_000,
    });

    mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-disabled-windows-sandbox-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-disabled-windows-sandbox-1"),
        ]),
    )
    .await;
    let results_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-disabled-windows-sandbox-1", "done"),
            ev_completed("resp-disabled-windows-sandbox-2"),
        ]),
    )
    .await;

    submit_user_turn(
        &test,
        "run unified exec with disabled Windows sandbox",
        AskForApproval::Never,
        PermissionProfile::read_only(),
        None,
    )
    .await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let output_item = results_mock.single_request().function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("function call output should include a string output payload");
    assert!(
        output.contains("cmd.exe /c dir") && output.contains("rejected: blocked by policy"),
        "unexpected output: {output}",
    );

    Ok(())
}

#[tokio::test]
async fn execpolicy_blocks_shell_invocation() -> Result<()> {
    let mut builder = test_codex().with_config(|config| {
        let policy_path = config.codex_home.join("rules").join("policy.rules");
        fs::create_dir_all(
            policy_path
                .parent()
                .expect("policy directory must have a parent"),
        )
        .expect("create policy directory");
        fs::write(
            &policy_path,
            r#"prefix_rule(pattern=["echo"], decision="forbidden")"#,
        )
        .expect("write policy file");
    });
    let server = start_mock_server().await;
    let test = builder.build(&server).await?;

    let call_id = "shell-forbidden";
    let args = json!({
        "command": "echo blocked",
        "timeout_ms": 1_000,
    });

    mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "shell_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    let session_model = test.session_configured.model.clone();
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::Disabled, test.config.cwd.as_path());
    test.codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "run shell command".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: test.cwd_path().to_path_buf(),
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy,
            permission_profile,
            model: session_model,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    let EventMsg::ExecCommandEnd(end) = wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::ExecCommandEnd(_))
    })
    .await
    else {
        unreachable!()
    };
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    assert!(
        end.aggregated_output
            .contains("policy forbids commands starting with `echo`"),
        "unexpected output: {}",
        end.aggregated_output
    );

    Ok(())
}

#[tokio::test]
async fn malformed_custom_rules_preserve_managed_forbidden_prefix() -> Result<()> {
    skip_if_target_windows!(
        Ok(()),
        "managed prefix fixture uses POSIX executable semantics"
    );

    let mut builder = test_codex()
        .with_cloud_config_bundle(
            CloudConfigBundleFixture::loader_with_enterprise_requirement(
                r#"
[rules]
prefix_rules = [
    { pattern = [{ token = "echo" }], decision = "forbidden" },
]
"#,
            ),
        )
        .with_config(|config| {
            config
                .features
                .enable(Feature::UnifiedExec)
                .expect("test config should allow feature update");
            let policy_path = config.codex_home.join("rules").join("broken.rules");
            fs::create_dir_all(
                policy_path
                    .parent()
                    .expect("policy directory must have a parent"),
            )
            .expect("create policy directory");
            fs::write(policy_path, "prefix_rule(").expect("write malformed policy file");
        });
    let server = start_mock_server().await;
    let test = builder.build_with_auto_env(&server).await?;
    let call_id = "managed-shell-forbidden";
    let args = json!({
        "cmd": "echo blocked",
        "yield_time_ms": 1_000,
    });

    mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-managed-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-managed-1"),
        ]),
    )
    .await;
    let results_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-managed-1", "done"),
            ev_completed("resp-managed-2"),
        ]),
    )
    .await;

    test.submit_turn_with_approval_and_permission_profile(
        "run shell command",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let output_item = results_mock.single_request().function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("function call output should include a string output payload");
    assert!(
        output.contains("policy forbids commands starting with `echo`"),
        "unexpected output: {output}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shell_command_empty_script_with_collaboration_mode_does_not_panic() -> Result<()> {
    let server = start_mock_server().await;
    let mut builder = test_codex().with_model("gpt-5.2").with_config(|config| {
        config
            .features
            .enable(Feature::CollaborationModes)
            .expect("test config should allow feature update");
    });
    let test = builder.build(&server).await?;
    let call_id = "shell-empty-script-collab";
    let args = json!({
        "command": "",
        "timeout_ms": 1_000,
    });

    mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-empty-shell-1"),
            ev_function_call(call_id, "shell_command", &serde_json::to_string(&args)?),
            ev_completed("resp-empty-shell-1"),
        ]),
    )
    .await;
    let results_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-empty-shell-1", "done"),
            ev_completed("resp-empty-shell-2"),
        ]),
    )
    .await;

    let collaboration_mode = collaboration_mode_for_model(test.session_configured.model.clone());
    submit_user_turn(
        &test,
        "run an empty shell command",
        AskForApproval::OnRequest,
        PermissionProfile::Disabled,
        Some(collaboration_mode),
    )
    .await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let output_item = results_mock.single_request().function_call_output(call_id);
    assert_no_matched_rules_invariant(&output_item);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_empty_script_with_collaboration_mode_does_not_panic() -> Result<()> {
    let server = start_mock_server().await;
    let mut builder = test_codex().with_model("gpt-5.2").with_config(|config| {
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
        config
            .features
            .enable(Feature::CollaborationModes)
            .expect("test config should allow feature update");
    });
    let test = builder.build(&server).await?;
    let call_id = "unified-exec-empty-script-collab";
    let args = json!({
        "cmd": "",
        "yield_time_ms": 1_000,
    });

    mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-empty-unified-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-empty-unified-1"),
        ]),
    )
    .await;
    let results_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-empty-unified-1", "done"),
            ev_completed("resp-empty-unified-2"),
        ]),
    )
    .await;

    let collaboration_mode = collaboration_mode_for_model(test.session_configured.model.clone());
    submit_user_turn(
        &test,
        "run empty unified exec command",
        AskForApproval::OnRequest,
        PermissionProfile::Disabled,
        Some(collaboration_mode),
    )
    .await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let output_item = results_mock.single_request().function_call_output(call_id);
    assert_no_matched_rules_invariant(&output_item);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shell_command_whitespace_script_with_collaboration_mode_does_not_panic() -> Result<()> {
    let server = start_mock_server().await;
    let mut builder = test_codex().with_model("gpt-5.2").with_config(|config| {
        config
            .features
            .enable(Feature::CollaborationModes)
            .expect("test config should allow feature update");
    });
    let test = builder.build(&server).await?;
    let call_id = "shell-whitespace-script-collab";
    let args = json!({
        "command": "  \n\t  ",
        "timeout_ms": 1_000,
    });

    mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-whitespace-shell-1"),
            ev_function_call(call_id, "shell_command", &serde_json::to_string(&args)?),
            ev_completed("resp-whitespace-shell-1"),
        ]),
    )
    .await;
    let results_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-whitespace-shell-1", "done"),
            ev_completed("resp-whitespace-shell-2"),
        ]),
    )
    .await;

    let collaboration_mode = collaboration_mode_for_model(test.session_configured.model.clone());
    submit_user_turn(
        &test,
        "run whitespace shell command",
        AskForApproval::OnRequest,
        PermissionProfile::Disabled,
        Some(collaboration_mode),
    )
    .await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let output_item = results_mock.single_request().function_call_output(call_id);
    assert_no_matched_rules_invariant(&output_item);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unified_exec_whitespace_script_with_collaboration_mode_does_not_panic() -> Result<()> {
    let server = start_mock_server().await;
    let mut builder = test_codex().with_model("gpt-5.2").with_config(|config| {
        config
            .features
            .enable(Feature::UnifiedExec)
            .expect("test config should allow feature update");
        config
            .features
            .enable(Feature::CollaborationModes)
            .expect("test config should allow feature update");
    });
    let test = builder.build(&server).await?;
    let call_id = "unified-exec-whitespace-script-collab";
    let args = json!({
        "cmd": " \n \t",
        "yield_time_ms": 1_000,
    });

    mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-whitespace-unified-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-whitespace-unified-1"),
        ]),
    )
    .await;
    let results_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-whitespace-unified-1", "done"),
            ev_completed("resp-whitespace-unified-2"),
        ]),
    )
    .await;

    let collaboration_mode = collaboration_mode_for_model(test.session_configured.model.clone());
    submit_user_turn(
        &test,
        "run whitespace unified exec command",
        AskForApproval::OnRequest,
        PermissionProfile::Disabled,
        Some(collaboration_mode),
    )
    .await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let output_item = results_mock.single_request().function_call_output(call_id);
    assert_no_matched_rules_invariant(&output_item);

    Ok(())
}
