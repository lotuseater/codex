use anyhow::Result;
use codex_config::types::Personality;
use codex_core_test_runtime::responses::ev_completed_with_tokens;
use codex_core_test_runtime::responses::ev_image_generation_call;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::mount_models_once;
use codex_core_test_runtime::responses::mount_sse_once;
use codex_core_test_runtime::responses::mount_sse_sequence;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::responses::sse_completed;
use codex_core_test_runtime::responses::start_mock_server;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::test_codex::TestCodex;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::test_codex::turn_permission_fields;
use codex_core_test_runtime::wait_for_event;
use codex_features::Feature;
use codex_login::CodexAuth;
use codex_models_manager::manager::RefreshStrategy;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::models::PermissionProfile;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelServiceTier;
use codex_protocol::openai_models::ModelVisibility;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_protocol::openai_models::TruncationPolicyConfig;
use codex_protocol::openai_models::default_input_modalities;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use pretty_assertions::assert_eq;
use std::path::Path;
use std::path::PathBuf;
use wiremock::MockServer;

fn read_only_user_turn(test: &TestCodex, items: Vec<UserInput>, model: String) -> Op {
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::read_only(), test.cwd_path());
    Op::UserTurn {
        environments: None,
        items,
        final_output_json_schema: None,
        cwd: test.cwd_path().to_path_buf(),
        approval_policy: AskForApproval::Never,
        approvals_reviewer: None,
        sandbox_policy,
        permission_profile,
        model,
        effort: test.config.model_reasoning_effort,
        summary: None,
        service_tier: None,
        context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
        collaboration_mode: None,
        personality: None,
    }
}

fn image_generation_artifact_path(codex_home: &Path, session_id: &str, call_id: &str) -> PathBuf {
    fn sanitize(value: &str) -> String {
        let mut sanitized: String = value
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect();
        if sanitized.is_empty() {
            sanitized = "generated_image".to_string();
        }
        sanitized
    }

    codex_home
        .join("generated_images")
        .join(sanitize(session_id))
        .join(format!("{}.png", sanitize(call_id)))
}

