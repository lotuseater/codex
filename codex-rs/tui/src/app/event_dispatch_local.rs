//! Fork-local `AppEvent` handlers split out of the central dispatcher.
//!
//! These handlers cover fork-specific events (auto-loop control and context-budget/"slow mode"
//! persistence) whose bodies would otherwise live inline in [`super::event_dispatch`]. Keeping them
//! here isolates fork logic from upstream churn in the main match, so the dispatcher arms stay as
//! thin one-line calls.

use super::*;
use crate::app_event::AutoLoopUpdate;
use crate::app_event::PromptReductionTuningField;
use codex_config::types::PromptReductionModeToml;

/// Body of `AppEvent::AutoLoop`.
pub(crate) fn on_auto_loop(app: &mut App, tui: &mut tui::Tui, update: AutoLoopUpdate) {
    app.handle_auto_loop_update(update);
    tui.frame_requester().schedule_frame();
}

/// Body of `AppEvent::SubmitAutoLoopAfterSelfReview`.
pub(crate) fn on_submit_auto_loop_after_self_review(app: &mut App, tui: &mut tui::Tui) {
    app.handle_auto_loop_after_self_review();
    tui.frame_requester().schedule_frame();
}

/// Body of `AppEvent::PersistContextBudgetModeSelection`.
pub(crate) async fn on_persist_context_budget_mode_selection(
    app: &mut App,
    mode: codex_protocol::config_types::ContextBudgetMode,
) {
    app.refresh_status_line();
    let profile = app.active_profile.as_deref();
    app.config.context_budget_mode = mode;
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_profile(profile)
        .set_context_budget_mode(mode)
        .apply()
        .await
    {
        Ok(()) => {
            let status = if mode == codex_protocol::config_types::ContextBudgetMode::Slow {
                "on"
            } else {
                "off"
            };
            let mut message = format!("Slow mode set to {status}");
            if let Some(profile) = profile {
                message.push_str(" for ");
                message.push_str(profile);
                message.push_str(" profile");
            }
            app.chat_widget.add_info_message(message, /*hint*/ None);
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist slow mode selection");
            if let Some(profile) = profile {
                app.chat_widget.add_error_message(format!(
                    "Failed to save Slow mode for profile `{profile}`: {err}"
                ));
            } else {
                app.chat_widget
                    .add_error_message(format!("Failed to save default Slow mode: {err}"));
            }
        }
    }
}

