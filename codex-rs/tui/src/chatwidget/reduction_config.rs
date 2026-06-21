//! Runtime controls for the `/reduction-config` slash command.
//!
//! Mirrors the structure of `compact_config.rs` for the prompt-reduction
//! configuration block. Mode changes take effect immediately for the current
//! session and are persisted via `AppEvent::PersistPromptReductionMode` /
//! `AppEvent::PersistPromptReductionTuning` handled in `app/event_dispatch.rs`.

use super::*;
use crate::app_event::PromptReductionTuningField;
use codex_config::types::PromptReductionModeToml;

impl ChatWidget {
    // ----- /reduction-config --------------------------------------------------

    pub(super) fn show_reduction_config_status(&mut self) {
        let mode = match self.config.prompt_reduction_mode {
            PromptReductionModeToml::Off => "off",
            PromptReductionModeToml::Conservative => "conservative",
            PromptReductionModeToml::RecencyWeighted => "recency_weighted",
        };
        let t = &self.config.prompt_reduction;
        let mut lines = vec![format!("Reduction config: mode={mode}")];
        if let Some(v) = t.preserve_recent_items {
            lines.push(format!("  preserve_recent_items={v}"));
        }
        if let Some(v) = t.recent_window_items {
            lines.push(format!("  recent_window_items={v}"));
        }
        if let Some(v) = t.mid_window_items {
            lines.push(format!("  mid_window_items={v}"));
        }
        if let Some(v) = t.old_threshold_mult {
            lines.push(format!("  old_threshold_mult={v}"));
        }
        if let Some(v) = t.old_excerpt_mult {
            lines.push(format!("  old_excerpt_mult={v}"));
        }
        if let Some(ref cats) = t.disabled_categories {
            lines.push(format!("  disabled_categories={}", cats.join(",")));
        }
        if let Some(v) = t.min_reduce_chars {
            lines.push(format!("  min_reduce_chars={v}"));
        }
        if let Some(v) = t.min_saved_tokens {
            lines.push(format!("  min_saved_tokens={v}"));
        }
        if lines.len() == 1 {
            lines.push("  (all tuning knobs at defaults)".to_string());
        }
        self.add_info_message(lines.join("\n"), /*hint*/ None);
    }

