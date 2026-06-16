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
async fn override_before_first_turn_emits_environment_context() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let req = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex().build(&server).await?;

    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model: "gpt-5.4".to_string(),
            reasoning_effort: Some(ReasoningEffort::High),
            developer_instructions: None,
        },
    };

    codex
        .submit(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: Some(AskForApproval::Never),
            approvals_reviewer: None,
            sandbox_policy: None,
            permission_profile: None,
            windows_sandbox_level: None,
            model: Some("gpt-5.4".to_string()),
            effort: Some(Some(ReasoningEffort::Low)),
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: Some(collaboration_mode),
            personality: None,
        })
        .await?;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "first message".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await?;

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let body = req.single_request().body_json();
    assert_eq!(body["model"].as_str(), Some("gpt-5.4"));
    assert_eq!(
        body.get("reasoning")
            .and_then(|reasoning| reasoning.get("effort"))
            .and_then(|value| value.as_str()),
        Some("high")
    );
    let input = body["input"]
        .as_array()
        .expect("input array must be present");
    assert!(
        !input.is_empty(),
        "expected at least environment context and user message"
    );

    let env_texts: Vec<&str> = input
        .iter()
        .filter_map(|msg| {
            msg["content"].as_array().map(|content| {
                content
                    .iter()
                    .filter_map(|item| item["text"].as_str())
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .filter(|text| text.starts_with(ENVIRONMENT_CONTEXT_OPEN_TAG))
        .collect();
    assert!(
        !env_texts.is_empty(),
        "expected environment context to be emitted: {env_texts:?}"
    );
    assert!(
        env_texts
            .iter()
            .any(|text| text.contains("<current_date>") && text.contains("</current_date>")),
        "expected current_date in environment context: {env_texts:?}"
    );
    assert!(
        env_texts
            .iter()
            .any(|text| text.contains("<timezone>") && text.contains("</timezone>")),
        "expected timezone in environment context: {env_texts:?}"
    );

    let env_count = input
        .iter()
        .filter(|msg| {
            msg["content"]
                .as_array()
                .and_then(|content| {
                    content.iter().find(|item| {
                        item["type"].as_str() == Some("input_text")
                            && item["text"]
                                .as_str()
                                .map(|text| text.starts_with(ENVIRONMENT_CONTEXT_OPEN_TAG))
                                .unwrap_or(false)
                    })
                })
                .is_some()
        })
        .count();
    assert!(
        env_count >= 1,
        "environment context should appear at least once, found {env_count}"
    );

    let permissions_texts: Vec<&str> = input
        .iter()
        .filter_map(|msg| {
            let role = msg["role"].as_str()?;
            if role != "developer" {
                return None;
            }
            msg["content"]
                .as_array()
                .and_then(|content| content.first())
                .and_then(|item| item["text"].as_str())
        })
        .collect();
    assert!(
        permissions_texts.iter().any(|text| {
            let lower = text.to_ascii_lowercase();
            (lower.contains("approval policy") || lower.contains("approval_policy"))
                && lower.contains("never")
        }),
        "permissions message should reflect overridden approval policy: {permissions_texts:?}"
    );

    let user_texts: Vec<&str> = input
        .iter()
        .filter_map(|msg| {
            msg["content"].as_array().map(|content| {
                content
                    .iter()
                    .filter_map(|item| item["text"].as_str())
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .collect();
    assert!(
        user_texts.contains(&"first message"),
        "expected user message text, got {user_texts:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn per_turn_overrides_keep_cached_prefix_and_key_constant() -> anyhow::Result<()> {
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

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config.user_instructions = Some("be consistent and helpful".to_string());
            config
                .features
                .enable(Feature::CollaborationModes)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await?;

    // First turn
    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello 1".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    // Second turn using per-turn overrides via UserTurn
    let new_cwd = TempDir::new().unwrap();
    let writable = TempDir::new().unwrap();
    let permission_profile = PermissionProfile::workspace_write_with(
        &[writable.abs()],
        NetworkSandboxPolicy::Enabled,
        /*exclude_tmpdir_env_var*/ true,
        /*exclude_slash_tmp*/ true,
    );
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(permission_profile, new_cwd.path());
    codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello 2".into(),
                text_elements: Vec::new(),
            }],
            cwd: new_cwd.path().to_path_buf(),
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

    // prompt_cache_key should remain constant across per-turn overrides
    assert_eq!(
        body1["prompt_cache_key"], body2["prompt_cache_key"],
        "prompt_cache_key should not change across per-turn overrides"
    );

    // The entire prefix from the first request should be identical and reused
    // as the prefix of the second request.
    let expected_user_message_2 = serde_json::json!({
        "type": "message",
        "role": "user",
        "content": [ { "type": "input_text", "text": "hello 2" } ]
    });
    let expected_permissions_msg = body1["input"][0].clone();
    let body1_input = body1["input"].as_array().expect("input array");
    let expected_settings_update_msg = body2["input"][body1_input.len()].clone();
    assert_ne!(
        expected_settings_update_msg, expected_permissions_msg,
        "expected updated permissions message after per-turn override"
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
    let expected_env_msg_2 = body2["input"][body1_input.len() + 1].clone();
    assert_eq!(expected_env_msg_2["role"].as_str(), Some("user"));
    let env_text = expected_env_msg_2["content"][0]["text"]
        .as_str()
        .expect("environment context text");
    let expected_cwd = new_cwd.path().display().to_string();
    assert_default_env_context(env_text, &expected_cwd);
    let mut expected_body2 = body1_input.to_vec();
    expected_body2.push(expected_settings_update_msg);
    expected_body2.push(expected_env_msg_2);
    expected_body2.push(expected_user_message_2);
    assert_eq!(body2["input"], serde_json::Value::Array(expected_body2));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
