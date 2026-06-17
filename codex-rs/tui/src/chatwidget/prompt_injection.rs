//! Runtime controls for the two developer prompt-injection blocks, driven by the
//! `/action-prompt` and `/batch-prompt` slash commands.
//!
//! Keeping this wiring out of the slash dispatcher lets the command parser stay
//! thin while preserving one owner for parsing, config updates, and the
//! config-persistence handoff. The TUI-side `self.config` field is updated here so
//! the status readout reflects the new selection immediately; the async write to
//! `config.toml` is performed by the matching `AppEvent::Persist*` handler in
//! `app/event_dispatch.rs`, mirroring the `/fast` and `/slow` commands.
//!
//! NOTE: these two blocks are materialized into the session's `Config` once at
//! session start, so the persisted change takes effect on the next session (after
//! restart), not mid-session. Updating the TUI-side config keeps the `status`
//! output and any recreated widgets (new/resume/fork) consistent with what was
//! written to disk.

use super::*;
use crate::legacy_core::config::ActionOptimizationInstructionsMode;
use crate::legacy_core::config::ActionOptimizationInstructionsVariant;
use crate::legacy_core::config::BatchMiniProgrammingInstructionsMode;
use crate::legacy_core::config::BatchMiniProgrammingInstructionsVariant;

/// Valid action-optimization variant tokens, in display order.
const ACTION_VARIANTS: &[&str] = &["action_route_selection", "routing", "verbose"];
/// Valid batch mini-programming variant tokens, in display order.
const BATCH_VARIANTS: &[&str] = &["current", "aggressive", "compact"];

/// Maximum number of characters shown for a resolved-body preview.
const BODY_PREVIEW_CHARS: usize = 80;

fn action_variant_token(variant: ActionOptimizationInstructionsVariant) -> &'static str {
    match variant {
        ActionOptimizationInstructionsVariant::ActionRouteSelection => "action_route_selection",
        ActionOptimizationInstructionsVariant::Routing => "routing",
        ActionOptimizationInstructionsVariant::Verbose => "verbose",
    }
}

pub(crate) fn action_variant_from_token(
    token: &str,
) -> Option<ActionOptimizationInstructionsVariant> {
    match token {
        "action_route_selection" => {
            Some(ActionOptimizationInstructionsVariant::ActionRouteSelection)
        }
        "routing" => Some(ActionOptimizationInstructionsVariant::Routing),
        "verbose" => Some(ActionOptimizationInstructionsVariant::Verbose),
        _ => None,
    }
}

pub(crate) fn action_mode_from_token(token: &str) -> Option<ActionOptimizationInstructionsMode> {
    match token {
        "off" => Some(ActionOptimizationInstructionsMode::Off),
        "plan" => Some(ActionOptimizationInstructionsMode::Plan),
        "first_turn" => Some(ActionOptimizationInstructionsMode::FirstTurn),
        "tool_turn" => Some(ActionOptimizationInstructionsMode::ToolTurn),
        "always" => Some(ActionOptimizationInstructionsMode::Always),
        _ => None,
    }
}

fn action_mode_token(mode: ActionOptimizationInstructionsMode) -> &'static str {
    match mode {
        ActionOptimizationInstructionsMode::Off => "off",
        ActionOptimizationInstructionsMode::Plan => "plan",
        ActionOptimizationInstructionsMode::FirstTurn => "first_turn",
        ActionOptimizationInstructionsMode::ToolTurn => "tool_turn",
        ActionOptimizationInstructionsMode::Always => "always",
    }
}

fn batch_variant_token(variant: BatchMiniProgrammingInstructionsVariant) -> &'static str {
    match variant {
        BatchMiniProgrammingInstructionsVariant::Current => "current",
        BatchMiniProgrammingInstructionsVariant::Aggressive => "aggressive",
        BatchMiniProgrammingInstructionsVariant::Compact => "compact",
    }
}

pub(crate) fn batch_variant_from_token(
    token: &str,
) -> Option<BatchMiniProgrammingInstructionsVariant> {
    match token {
        "current" => Some(BatchMiniProgrammingInstructionsVariant::Current),
        "aggressive" => Some(BatchMiniProgrammingInstructionsVariant::Aggressive),
        "compact" => Some(BatchMiniProgrammingInstructionsVariant::Compact),
        _ => None,
    }
}

