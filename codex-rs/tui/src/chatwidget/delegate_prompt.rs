//! Runtime controls for the `/delegate-prompt` slash command.
//!
//! Mirrors the structure of `prompt_injection.rs` for the delegate/decompose
//! prompt-injection block. Configuration is persisted via `AppEvent::Persist*`
//! variants handled in `app/event_dispatch.rs`.

use super::*;

/// Maximum number of characters shown for a text preview.
const TEXT_PREVIEW_CHARS: usize = 80;

fn text_preview(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview: String = collapsed.chars().take(TEXT_PREVIEW_CHARS).collect();
    if collapsed.chars().count() > TEXT_PREVIEW_CHARS {
        preview.push_str("...");
    }
    preview
}

impl ChatWidget {
    // ----- /delegate-prompt ---------------------------------------------------

    pub(super) fn show_delegate_prompt_status(&mut self) {
        let cfg = &self.config.multi_agent_v2;
        let enabled = if cfg.usage_hint_enabled { "on" } else { "off" };
        let k = cfg.plan_token_economy_delegation_k;
        let root_state = match cfg.root_agent_usage_hint_text.as_deref() {
            Some(t) if !t.trim().is_empty() => {
                format!("(custom) {}", text_preview(t))
            }
            _ => "(default)".to_string(),
        };
        let sub_state = match cfg.subagent_usage_hint_text.as_deref() {
            Some(t) if !t.trim().is_empty() => {
                format!("(custom) {}", text_preview(t))
            }
            _ => "(default)".to_string(),
        };
        self.add_info_message(
            format!(
                "Delegate prompt: enabled={enabled}, K={k}\n  root -> {root_state}\n  sub  -> {sub_state}"
            ),
            /*hint*/ None,
        );
    }

    pub(super) fn handle_delegate_prompt_command_args(&mut self, trimmed: &str) {
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let token = parts.next().unwrap_or_default();
        let rest = parts.next().unwrap_or_default().trim();
        match token.to_ascii_lowercase().as_str() {
            "" | "status" => self.show_delegate_prompt_status(),
            "on" => {
                self.config.multi_agent_v2.usage_hint_enabled = true;
                self.app_event_tx
                    .send(AppEvent::PersistDelegatePromptEnabled { enabled: true });
                self.add_info_message("Delegate prompt enabled.".to_string(), /*hint*/ None);
                self.show_delegate_prompt_status();
            }
            "off" => {
                self.config.multi_agent_v2.usage_hint_enabled = false;
                self.app_event_tx
                    .send(AppEvent::PersistDelegatePromptEnabled { enabled: false });
                self.add_info_message("Delegate prompt disabled.".to_string(), /*hint*/ None);
                self.show_delegate_prompt_status();
            }
            "k" => {
                if rest.is_empty() {
                    self.add_error_message("Usage: /delegate-prompt k <n>".to_string());
                    return;
                }
                match rest.parse::<usize>() {
                    Ok(k) if k >= 1000 => {
                        self.config.multi_agent_v2.plan_token_economy_delegation_k = k;
                        self.app_event_tx
                            .send(AppEvent::PersistDelegatePromptK { k });
                        self.add_info_message(
                            format!("Delegate prompt K set to {k}."),
                            /*hint*/ None,
                        );
                        self.show_delegate_prompt_status();
                    }
                    Ok(k) => {
                        self.add_error_message(format!(
                            "K must be >= 1000 (got {k}). Delegation threshold is in tokens."
                        ));
                    }
                    Err(_) => {
                        self.add_error_message(format!(
                            "Invalid number '{rest}'. Usage: /delegate-prompt k <n>"
                        ));
                    }
                }
            }
            "root" => {
                let text = if rest.is_empty() {
                    None
                } else {
                    Some(rest.to_string())
                };
                self.config.multi_agent_v2.root_agent_usage_hint_text = text.clone();
                self.app_event_tx
                    .send(AppEvent::PersistDelegatePromptRootText { text });
                self.add_info_message(
                    "Delegate prompt root text updated.".to_string(),
                    /*hint*/ None,
                );
                self.show_delegate_prompt_status();
            }
            "sub" => {
                let text = if rest.is_empty() {
                    None
                } else {
                    Some(rest.to_string())
                };
                self.config.multi_agent_v2.subagent_usage_hint_text = text.clone();
                self.app_event_tx
                    .send(AppEvent::PersistDelegatePromptSubText { text });
                self.add_info_message(
                    "Delegate prompt sub text updated.".to_string(),
                    /*hint*/ None,
                );
                self.show_delegate_prompt_status();
            }
            "clear" => {
                self.config.multi_agent_v2.root_agent_usage_hint_text = None;
                self.config.multi_agent_v2.subagent_usage_hint_text = None;
                self.app_event_tx
                    .send(AppEvent::PersistDelegatePromptRootText { text: None });
                self.app_event_tx
                    .send(AppEvent::PersistDelegatePromptSubText { text: None });
                self.add_info_message(
                    "Delegate prompt custom texts cleared (using defaults).".to_string(),
                    /*hint*/ None,
                );
                self.show_delegate_prompt_status();
            }
            other => {
                self.add_error_message(format!(
                    "Unknown /delegate-prompt argument '{other}'. Valid: status | on | off | k <n> | root <text> | sub <text> | clear"
                ));
            }
        }
    }
}
