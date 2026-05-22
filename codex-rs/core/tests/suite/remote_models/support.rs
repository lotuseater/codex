#![cfg(not(target_os = "windows"))]
#![allow(clippy::expect_used)]
pub(crate) use anyhow::Result;
pub(crate) use codex_core_test_runtime::load_default_config_for_test;
pub(crate) use codex_core_test_runtime::responses::ev_assistant_message;
pub(crate) use codex_core_test_runtime::responses::ev_completed;
pub(crate) use codex_core_test_runtime::responses::ev_function_call;
pub(crate) use codex_core_test_runtime::responses::ev_response_created;
pub(crate) use codex_core_test_runtime::responses::mount_models_once;
pub(crate) use codex_core_test_runtime::responses::mount_models_once_with_delay;
pub(crate) use codex_core_test_runtime::responses::mount_sse_once;
pub(crate) use codex_core_test_runtime::responses::mount_sse_sequence;
pub(crate) use codex_core_test_runtime::responses::sse;
pub(crate) use codex_core_test_runtime::skip_if_no_network;
pub(crate) use codex_core_test_runtime::skip_if_sandbox;
pub(crate) use codex_core_test_runtime::test_codex::TestCodex;
pub(crate) use codex_core_test_runtime::test_codex::test_codex;
pub(crate) use codex_core_test_runtime::test_codex::turn_permission_fields;
pub(crate) use codex_core_test_runtime::wait_for_event;
pub(crate) use codex_core_test_runtime::wait_for_event_match;
pub(crate) use codex_login::CodexAuth;
pub(crate) use codex_model_provider_info::ModelProviderInfo;
pub(crate) use codex_model_provider_info::built_in_model_providers;
pub(crate) use codex_models_manager::bundled_models_response;
pub(crate) use codex_models_manager::manager::RefreshStrategy;
pub(crate) use codex_models_manager::manager::SharedModelsManager;
pub(crate) use codex_protocol::config_types::ReasoningSummary;
pub(crate) use codex_protocol::models::PermissionProfile;
pub(crate) use codex_protocol::openai_models::ConfigShellToolType;
pub(crate) use codex_protocol::openai_models::ModelInfo;
pub(crate) use codex_protocol::openai_models::ModelPreset;
pub(crate) use codex_protocol::openai_models::ModelVisibility;
pub(crate) use codex_protocol::openai_models::ModelsResponse;
pub(crate) use codex_protocol::openai_models::ReasoningEffort;
pub(crate) use codex_protocol::openai_models::ReasoningEffortPreset;
pub(crate) use codex_protocol::openai_models::TruncationPolicyConfig;
pub(crate) use codex_protocol::openai_models::default_input_modalities;
pub(crate) use codex_protocol::protocol::AskForApproval;
pub(crate) use codex_protocol::protocol::EventMsg;
pub(crate) use codex_protocol::protocol::ExecCommandSource;
pub(crate) use codex_protocol::protocol::Op;
pub(crate) use codex_protocol::user_input::UserInput;
pub(crate) use pretty_assertions::assert_eq;
pub(crate) use serde_json::json;
pub(crate) use tempfile::TempDir;
pub(crate) use tokio::time::Duration;
pub(crate) use tokio::time::Instant;
pub(crate) use tokio::time::sleep;
pub(crate) use tokio::time::timeout;
pub(crate) use wiremock::BodyPrintLimit;
pub(crate) use wiremock::MockServer;

pub(crate) const REMOTE_MODEL_SLUG: &str = "codex-test";

pub(crate) async fn wait_for_model_available(
    manager: &SharedModelsManager,
    slug: &str,
) -> ModelPreset {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(model) = {
            let guard = manager.list_models(RefreshStrategy::OnlineIfUncached).await;
            guard.iter().find(|model| model.model == slug).cloned()
        } {
            return model;
        }
        if Instant::now() >= deadline {
            panic!("timed out waiting for the remote model {slug} to appear");
        }
        sleep(Duration::from_millis(25)).await;
    }
}

pub(crate) fn bundled_model_slug() -> String {
    let response = bundled_models_response()
        .unwrap_or_else(|err| panic!("bundled models.json should parse: {err}"));
    response
        .models
        .first()
        .expect("bundled models.json should include at least one model")
        .slug
        .clone()
}

pub(crate) fn bundled_default_model_slug() -> String {
    codex_core::test_support::all_model_presets()
        .iter()
        .find(|preset| preset.is_default)
        .expect("bundled models should include a default")
        .model
        .clone()
}

pub(crate) fn test_remote_model(
    slug: &str,
    visibility: ModelVisibility,
    priority: i32,
) -> ModelInfo {
    test_remote_model_with_policy(
        slug,
        visibility,
        priority,
        TruncationPolicyConfig::bytes(/*limit*/ 10_000),
    )
}

pub(crate) fn test_remote_model_with_policy(
    slug: &str,
    visibility: ModelVisibility,
    priority: i32,
    truncation_policy: TruncationPolicyConfig,
) -> ModelInfo {
    ModelInfo {
        slug: slug.to_string(),
        display_name: format!("{slug} display"),
        description: Some(format!("{slug} description")),
        default_reasoning_level: Some(ReasoningEffort::Medium),
        supported_reasoning_levels: vec![ReasoningEffortPreset {
            effort: ReasoningEffort::Medium,
            description: ReasoningEffort::Medium.to_string(),
        }],
        shell_type: ConfigShellToolType::ShellCommand,
        visibility,
        supported_in_api: true,
        input_modalities: default_input_modalities(),
        used_fallback_model_metadata: false,
        supports_search_tool: false,
        priority,
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
        truncation_policy,
        supports_parallel_tool_calls: false,
        supports_image_detail_original: false,
        context_window: Some(272_000),
        max_context_window: None,
        auto_compact_token_limit: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
    }
}
