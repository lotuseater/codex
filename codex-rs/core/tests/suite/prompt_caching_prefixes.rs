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
async fn prefixes_context_and_instructions_once_and_consistently_across_requests()
-> anyhow::Result<()> {
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

    let TestCodex { codex, config, .. } = test_codex()
        .with_config(|config| {
            config.user_instructions = Some("be consistent and helpful".to_string());
            config
                .features
                .enable(Feature::CollaborationModes)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await?;

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

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello 2".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let body1 = req1.single_request().body_json();
    let input1 = body1["input"].as_array().expect("input array");
    assert_eq!(
        input1.len(),
        3,
        "expected permissions + cached contextual user prefix + user msg"
    );

    let ui_text = input1[1]["content"][0]["text"]
        .as_str()
        .expect("ui message text");
    assert!(
        ui_text.contains("be consistent and helpful"),
        "expected user instructions in UI message: {ui_text}"
    );

    let cwd_str = config.cwd.to_string_lossy();
    let env_text = input1[1]["content"][1]["text"]
        .as_str()
        .expect("environment context text");
    assert_default_env_context(env_text, &cwd_str);
    assert_eq!(
        input1[1]["content"][1]["type"].as_str(),
        Some("input_text"),
        "expected environment context bundled after UI message in cached contextual message"
    );
    assert_eq!(input1[2], text_user_input("hello 1".to_string()));

    let body2 = req2.single_request().body_json();
    let input2 = body2["input"].as_array().expect("input array");
    assert_eq!(
        &input2[..input1.len()],
        input1.as_slice(),
        "expected cached prefix to be reused"
    );
    assert_eq!(input2[input1.len()], text_user_input("hello 2".to_string()));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn overrides_turn_context_but_keeps_cached_prefix_and_key_constant() -> anyhow::Result<()> {
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

    let TestCodex { codex, config, .. } = test_codex()
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

    let writable = TempDir::new().unwrap();
    let permission_profile = PermissionProfile::workspace_write_with(
        &[writable.abs()],
        NetworkSandboxPolicy::Enabled,
        /*exclude_tmpdir_env_var*/ true,
        /*exclude_slash_tmp*/ true,
    );
    let sandbox_policy = permission_profile
        .to_legacy_sandbox_policy(config.cwd.as_path())
        .expect("workspace profile should have legacy projection");
    codex
        .submit(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: Some(AskForApproval::Never),
            approvals_reviewer: None,
            sandbox_policy: Some(sandbox_policy),
            permission_profile: Some(permission_profile),
            windows_sandbox_level: None,
            model: None,
            effort: Some(Some(ReasoningEffort::High)),
            summary: Some(ReasoningSummary::Detailed),
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    // Second turn after overrides
    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello 2".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let request1 = req1.single_request();
    let request2 = req2.single_request();
    let body1 = request1.body_json();
    let body2 = request2.body_json();
    // prompt_cache_key should remain constant across overrides
    assert_eq!(
        body1["prompt_cache_key"], body2["prompt_cache_key"],
        "prompt_cache_key should not change across overrides"
    );

    // The entire prefix from the first request should be identical and reused
    // as the prefix of the second request, ensuring cache hit potential.
    let expected_user_message_2 = serde_json::json!({
        "type": "message",
        "role": "user",
        "content": [ { "type": "input_text", "text": "hello 2" } ]
    });
    let expected_permissions_msg = body1["input"][0].clone();
    let body1_input = body1["input"].as_array().expect("input array");
    // After overriding the turn context, emit one updated permissions message.
    let expected_permissions_msg_2 = body2["input"][body1_input.len()].clone();
    assert_ne!(
        expected_permissions_msg_2, expected_permissions_msg,
        "expected updated permissions message after override"
    );
    let mut expected_body2 = body1_input.to_vec();
    expected_body2.push(expected_permissions_msg_2);
    expected_body2.push(expected_user_message_2);
    assert_eq!(body2["input"], serde_json::Value::Array(expected_body2));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
