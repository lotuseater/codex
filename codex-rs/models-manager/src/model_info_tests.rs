use super::*;
use crate::ModelsManagerConfig;
use codex_protocol::openai_models::ApprovalMessages;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelVisibility;
use codex_protocol::openai_models::TruncationPolicyConfig;
use pretty_assertions::assert_eq;

fn remote_style_model() -> ModelInfo {
    ModelInfo {
        slug: "remote-model".to_string(),
        display_name: "Remote Model".to_string(),
        description: None,
        default_reasoning_level: None,
        supported_reasoning_levels: Vec::new(),
        shell_type: codex_protocol::openai_models::ConfigShellToolType::Default,
        visibility: ModelVisibility::None,
        supported_in_api: true,
        priority: 0,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        availability_nux: None,
        upgrade: None,
        base_instructions: "Remote base instructions.".to_string(),
        model_messages: None,
        supports_reasoning_summaries: false,
        default_reasoning_summary: codex_protocol::config_types::ReasoningSummary::Auto,
        support_verbosity: false,
        default_verbosity: None,
        apply_patch_tool_type: None,
        web_search_tool_type: codex_protocol::openai_models::WebSearchToolType::Text,
        truncation_policy: TruncationPolicyConfig::bytes(/*limit*/ 10_000),
        supports_parallel_tool_calls: false,
        supports_image_detail_original: false,
        context_window: Some(272_000),
        max_context_window: Some(272_000),
        auto_compact_token_limit: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
        input_modalities: codex_protocol::openai_models::default_input_modalities(),
        used_fallback_model_metadata: false,
        supports_search_tool: false,
    }
}

#[test]
fn fallback_model_includes_self_review_instructions() {
    let model = model_info_from_slug("unknown-model");

    assert!(
        model
            .base_instructions
            .contains("## Self-Review Discipline")
    );
    assert!(model.base_instructions.contains("once every 10 minutes"));
    assert!(
        model
            .base_instructions
            .contains("including after drafting plans")
    );
    assert!(
        model
            .base_instructions
            .contains("brief and token-efficient")
    );
    assert!(
        model
            .base_instructions
            .contains("Report self-review details only")
    );
    assert!(model.base_instructions.contains("review before the commit"));
}

#[test]
fn config_overrides_append_self_review_to_remote_style_model() {
    let model = remote_style_model();
    let updated = with_config_overrides(model, &ModelsManagerConfig::default());

    assert!(
        updated
            .base_instructions
            .contains("Remote base instructions.")
    );
    assert!(
        updated
            .base_instructions
            .contains("## Self-Review Discipline")
    );
}

#[test]
fn config_overrides_do_not_duplicate_self_review_instructions() {
    let model = model_info_from_slug("unknown-model");
    let updated = with_config_overrides(model, &ModelsManagerConfig::default());

    assert_eq!(
        updated
            .base_instructions
            .matches("## Self-Review Discipline")
            .count(),
        1
    );
}

#[test]
fn explicit_base_instructions_override_self_review_overlay() {
    let model = remote_style_model();
    let config = ModelsManagerConfig {
        base_instructions: Some("Custom base instructions.".to_string()),
        ..Default::default()
    };

    let updated = with_config_overrides(model, &config);

    assert_eq!(updated.base_instructions, "Custom base instructions.");
}

#[test]
fn reasoning_summaries_override_true_enables_support() {
    let model = model_info_from_slug("unknown-model");
    let config = ModelsManagerConfig {
        model_supports_reasoning_summaries: Some(true),
        ..Default::default()
    };

    let updated = with_config_overrides(model.clone(), &config);
    let mut expected = model;
    expected.supports_reasoning_summaries = true;

    assert_eq!(updated, expected);
}

#[test]
fn reasoning_summaries_override_false_does_not_disable_support() {
    let mut model = model_info_from_slug("unknown-model");
    model.supports_reasoning_summaries = true;
    let config = ModelsManagerConfig {
        model_supports_reasoning_summaries: Some(false),
        ..Default::default()
    };

    let updated = with_config_overrides(model.clone(), &config);

    assert_eq!(updated, model);
}

#[test]
fn reasoning_summaries_override_false_is_noop_when_model_is_false() {
    let model = model_info_from_slug("unknown-model");
    let config = ModelsManagerConfig {
        model_supports_reasoning_summaries: Some(false),
        ..Default::default()
    };

    let updated = with_config_overrides(model.clone(), &config);

    assert_eq!(updated, model);
}

#[test]
fn base_instruction_override_preserves_catalog_approval_messages() {
    let mut model = model_info_from_slug("unknown-model");
    let approvals = ApprovalMessages {
        on_request: Some("user approvals".to_string()),
        on_request_auto_review: Some("auto approvals".to_string()),
    };
    model.model_messages = Some(ModelMessages {
        instructions_template: Some("template".to_string()),
        instructions_variables: Some(ModelInstructionsVariables {
            personality_default: Some("default".to_string()),
            personality_friendly: Some("friendly".to_string()),
            personality_pragmatic: Some("pragmatic".to_string()),
        }),
        approvals: Some(approvals.clone()),
    });
    let config = ModelsManagerConfig {
        base_instructions: Some("override".to_string()),
        ..Default::default()
    };

    let updated = with_config_overrides(model, &config);

    assert_eq!(
        updated.model_messages,
        Some(ModelMessages {
            instructions_template: None,
            instructions_variables: None,
            approvals: Some(approvals),
        })
    );
}

#[test]
fn disabled_personality_preserves_catalog_approval_messages() {
    let mut model = model_info_from_slug("unknown-model");
    let approvals = ApprovalMessages {
        on_request: Some("user approvals".to_string()),
        on_request_auto_review: None,
    };
    model.model_messages = Some(ModelMessages {
        instructions_template: Some("template".to_string()),
        instructions_variables: None,
        approvals: Some(approvals.clone()),
    });
    let config = ModelsManagerConfig {
        personality_enabled: false,
        ..Default::default()
    };

    let updated = with_config_overrides(model, &config);

    assert_eq!(
        updated.model_messages,
        Some(ModelMessages {
            instructions_template: None,
            instructions_variables: None,
            approvals: Some(approvals),
        })
    );
}

#[test]
fn model_context_window_override_clamps_to_max_context_window() {
    let mut model = model_info_from_slug("unknown-model");
    model.context_window = Some(273_000);
    model.max_context_window = Some(400_000);
    let config = ModelsManagerConfig {
        model_context_window: Some(500_000),
        ..Default::default()
    };

    let updated = with_config_overrides(model.clone(), &config);
    let mut expected = model;
    expected.context_window = Some(400_000);

    assert_eq!(updated, expected);
}

#[test]
fn model_context_window_uses_model_value_without_override() {
    let mut model = model_info_from_slug("unknown-model");
    model.context_window = Some(273_000);
    model.max_context_window = Some(400_000);
    let config = ModelsManagerConfig::default();

    let updated = with_config_overrides(model.clone(), &config);

    assert_eq!(updated, model);
}