/// Body of `AppEvent::PersistActionPromptMode`.
pub(crate) async fn on_persist_action_prompt_mode(app: &mut App, mode_token: String) {
    let edit = crate::legacy_core::config::edit::action_optimization_mode_edit(&mode_token);
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            if let Some(mode) =
                crate::chatwidget::prompt_injection::action_mode_from_token(&mode_token)
            {
                app.config.action_optimization_instructions.mode = mode;
            }
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist action prompt mode");
            app.chat_widget
                .add_error_message(format!("Failed to save action prompt mode: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistActionPromptVariant`.
pub(crate) async fn on_persist_action_prompt_variant(app: &mut App, variant: String) {
    let edit = crate::legacy_core::config::edit::action_optimization_variant_edit(&variant);
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            if let Some(value) =
                crate::chatwidget::prompt_injection::action_variant_from_token(&variant)
            {
                app.config.action_optimization_instructions.variant = value;
            }
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist action prompt variant");
            app.chat_widget
                .add_error_message(format!("Failed to save action prompt variant: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistActionPromptCustomText`.
pub(crate) async fn on_persist_action_prompt_custom_text(
    app: &mut App,
    custom_text: Option<String>,
) {
    let edit = crate::legacy_core::config::edit::action_optimization_custom_text_edit(
        custom_text.as_deref(),
    );
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            app.config.action_optimization_instructions.custom_text = custom_text.clone();
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist action prompt custom text");
            app.chat_widget
                .add_error_message(format!("Failed to save action prompt custom text: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistBatchPromptMode`.
pub(crate) async fn on_persist_batch_prompt_mode(app: &mut App, mode_token: String) {
    let edit = crate::legacy_core::config::edit::batch_mini_programming_mode_edit(&mode_token);
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            if let Some(mode) =
                crate::chatwidget::prompt_injection::batch_mode_from_token(&mode_token)
            {
                app.config.batch_mini_programming_instructions.mode = mode;
            }
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist batch prompt mode");
            app.chat_widget
                .add_error_message(format!("Failed to save batch prompt mode: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistBatchPromptVariant`.
pub(crate) async fn on_persist_batch_prompt_variant(app: &mut App, variant: String) {
    let edit = crate::legacy_core::config::edit::batch_mini_programming_variant_edit(&variant);
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            if let Some(value) =
                crate::chatwidget::prompt_injection::batch_variant_from_token(&variant)
            {
                app.config.batch_mini_programming_instructions.variant = value;
            }
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist batch prompt variant");
            app.chat_widget
                .add_error_message(format!("Failed to save batch prompt variant: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistBatchPromptCustomText`.
pub(crate) async fn on_persist_batch_prompt_custom_text(
    app: &mut App,
    custom_text: Option<String>,
) {
    let edit = crate::legacy_core::config::edit::batch_mini_programming_custom_text_edit(
        custom_text.as_deref(),
    );
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            app.config.batch_mini_programming_instructions.custom_text = custom_text.clone();
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist batch prompt custom text");
            app.chat_widget
                .add_error_message(format!("Failed to save batch prompt custom text: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistDelegatePromptEnabled`.
pub(crate) async fn on_persist_delegate_prompt_enabled(app: &mut App, enabled: bool) {
    let edit = crate::legacy_core::config::edit::multi_agent_v2_usage_hint_enabled_edit(enabled);
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            app.config.multi_agent_v2.usage_hint_enabled = enabled;
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist delegate prompt enabled");
            app.chat_widget
                .add_error_message(format!("Failed to save delegate prompt enabled: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistDelegatePromptK`.
pub(crate) async fn on_persist_delegate_prompt_k(app: &mut App, k: usize) {
    let edit = crate::legacy_core::config::edit::multi_agent_v2_delegation_k_edit(k);
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            app.config.multi_agent_v2.plan_token_economy_delegation_k = k;
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist delegate prompt K");
            app.chat_widget
                .add_error_message(format!("Failed to save delegate prompt K: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistDelegatePromptRootText`.
pub(crate) async fn on_persist_delegate_prompt_root_text(app: &mut App, text: Option<String>) {
    let edit =
        crate::legacy_core::config::edit::multi_agent_v2_root_usage_hint_text_edit(text.as_deref());
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            app.config.multi_agent_v2.root_agent_usage_hint_text = text.clone();
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist delegate prompt root text");
            app.chat_widget
                .add_error_message(format!("Failed to save delegate prompt root text: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistDelegatePromptSubText`.
pub(crate) async fn on_persist_delegate_prompt_sub_text(app: &mut App, text: Option<String>) {
    let edit = crate::legacy_core::config::edit::multi_agent_v2_subagent_usage_hint_text_edit(
        text.as_deref(),
    );
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            app.config.multi_agent_v2.subagent_usage_hint_text = text.clone();
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist delegate prompt sub text");
            app.chat_widget
                .add_error_message(format!("Failed to save delegate prompt sub text: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistAutoCompactEnabled`.
pub(crate) async fn on_persist_auto_compact_enabled(app: &mut App, enabled: bool) {
    let edit = crate::legacy_core::config::edit::auto_compact_enabled_edit(enabled);
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            app.config.auto_compact_enabled = enabled;
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist auto compact enabled");
            app.chat_widget
                .add_error_message(format!("Failed to save auto compact enabled: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistAutoCompactPercent`.
pub(crate) async fn on_persist_auto_compact_percent(app: &mut App, percent: u8) {
    let edit = crate::legacy_core::config::edit::model_compact_percentage_edit(percent);
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            app.config.model_compact_percentage = percent;
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist auto compact percent");
            app.chat_widget
                .add_error_message(format!("Failed to save auto compact percent: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistAutoCompactPrompt`.
pub(crate) async fn on_persist_auto_compact_prompt(app: &mut App, prompt: Option<String>) {
    let edit = crate::legacy_core::config::edit::compact_prompt_edit(prompt.as_deref());
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            app.config.compact_prompt = prompt.clone();
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist auto compact prompt");
            app.chat_widget
                .add_error_message(format!("Failed to save auto compact prompt: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistPromptReductionMode`.
pub(crate) async fn on_persist_prompt_reduction_mode(app: &mut App, mode: PromptReductionModeToml) {
    let mode_str = match mode {
        PromptReductionModeToml::Off => "off",
        PromptReductionModeToml::Conservative => "conservative",
        PromptReductionModeToml::RecencyWeighted => "recency_weighted",
    };
    let edit = crate::legacy_core::config::edit::ConfigEdit::SetPath {
        segments: vec!["prompt_reduction_mode".to_string()],
        value: toml_edit::value(mode_str),
    };
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        Ok(()) => {
            app.config.prompt_reduction_mode = mode;
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist prompt reduction mode");
            app.chat_widget
                .add_error_message(format!("Failed to save prompt reduction mode: {err}"));
        }
    }
}

/// Body of `AppEvent::PersistPromptReductionTuning`.
pub(crate) async fn on_persist_prompt_reduction_tuning(
    app: &mut App,
    field: PromptReductionTuningField,
) {
    let edit = match &field {
        PromptReductionTuningField::PreserveRecentItems(n) => ConfigEdit::SetPath {
            segments: vec![
                "prompt_reduction".to_string(),
                "preserve_recent_items".to_string(),
            ],
            value: (*n as i64).into(),
        },
        PromptReductionTuningField::RecentWindowItems(n) => ConfigEdit::SetPath {
            segments: vec![
                "prompt_reduction".to_string(),
                "recent_window_items".to_string(),
            ],
            value: (*n as i64).into(),
        },
        PromptReductionTuningField::MidWindowItems(n) => ConfigEdit::SetPath {
            segments: vec![
                "prompt_reduction".to_string(),
                "mid_window_items".to_string(),
            ],
            value: (*n as i64).into(),
        },
        PromptReductionTuningField::OldThresholdMult(v) => ConfigEdit::SetPath {
            segments: vec![
                "prompt_reduction".to_string(),
                "old_threshold_mult".to_string(),
            ],
            value: (f64::from(*v)).into(),
        },
        PromptReductionTuningField::OldExcerptMult(v) => ConfigEdit::SetPath {
            segments: vec![
                "prompt_reduction".to_string(),
                "old_excerpt_mult".to_string(),
            ],
            value: (f64::from(*v)).into(),
        },
        PromptReductionTuningField::DisabledCategories(None) => ConfigEdit::ClearPath {
            segments: vec![
                "prompt_reduction".to_string(),
                "disabled_categories".to_string(),
            ],
        },
        PromptReductionTuningField::DisabledCategories(Some(cats)) => {
            let arr: toml_edit::Array = cats.iter().map(|s| s.as_str()).collect();
            ConfigEdit::SetPath {
                segments: vec![
                    "prompt_reduction".to_string(),
                    "disabled_categories".to_string(),
                ],
                value: toml_edit::Item::Value(arr.into()),
            }
        }
        PromptReductionTuningField::MinReduceChars(n) => ConfigEdit::SetPath {
            segments: vec![
                "prompt_reduction".to_string(),
                "min_reduce_chars".to_string(),
            ],
            value: (*n as i64).into(),
        },
        PromptReductionTuningField::MinSavedTokens(n) => ConfigEdit::SetPath {
            segments: vec![
                "prompt_reduction".to_string(),
                "min_saved_tokens".to_string(),
            ],
            value: (*n as i64).into(),
        },
    };
    if let Err(err) = ConfigEditsBuilder::new(&app.config.codex_home)
        .with_edits([edit])
        .apply()
        .await
    {
        tracing::error!(error = %err, "failed to persist prompt reduction tuning");
        app.chat_widget
            .add_error_message(format!("Failed to save prompt reduction tuning: {err}"));
    }
}