fn test_model_info(
    slug: &str,
    display_name: &str,
    description: &str,
    input_modalities: Vec<InputModality>,
) -> ModelInfo {
    ModelInfo {
        slug: slug.to_string(),
        display_name: display_name.to_string(),
        description: Some(description.to_string()),
        default_reasoning_level: Some(ReasoningEffort::Medium),
        supported_reasoning_levels: vec![ReasoningEffortPreset {
            effort: ReasoningEffort::Medium,
            description: ReasoningEffort::Medium.to_string(),
        }],
        shell_type: ConfigShellToolType::ShellCommand,
        visibility: ModelVisibility::List,
        supported_in_api: true,
        input_modalities,
        used_fallback_model_metadata: false,
        supports_search_tool: false,
        priority: 1,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        upgrade: None,
        base_instructions: "base instructions".to_string(),
        model_messages: None,
        supports_reasoning_summaries: false,
        default_reasoning_summary: ReasoningSummary::Auto,
        support_verbosity: false,
        default_verbosity: None,
        availability_nux: None,
        apply_patch_tool_type: None,
        web_search_tool_type: Default::default(),
        truncation_policy: TruncationPolicyConfig::bytes(/*limit*/ 10_000),
        supports_parallel_tool_calls: false,
        supports_image_detail_original: false,
        context_window: Some(272_000),
        max_context_window: None,
        auto_compact_token_limit: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn model_change_appends_model_instructions_developer_message() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = MockServer::start().await;
    let resp_mock = mount_sse_sequence(
        &server,
        vec![sse_completed("resp-1"), sse_completed("resp-2")],
    )
    .await;

    let mut builder = test_codex().with_model("gpt-5.3-codex");
    let test = builder.build(&server).await?;
    let next_model = "gpt-5.4";

    test.codex
        .submit(read_only_user_turn(
            &test,
            vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            test.session_configured.model.clone(),
        ))
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    test.codex
        .submit(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: None,
            approvals_reviewer: None,
            sandbox_policy: None,
            permission_profile: None,
            windows_sandbox_level: None,
            model: Some(next_model.to_string()),
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    test.codex
        .submit(read_only_user_turn(
            &test,
            vec![UserInput::Text {
                text: "switch models".into(),
                text_elements: Vec::new(),
            }],
            next_model.to_string(),
        ))
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = resp_mock.requests();
    assert_eq!(requests.len(), 2, "expected two model requests");

    let second_request = requests.last().expect("expected second request");
    let developer_texts = second_request.message_input_texts("developer");
    let model_switch_text = developer_texts
        .iter()
        .find(|text| text.contains("<model_switch>"))
        .expect("expected model switch message in developer input");
    assert!(
        model_switch_text.contains("The user was previously using a different model."),
        "expected model switch preamble, got: {model_switch_text:?}"
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn model_and_personality_change_only_appends_model_instructions() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let resp_mock = mount_sse_sequence(
        &server,
        vec![sse_completed("resp-1"), sse_completed("resp-2")],
    )
    .await;

    let mut builder = test_codex()
        .with_model("gpt-5.3-codex")
        .with_config(|config| {
            config
                .features
                .enable(Feature::Personality)
                .expect("test config should allow feature update");
        });
    let test = builder.build(&server).await?;
    let next_model = "exp-codex-personality";

    test.codex
        .submit(read_only_user_turn(
            &test,
            vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            test.session_configured.model.clone(),
        ))
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    test.codex
        .submit(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: None,
            approvals_reviewer: None,
            sandbox_policy: None,
            permission_profile: None,
            windows_sandbox_level: None,
            model: Some(next_model.to_string()),
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: Some(Personality::Pragmatic),
        })
        .await?;

    test.codex
        .submit(read_only_user_turn(
            &test,
            vec![UserInput::Text {
                text: "switch model and personality".into(),
                text_elements: Vec::new(),
            }],
            next_model.to_string(),
        ))
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = resp_mock.requests();
    assert_eq!(requests.len(), 2, "expected two model requests");

    let second_request = requests.last().expect("expected second request");
    let developer_texts = second_request.message_input_texts("developer");
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("<model_switch>")),
        "expected model switch message when model changes"
    );
    assert!(
        !developer_texts
            .iter()
            .any(|text| text.contains("<personality_spec>")),
        "did not expect personality update message when model changed in same turn"
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn service_tier_change_is_applied_on_next_http_turn() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let resp_mock = mount_sse_sequence(
        &server,
        vec![sse_completed("resp-1"), sse_completed("resp-2")],
    )
    .await;

    let test = test_codex().build(&server).await?;

    test.submit_turn_with_service_tier("fast turn", Some(ServiceTier::Fast))
        .await?;
    test.submit_turn_with_service_tier("standard turn", /*service_tier*/ None)
        .await?;

    let requests = resp_mock.requests();
    assert_eq!(requests.len(), 2, "expected two model requests");

    let first_body = requests[0].body_json();
    let second_body = requests[1].body_json();

    assert_eq!(first_body["service_tier"].as_str(), Some("priority"));
    assert_eq!(second_body.get("service_tier"), None);

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flex_service_tier_is_applied_to_http_turn() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let model_slug = "test-flex-model";
    let mut flex_model = test_model_info(
        model_slug,
        model_slug,
        "supports flex tier",
        default_input_modalities(),
    );
    flex_model.service_tiers = vec![ModelServiceTier {
        id: ServiceTier::Flex.request_value().to_string(),
        name: "flex".to_string(),
        description: "Flexible processing.".to_string(),
    }];
    let resp_mock = mount_sse_once(&server, sse_completed("resp-1")).await;

    let mut builder = test_codex()
        .with_model(model_slug)
        .with_config(move |config| {
            config.model_catalog = Some(ModelsResponse {
                models: vec![flex_model],
            });
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_service_tier("flex turn", Some(ServiceTier::Flex))
        .await?;

    let request = resp_mock.single_request();
    let body = request.body_json();
    assert_eq!(body["service_tier"].as_str(), Some("flex"));

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_service_tier_is_omitted_from_http_turn() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let model_slug = "test-no-tier-model";
    let model = test_model_info(
        model_slug,
        model_slug,
        "no service tiers",
        default_input_modalities(),
    );
    let resp_mock = mount_sse_once(&server, sse_completed("resp-1")).await;

    let mut builder = test_codex()
        .with_model(model_slug)
        .with_config(move |config| {
            config.model_catalog = Some(ModelsResponse {
                models: vec![model],
            });
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_service_tier("fast turn", Some(ServiceTier::Fast))
        .await?;

    let request = resp_mock.single_request();
    let body = request.body_json();
    assert_eq!(body.get("service_tier"), None);

    Ok(())
}
