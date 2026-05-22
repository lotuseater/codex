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
async fn prompt_tools_are_consistent_across_requests() -> anyhow::Result<()> {
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
        thread_manager,
        ..
    } = test_codex()
        .with_config(|config| {
            config.user_instructions = Some("be consistent and helpful".to_string());
            config.model = Some("gpt-5.2".to_string());
            // Keep tool expectations stable when the default web_search mode changes.
            config
                .web_search_mode
                .set(WebSearchMode::Cached)
                .expect("test web_search_mode should satisfy constraints");
            config
                .features
                .enable(Feature::CollaborationModes)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await?;
    let base_instructions = thread_manager
        .get_models_manager()
        .get_model_info(
            config
                .model
                .as_deref()
                .expect("test config should have a model"),
            &config.to_models_manager_config(),
        )
        .await
        .base_instructions;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello 1".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
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
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let mut expected_tools_names = if cfg!(windows) {
        vec!["shell_command"]
    } else {
        vec!["exec_command", "write_stdin"]
    };
    expected_tools_names.extend([
        "update_plan",
        "request_user_input",
        "apply_patch",
        "view_image",
        "spawn_agent",
        "send_input",
        "resume_agent",
        "wait_agent",
        "close_agent",
        "web_search",
    ]);
    let body0 = req1.single_request().body_json();

    let expected_instructions = if expected_tools_names.contains(&"apply_patch") {
        base_instructions
    } else {
        [base_instructions, APPLY_PATCH_TOOL_INSTRUCTIONS.to_string()].join("\n")
    };

    assert_eq!(
        body0["instructions"],
        serde_json::json!(expected_instructions),
    );
    assert_tool_names(&body0, &expected_tools_names);

    let body1 = req2.single_request().body_json();
    assert_eq!(
        body1["instructions"],
        serde_json::json!(expected_instructions),
    );
    assert_tool_names(&body1, &expected_tools_names);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn gpt_5_tools_without_apply_patch_append_apply_patch_instructions() -> anyhow::Result<()> {
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
                .disable(Feature::ApplyPatchFreeform)
                .expect("test config should allow feature update");
            config
                .features
                .enable(Feature::CollaborationModes)
                .expect("test config should allow feature update");
            config.model = Some("gpt-5.2".to_string());
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
        })
        .await?;

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let body0 = req1.single_request().body_json();
    let instructions0 = body0["instructions"]
        .as_str()
        .expect("instructions should be a string");
    assert!(
        instructions0.contains("You are"),
        "expected non-empty instructions"
    );

    let body1 = req2.single_request().body_json();
    let instructions1 = body1["instructions"]
        .as_str()
        .expect("instructions should be a string");
    assert_eq!(
        normalize_newlines(instructions1),
        normalize_newlines(instructions0)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