pub(crate) fn batch_mode_from_token(token: &str) -> Option<BatchMiniProgrammingInstructionsMode> {
    match token {
        "off" => Some(BatchMiniProgrammingInstructionsMode::Off),
        "always" => Some(BatchMiniProgrammingInstructionsMode::Always),
        _ => None,
    }
}

fn batch_mode_token(mode: BatchMiniProgrammingInstructionsMode) -> &'static str {
    match mode {
        BatchMiniProgrammingInstructionsMode::Off => "off",
        BatchMiniProgrammingInstructionsMode::Always => "always",
    }
}

/// Build a short, single-line preview of the body that would be injected.
///
/// When `custom_text` is set and non-empty it overrides the variant, so preview
/// that; otherwise report which baked variant is active.
fn resolved_body_preview(custom_text: Option<&str>, variant_token: &str) -> String {
    match custom_text {
        Some(text) if !text.trim().is_empty() => {
            let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
            let mut preview: String = collapsed.chars().take(BODY_PREVIEW_CHARS).collect();
            if collapsed.chars().count() > BODY_PREVIEW_CHARS {
                preview.push_str("...");
            }
            format!("custom: {preview}")
        }
        _ => format!("variant: {variant_token}"),
    }
}

fn custom_text_state(custom_text: Option<&str>) -> &'static str {
    match custom_text {
        Some(text) if !text.trim().is_empty() => "set",
        _ => "unset",
    }
}

impl ChatWidget {
    // ----- /action-prompt -----------------------------------------------------

    pub(super) fn show_action_prompt_status(&mut self) {
        let cfg = &self.config.action_optimization_instructions;
        let mode = action_mode_token(cfg.mode);
        let variant = action_variant_token(cfg.variant);
        let custom = custom_text_state(cfg.custom_text.as_deref());
        let preview = resolved_body_preview(cfg.custom_text.as_deref(), variant);
        self.add_info_message(
            format!(
                "Action prompt: mode={mode}, variant={variant}, custom_text={custom}\n  body -> {preview}"
            ),
            /*hint*/ None,
        );
    }

    pub(super) fn handle_action_prompt_command_args(&mut self, trimmed: &str) {
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let token = parts.next().unwrap_or_default();
        let rest = parts.next().unwrap_or_default().trim();
        match token.to_ascii_lowercase().as_str() {
            "" | "status" => self.show_action_prompt_status(),
            "on" => self.set_action_prompt_mode(ActionOptimizationInstructionsMode::Always),
            "off" => self.set_action_prompt_mode(ActionOptimizationInstructionsMode::Off),
            "custom" => {
                if rest.is_empty() {
                    self.add_error_message("Usage: /action-prompt custom <text>".to_string());
                } else {
                    self.set_action_prompt_custom_text(Some(rest.to_string()));
                }
            }
            "clear" | "default" => self.set_action_prompt_custom_text(None),
            other => {
                if let Some(variant) = action_variant_from_token(other) {
                    self.set_action_prompt_variant(variant);
                } else {
                    self.add_error_message(format!(
                        "Unknown /action-prompt argument '{other}'. Valid: status | on | off | {} | custom <text> | clear",
                        ACTION_VARIANTS.join(" | ")
                    ));
                }
            }
        }
    }

    fn set_action_prompt_mode(&mut self, mode: ActionOptimizationInstructionsMode) {
        self.config.action_optimization_instructions.mode = mode;
        let mode_token = action_mode_token(mode).to_string();
        self.app_event_tx
            .send(AppEvent::PersistActionPromptMode { mode_token });
        self.add_info_message(
            format!("Action prompt mode set to {}.", action_mode_token(mode)),
            /*hint*/ None,
        );
        self.show_action_prompt_status();
    }

    fn set_action_prompt_variant(&mut self, variant: ActionOptimizationInstructionsVariant) {
        self.config.action_optimization_instructions.variant = variant;
        let token = action_variant_token(variant);
        self.app_event_tx
            .send(AppEvent::PersistActionPromptVariant {
                variant: token.to_string(),
            });
        self.add_info_message(
            format!("Action prompt variant set to {token}."),
            /*hint*/ None,
        );
        self.show_action_prompt_status();
    }

