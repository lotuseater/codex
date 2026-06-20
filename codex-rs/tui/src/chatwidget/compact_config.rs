//! Runtime controls for the `/compact-config` slash command.
//!
//! Mirrors the structure of `prompt_injection.rs` for the auto-compaction
//! configuration block. Configuration is persisted via `AppEvent::Persist*`
//! variants handled in `app/event_dispatch.rs`.

use super::*;

/// Maximum number of characters shown for a prompt text preview.
const PROMPT_PREVIEW_CHARS: usize = 80;

fn prompt_preview(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview: String = collapsed.chars().take(PROMPT_PREVIEW_CHARS).collect();
    if collapsed.chars().count() > PROMPT_PREVIEW_CHARS {
        preview.push_str("...");
    }
    preview
}

impl ChatWidget {
    // ----- /compact-config ----------------------------------------------------

    pub(super) fn show_compact_config_status(&mut self) {
        let enabled = if self.config.auto_compact_enabled {
            "on"
        } else {
            "off"
        };
        let percent = self.config.model_compact_percentage;
        let prompt_state = match self.config.compact_prompt.as_deref() {
            Some(t) if !t.trim().is_empty() => {
                format!("(custom) {}", prompt_preview(t))
            }
            _ => "(default)".to_string(),
        };
        self.add_info_message(
            format!(
                "Compact config: enabled={enabled}, percent={percent}%\n  prompt -> {prompt_state}"
            ),
            /*hint*/ None,
        );
    }

    pub(super) fn handle_compact_config_command_args(&mut self, trimmed: &str) {
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let token = parts.next().unwrap_or_default();
        let rest = parts.next().unwrap_or_default().trim();
        match token.to_ascii_lowercase().as_str() {
            "" | "status" => self.show_compact_config_status(),
            "on" => {
                self.config.auto_compact_enabled = true;
                self.app_event_tx
                    .send(AppEvent::PersistAutoCompactEnabled { enabled: true });
                self.add_info_message("Auto-compaction enabled.".to_string(), /*hint*/ None);
                self.show_compact_config_status();
            }
            "off" => {
                self.config.auto_compact_enabled = false;
                self.app_event_tx
                    .send(AppEvent::PersistAutoCompactEnabled { enabled: false });
                self.add_info_message("Auto-compaction disabled.".to_string(), /*hint*/ None);
                self.show_compact_config_status();
            }
            "percent" => {
                if rest.is_empty() {
                    self.add_error_message("Usage: /compact-config percent <1-100>".to_string());
                    return;
                }
                match rest.parse::<u8>() {
                    Ok(p) if (1..=100).contains(&p) => {
                        self.config.model_compact_percentage = p;
                        self.app_event_tx
                            .send(AppEvent::PersistAutoCompactPercent { percent: p });
                        self.add_info_message(
                            format!("Auto-compact threshold set to {p}%."),
                            /*hint*/ None,
                        );
                        self.show_compact_config_status();
                    }
                    Ok(p) => {
                        self.add_error_message(format!(
                            "Percent must be between 1 and 100 (got {p})."
                        ));
                    }
                    Err(_) => {
                        self.add_error_message(format!(
                            "Invalid number '{rest}'. Usage: /compact-config percent <1-100>"
                        ));
                    }
                }
            }
            "prompt" => {
                let prompt = if rest.is_empty() {
                    None
                } else {
                    Some(rest.to_string())
                };
                self.config.compact_prompt = prompt.clone();
                self.app_event_tx
                    .send(AppEvent::PersistAutoCompactPrompt { prompt });
                self.add_info_message("Compact prompt updated.".to_string(), /*hint*/ None);
                self.show_compact_config_status();
            }
            other => {
                self.add_error_message(format!(
                    "Unknown /compact-config argument '{other}'. Valid: status | on | off | percent <n> | prompt [<text>]"
                ));
            }
        }
    }
}