    pub(super) fn handle_reduction_config_command_args(&mut self, trimmed: &str) {
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let token = parts.next().unwrap_or_default();
        let rest = parts.next().unwrap_or_default().trim();

        match token.to_ascii_lowercase().as_str() {
            "" | "status" => self.show_reduction_config_status(),

            // ---- mode tokens --------------------------------------------------
            "off" => {
                self.config.prompt_reduction_mode = PromptReductionModeToml::Off;
                self.app_event_tx
                    .send(AppEvent::PersistPromptReductionMode {
                        mode: PromptReductionModeToml::Off,
                    });
                self.add_info_message(
                    "Prompt reduction mode set to off.".to_string(),
                    /*hint*/ None,
                );
                self.show_reduction_config_status();
            }
            "conservative" => {
                self.config.prompt_reduction_mode = PromptReductionModeToml::Conservative;
                self.app_event_tx
                    .send(AppEvent::PersistPromptReductionMode {
                        mode: PromptReductionModeToml::Conservative,
                    });
                self.add_info_message(
                    "Prompt reduction mode set to conservative.".to_string(),
                    /*hint*/ None,
                );
                self.show_reduction_config_status();
            }
            "recency_weighted" => {
                self.config.prompt_reduction_mode = PromptReductionModeToml::RecencyWeighted;
                self.app_event_tx
                    .send(AppEvent::PersistPromptReductionMode {
                        mode: PromptReductionModeToml::RecencyWeighted,
                    });
                self.add_info_message(
                    "Prompt reduction mode set to recency_weighted.".to_string(),
                    /*hint*/ None,
                );
                self.show_reduction_config_status();
            }

            // ---- usize tuning knobs -----------------------------------------
            "preserve_recent_items" => {
                self.apply_usize_tuning(rest, "preserve_recent_items", |cfg, n| {
                    cfg.preserve_recent_items = Some(n);
                    PromptReductionTuningField::PreserveRecentItems(n)
                });
            }
            "recent_window_items" => {
                self.apply_usize_tuning(rest, "recent_window_items", |cfg, n| {
                    cfg.recent_window_items = Some(n);
                    PromptReductionTuningField::RecentWindowItems(n)
                });
            }
            "mid_window_items" => {
                self.apply_usize_tuning(rest, "mid_window_items", |cfg, n| {
                    cfg.mid_window_items = Some(n);
                    PromptReductionTuningField::MidWindowItems(n)
                });
            }
            "min_reduce_chars" => {
                self.apply_usize_tuning(rest, "min_reduce_chars", |cfg, n| {
                    cfg.min_reduce_chars = Some(n);
                    PromptReductionTuningField::MinReduceChars(n)
                });
            }
            "min_saved_tokens" => {
                self.apply_usize_tuning(rest, "min_saved_tokens", |cfg, n| {
                    cfg.min_saved_tokens = Some(n);
                    PromptReductionTuningField::MinSavedTokens(n)
                });
            }

            // ---- f32 tuning knobs -------------------------------------------
            "old_threshold_mult" => {
                self.apply_f32_tuning(rest, "old_threshold_mult", |cfg, v| {
                    cfg.old_threshold_mult = Some(v);
                    PromptReductionTuningField::OldThresholdMult(v)
                });
            }
            "old_excerpt_mult" => {
                self.apply_f32_tuning(rest, "old_excerpt_mult", |cfg, v| {
                    cfg.old_excerpt_mult = Some(v);
                    PromptReductionTuningField::OldExcerptMult(v)
                });
            }

            // ---- list tuning knob -------------------------------------------
            "disabled_categories" => {
                if rest.is_empty() {
                    self.config.prompt_reduction.disabled_categories = None;
                    self.app_event_tx
                        .send(AppEvent::PersistPromptReductionTuning {
                            field: PromptReductionTuningField::DisabledCategories(None),
                        });
                    self.add_info_message(
                        "Reduction tuning disabled_categories cleared.".to_string(),
                        /*hint*/ None,
                    );
                } else {
                    let cats: Vec<String> = rest.split(',').map(|s| s.trim().to_string()).collect();
                    self.config.prompt_reduction.disabled_categories = Some(cats.clone());
                    self.app_event_tx
                        .send(AppEvent::PersistPromptReductionTuning {
                            field: PromptReductionTuningField::DisabledCategories(Some(
                                cats.clone(),
                            )),
                        });
                    self.add_info_message(
                        format!(
                            "Reduction tuning disabled_categories set to [{}].",
                            cats.join(", ")
                        ),
                        /*hint*/ None,
                    );
                }
                self.show_reduction_config_status();
            }

            other => {
                self.add_error_message(format!(
                    "Unknown /reduction-config argument '{other}'. \
                     Valid: status | off | conservative | recency_weighted | \
                     preserve_recent_items <n> | recent_window_items <n> | \
                     mid_window_items <n> | old_threshold_mult <f> | \
                     old_excerpt_mult <f> | disabled_categories [<cat,...>] | \
                     min_reduce_chars <n> | min_saved_tokens <n>"
                ));
            }
        }
    }

    /// Parse `raw` as a `usize`, mutate the live tuning config via `apply`, send the persist
    /// event, and echo confirmation — or emit an error on bad input.
    fn apply_usize_tuning(
        &mut self,
        raw: &str,
        key: &str,
        apply: impl FnOnce(
            &mut codex_config::types::PromptReductionTuning,
            usize,
        ) -> PromptReductionTuningField,
    ) {
        if raw.is_empty() {
            self.add_error_message(format!("Usage: /reduction-config {key} <usize>"));
            return;
        }
        match raw.parse::<usize>() {
            Ok(n) => {
                let field = apply(&mut self.config.prompt_reduction, n);
                self.app_event_tx
                    .send(AppEvent::PersistPromptReductionTuning { field });
                self.add_info_message(
                    format!("Reduction tuning {key} set to {n}."),
                    /*hint*/ None,
                );
                self.show_reduction_config_status();
            }
            Err(_) => {
                self.add_error_message(format!("Invalid usize '{raw}' for {key}."));
            }
        }
    }

    /// Parse `raw` as an `f32`, mutate the live tuning config via `apply`, send the persist
    /// event, and echo confirmation — or emit an error on bad input.
    fn apply_f32_tuning(
        &mut self,
        raw: &str,
        key: &str,
        apply: impl FnOnce(
            &mut codex_config::types::PromptReductionTuning,
            f32,
        ) -> PromptReductionTuningField,
    ) {
        if raw.is_empty() {
            self.add_error_message(format!("Usage: /reduction-config {key} <f32>"));
            return;
        }
        match raw.parse::<f32>() {
            Ok(v) => {
                let field = apply(&mut self.config.prompt_reduction, v);
                self.app_event_tx
                    .send(AppEvent::PersistPromptReductionTuning { field });
                self.add_info_message(
                    format!("Reduction tuning {key} set to {v}."),
                    /*hint*/ None,
                );
                self.show_reduction_config_status();
            }
            Err(_) => {
                self.add_error_message(format!("Invalid f32 '{raw}' for {key}."));
            }
        }
    }
}