    fn set_action_prompt_custom_text(&mut self, custom_text: Option<String>) {
        self.config.action_optimization_instructions.custom_text = custom_text.clone();
        self.app_event_tx
            .send(AppEvent::PersistActionPromptCustomText {
                custom_text: custom_text.clone(),
            });
        let message = if custom_text.is_some() {
            "Action prompt custom text saved.".to_string()
        } else {
            "Action prompt custom text cleared.".to_string()
        };
        self.add_info_message(message, /*hint*/ None);
        self.show_action_prompt_status();
    }

    // ----- /batch-prompt ------------------------------------------------------

    pub(super) fn show_batch_prompt_status(&mut self) {
        let cfg = &self.config.batch_mini_programming_instructions;
        let mode = batch_mode_token(cfg.mode);
        let variant = batch_variant_token(cfg.variant);
        let custom = custom_text_state(cfg.custom_text.as_deref());
        let preview = resolved_body_preview(cfg.custom_text.as_deref(), variant);
        self.add_info_message(
            format!(
                "Batch prompt: mode={mode}, variant={variant}, custom_text={custom}\n  body -> {preview}"
            ),
            /*hint*/ None,
        );
    }

    pub(super) fn handle_batch_prompt_command_args(&mut self, trimmed: &str) {
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let token = parts.next().unwrap_or_default();
        let rest = parts.next().unwrap_or_default().trim();
        match token.to_ascii_lowercase().as_str() {
            "" | "status" => self.show_batch_prompt_status(),
            "on" => self.set_batch_prompt_mode(BatchMiniProgrammingInstructionsMode::Always),
            "off" => self.set_batch_prompt_mode(BatchMiniProgrammingInstructionsMode::Off),
            "custom" => {
                if rest.is_empty() {
                    self.add_error_message("Usage: /batch-prompt custom <text>".to_string());
                } else {
                    self.set_batch_prompt_custom_text(Some(rest.to_string()));
                }
            }
            "clear" | "default" => self.set_batch_prompt_custom_text(None),
            other => {
                if let Some(variant) = batch_variant_from_token(other) {
                    self.set_batch_prompt_variant(variant);
                } else {
                    self.add_error_message(format!(
                        "Unknown /batch-prompt argument '{other}'. Valid: status | on | off | {} | custom <text> | clear",
                        BATCH_VARIANTS.join(" | ")
                    ));
                }
            }
        }
    }

    fn set_batch_prompt_mode(&mut self, mode: BatchMiniProgrammingInstructionsMode) {
        self.config.batch_mini_programming_instructions.mode = mode;
        let mode_token = batch_mode_token(mode).to_string();
        self.app_event_tx
            .send(AppEvent::PersistBatchPromptMode { mode_token });
        self.add_info_message(
            format!("Batch prompt mode set to {}.", batch_mode_token(mode)),
            /*hint*/ None,
        );
        self.show_batch_prompt_status();
    }

    fn set_batch_prompt_variant(&mut self, variant: BatchMiniProgrammingInstructionsVariant) {
        self.config.batch_mini_programming_instructions.variant = variant;
        let token = batch_variant_token(variant);
        self.app_event_tx.send(AppEvent::PersistBatchPromptVariant {
            variant: token.to_string(),
        });
        self.add_info_message(
            format!("Batch prompt variant set to {token}."),
            /*hint*/ None,
        );
        self.show_batch_prompt_status();
    }

    fn set_batch_prompt_custom_text(&mut self, custom_text: Option<String>) {
        self.config.batch_mini_programming_instructions.custom_text = custom_text.clone();
        self.app_event_tx
            .send(AppEvent::PersistBatchPromptCustomText {
                custom_text: custom_text.clone(),
            });
        let message = if custom_text.is_some() {
            "Batch prompt custom text saved.".to_string()
        } else {
            "Batch prompt custom text cleared.".to_string()
        };
        self.add_info_message(message, /*hint*/ None);
        self.show_batch_prompt_status();
    }
}
