#![allow(clippy::unwrap_used)]

use codex_apply_patch::APPLY_PATCH_TOOL_INSTRUCTIONS;
use codex_core::shell::default_user_shell;
use codex_features::Feature;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::config_types::Settings;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::models::PermissionProfile;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::permissions::NetworkSandboxPolicy;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::ENVIRONMENT_CONTEXT_OPEN_TAG;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use codex_core_test_runtime::TempDirExt;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::mount_sse_once;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::responses::start_mock_server;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::test_codex::TestCodex;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::test_codex::turn_permission_fields;
use codex_core_test_runtime::wait_for_event;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

fn text_user_input(text: String) -> serde_json::Value {
    text_user_input_parts(vec![text])
}

fn text_user_input_parts(texts: Vec<String>) -> serde_json::Value {
    serde_json::json!({
        "type": "message",
        "role": "user",
        "content": texts
            .into_iter()
            .map(|text| serde_json::json!({ "type": "input_text", "text": text }))
            .collect::<Vec<_>>()
    })
}

fn assert_default_env_context(text: &str, cwd: &str) {
    assert!(
        text.starts_with(ENVIRONMENT_CONTEXT_OPEN_TAG),
        "expected environment context fragment: {text}"
    );
    assert!(
        text.contains(&format!("<cwd>{cwd}</cwd>")),
        "expected cwd in environment context: {text}"
    );
    assert!(
        text.contains(&format!("<shell>{}</shell>", default_user_shell().name())),
        "expected shell in environment context: {text}"
    );
    assert!(
        text.contains("<current_date>") && text.contains("</current_date>"),
        "expected current_date in environment context: {text}"
    );
    assert!(
        text.contains("<timezone>") && text.contains("</timezone>"),
        "expected timezone in environment context: {text}"
    );
    assert!(
        text.ends_with("</environment_context>"),
        "expected closing environment_context tag: {text}"
    );
}

