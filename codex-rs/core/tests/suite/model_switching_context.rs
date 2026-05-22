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
async fn model_switch_to_smaller_model_updates_token_context_window() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let large_model_slug = "test-image-model";
    let smaller_model_slug = "test-text-only-model";
    let large_context_window = 272_000;
    let smaller_context_window = 128_000;
    let effective_context_window_percent = 95;
    let large_effective_window = (large_context_window * effective_context_window_percent) / 100;
    let smaller_effective_window =
        (smaller_context_window * effective_context_window_percent) / 100;

    let base_model = ModelInfo {
        slug: large_model_slug.to_string(),
        display_name: "Larger Model".to_string(),
        description: Some("larger context window model".to_string()),
        default_reasoning_level: Some(ReasoningEffort::Medium),
        supported_reasoning_levels: vec![ReasoningEffortPreset {
            effort: ReasoningEffort::Medium,
            description: ReasoningEffort::Medium.to_string(),
        }],
        shell_type: ConfigShellToolType::ShellCommand,
        visibility: ModelVisibility::List,
        supported_in_api: true,
        input_modalities: default_input_modalities(),
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
        context_window: Some(large_context_window),
        max_context_window: None,
        auto_compact_token_limit: None,
        effective_context_window_percent,
        experimental_supported_tools: Vec::new(),
    };
    let mut smaller_model = base_model.clone();
    smaller_model.slug = smaller_model_slug.to_string();
    smaller_model.display_name = "Smaller Model".to_string();
    smaller_model.description = Some("smaller context window model".to_string());
    smaller_model.context_window = Some(smaller_context_window);

    mount_models_once(
        &server,
        ModelsResponse {
            models: vec![base_model, smaller_model],
        },
    )
    .await;

    mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_completed_with_tokens("resp-1", /*total_tokens*/ 100),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_completed_with_tokens("resp-2", /*total_tokens*/ 120),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(|config| {
            config.model = Some(large_model_slug.to_string());
        });
    let test = builder.build(&server).await?;

    let models_manager = test.thread_manager.get_models_manager();
    let available_models = models_manager.list_models(RefreshStrategy::Online).await;
    assert!(
        available_models
            .iter()
            .any(|model| model.model == smaller_model_slug),
        "expected {smaller_model_slug} to be available in remote model list"
    );
    let large_model_info = models_manager
        .get_model_info(large_model_slug, &test.config.to_models_manager_config())
        .await;
    assert_eq!(large_model_info.context_window, Some(large_context_window));
    let smaller_model_info = models_manager
        .get_model_info(smaller_model_slug, &test.config.to_models_manager_config())
        .await;
    assert_eq!(
        smaller_model_info.context_window,
        Some(smaller_context_window)
    );

    test.codex
        .submit(read_only_user_turn(
            &test,
            vec![UserInput::Text {
                text: "use larger model".into(),
                text_elements: Vec::new(),
            }],
            large_model_slug.to_string(),
        ))
        .await?;

    let large_window_event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::TokenCount(token_count)
                if token_count
                    .info
                    .as_ref()
                    .is_some_and(|info| info.last_token_usage.total_tokens == 100)
        )
    })
    .await;
    let EventMsg::TokenCount(large_token_count) = large_window_event else {
        unreachable!("wait_for_event returned unexpected event");
    };
    assert_eq!(
        large_token_count
            .info
            .as_ref()
            .and_then(|info| info.model_context_window),
        Some(large_effective_window)
    );
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    test.codex
        .submit(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: None,
            approvals_reviewer: None,
            sandbox_policy: None,
            permission_profile: None,
            windows_sandbox_level: None,
            model: Some(smaller_model_slug.to_string()),
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
                text: "switch to smaller model".into(),
                text_elements: Vec::new(),
            }],
            smaller_model_slug.to_string(),
        ))
        .await?;

    let smaller_turn_started_event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::TurnStarted(started)
                if started.model_context_window == Some(smaller_effective_window)
        )
    })
    .await;
    let EventMsg::TurnStarted(smaller_turn_started) = smaller_turn_started_event else {
        unreachable!("wait_for_event returned unexpected event");
    };
    assert_eq!(
        smaller_turn_started.model_context_window,
        Some(smaller_effective_window)
    );

    let smaller_window_event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::TokenCount(token_count)
                if token_count
                    .info
                    .as_ref()
                    .is_some_and(|info| info.last_token_usage.total_tokens == 120)
        )
    })
    .await;
    let EventMsg::TokenCount(smaller_token_count) = smaller_window_event else {
        unreachable!("wait_for_event returned unexpected event");
    };
    let smaller_window = smaller_token_count
        .info
        .as_ref()
        .and_then(|info| info.model_context_window);
    assert_eq!(smaller_window, Some(smaller_effective_window));
    assert_ne!(smaller_window, Some(large_effective_window));
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    Ok(())
}
