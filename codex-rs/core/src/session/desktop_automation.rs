use codex_desktop_automation::DesktopAutomationContextConfig;

use crate::config::DesktopAutomationConfig;

pub(super) fn desktop_automation_context_for_prompt(
    config: DesktopAutomationConfig,
    prompt: &str,
) -> Option<String> {
    codex_desktop_automation::desktop_automation_context_for_prompt(
        DesktopAutomationContextConfig {
            enabled: config.enabled,
            proactive: config.proactive,
            allow_input: config.allow_input,
            prefer_app_harness: config.prefer_app_harness,
        },
        prompt,
    )
}

pub(super) fn merge_desktop_automation_context(
    desktop_automation_context: Option<String>,
    hook_contexts: Vec<String>,
) -> Vec<String> {
    codex_desktop_automation::merge_desktop_automation_context(
        desktop_automation_context,
        hook_contexts,
    )
}