fn assert_tool_names(body: &serde_json::Value, expected_names: &[&str]) {
    assert_eq!(
        body["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                t.get("name")
                    .and_then(|value| value.as_str())
                    .or_else(|| t.get("type").and_then(|value| value.as_str()))
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<_>>(),
        expected_names
    );
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn send_user_turn_with_no_changes_does_not_send_environment_context() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    use pretty_assertions::assert_eq;

    let server = start_mock_server().await;
    let req1 = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let req2 = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-2"), ev_completed("resp-2")]),
    )
    .await;

    let TestCodex {
        codex,
        config,
        session_configured,
        ..
    } = test_codex()
        .with_config(|config| {
            config.user_instructions = Some("be consistent and helpful".to_string());
            config
                .features
                .enable(Feature::CollaborationModes)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await?;

    let default_cwd = config.cwd.clone();
    let default_approval_policy = config.permissions.approval_policy.value();
    let default_sandbox_policy = &config.legacy_sandbox_policy();
    let default_model = session_configured.model;
    let default_effort = config.model_reasoning_effort;
    let default_summary = config.model_reasoning_summary;

    codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello 1".into(),
                text_elements: Vec::new(),
            }],
            cwd: default_cwd.to_path_buf(),
            approval_policy: default_approval_policy,
            approvals_reviewer: None,
            sandbox_policy: default_sandbox_policy.clone(),
            permission_profile: None,
            model: default_model.clone(),
            effort: default_effort,
            summary: Some(default_summary.unwrap_or(ReasoningSummary::Auto)),
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            final_output_json_schema: None,
            personality: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello 2".into(),
                text_elements: Vec::new(),
            }],
            cwd: default_cwd.to_path_buf(),
            approval_policy: default_approval_policy,
            approvals_reviewer: None,
            sandbox_policy: default_sandbox_policy.clone(),
            permission_profile: None,
            model: default_model.clone(),
            effort: default_effort,
            summary: Some(default_summary.unwrap_or(ReasoningSummary::Auto)),
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            final_output_json_schema: None,
            personality: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let request1 = req1.single_request();
    let request2 = req2.single_request();
    let body1 = request1.body_json();
    let body2 = request2.body_json();

    let expected_permissions_msg = body1["input"][0].clone();
    let expected_ui_msg = body1["input"][1].clone();

    let default_cwd_lossy = default_cwd.to_string_lossy();
    let expected_env_text_1 = expected_ui_msg["content"][1]["text"]
        .as_str()
        .expect("cached environment context text")
        .to_string();
    assert_default_env_context(&expected_env_text_1, &default_cwd_lossy);

    let expected_contextual_user_msg_1 = text_user_input_parts(vec![
        expected_ui_msg["content"][0]["text"]
            .as_str()
            .expect("cached user instructions text")
            .to_string(),
        expected_env_text_1,
    ]);
    let expected_user_message_1 = text_user_input("hello 1".to_string());

    let expected_input_1 = serde_json::Value::Array(vec![
        expected_permissions_msg.clone(),
        expected_contextual_user_msg_1.clone(),
        expected_user_message_1.clone(),
    ]);
    assert_eq!(body1["input"], expected_input_1);

    let expected_user_message_2 = text_user_input("hello 2".to_string());
    let expected_input_2 = serde_json::Value::Array(vec![
        expected_permissions_msg,
        expected_contextual_user_msg_1,
        expected_user_message_1,
        expected_user_message_2,
    ]);
    assert_eq!(body2["input"], expected_input_2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_user_turn_with_changes_sends_environment_context() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    use pretty_assertions::assert_eq;

    let server = start_mock_server().await;

    let req1 = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let req2 = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-2"), ev_completed("resp-2")]),
    )
    .await;
    let TestCodex {
        codex,
        config,
        session_configured,
        ..
    } = test_codex()
        .with_config(|config| {
            config.user_instructions = Some("be consistent and helpful".to_string());
            config
                .features
                .enable(Feature::CollaborationModes)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await?;

    let default_cwd = config.cwd.clone();
    let default_approval_policy = config.permissions.approval_policy.value();
    let default_sandbox_policy = &config.legacy_sandbox_policy();
    let default_model = session_configured.model;
    let default_effort = config.model_reasoning_effort;
    let default_summary = config.model_reasoning_summary;

    codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello 1".into(),
                text_elements: Vec::new(),
            }],
            cwd: default_cwd.to_path_buf(),
            approval_policy: default_approval_policy,
            approvals_reviewer: None,
            sandbox_policy: default_sandbox_policy.clone(),
            permission_profile: None,
            model: default_model,
            effort: default_effort,
            summary: Some(default_summary.unwrap_or(ReasoningSummary::Auto)),
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            final_output_json_schema: None,
            personality: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::Disabled, default_cwd.as_path());
    codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello 2".into(),
                text_elements: Vec::new(),
            }],
            cwd: default_cwd.to_path_buf(),
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy,
            permission_profile,
            model: "o3".to_string(),
            effort: Some(ReasoningEffort::High),
            summary: Some(ReasoningSummary::Detailed),
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            final_output_json_schema: None,
            personality: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let request1 = req1.single_request();
    let request2 = req2.single_request();
    let body1 = request1.body_json();
    let body2 = request2.body_json();

    let expected_permissions_msg = body1["input"][0].clone();
    let expected_ui_msg = body1["input"][1].clone();

    let expected_env_text_1 = expected_ui_msg["content"][1]["text"]
        .as_str()
        .expect("cached environment context text")
        .to_string();
    assert_default_env_context(&expected_env_text_1, &default_cwd.to_string_lossy());
    let expected_contextual_user_msg_1 = text_user_input_parts(vec![
        expected_ui_msg["content"][0]["text"]
            .as_str()
            .expect("cached user instructions text")
            .to_string(),
        expected_env_text_1,
    ]);
    let expected_user_message_1 = text_user_input("hello 1".to_string());
    let expected_input_1 = serde_json::Value::Array(vec![
        expected_permissions_msg.clone(),
        expected_contextual_user_msg_1.clone(),
        expected_user_message_1.clone(),
    ]);
    assert_eq!(body1["input"], expected_input_1);

    let body1_input = body1["input"].as_array().expect("input array");
    let expected_settings_update_msg = body2["input"][body1_input.len()].clone();
    assert_ne!(
        expected_settings_update_msg, expected_permissions_msg,
        "expected updated permissions message after policy change"
    );
    assert_eq!(
        expected_settings_update_msg["role"].as_str(),
        Some("developer")
    );
    assert!(
        request2.has_message_with_input_texts("developer", |texts| {
            texts.iter().any(|text| text.contains("<model_switch>"))
        }),
        "expected model switch section after model override: {expected_settings_update_msg:?}"
    );
    let expected_user_message_2 = text_user_input("hello 2".to_string());
    let expected_input_2 = serde_json::Value::Array(vec![
        expected_permissions_msg,
        expected_contextual_user_msg_1,
        expected_user_message_1,
        expected_settings_update_msg,
        expected_user_message_2,
    ]);
    assert_eq!(body2["input"], expected_input_2);

    Ok(())
}
