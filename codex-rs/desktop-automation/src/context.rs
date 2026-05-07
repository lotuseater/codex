#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopAutomationContextConfig {
    pub enabled: bool,
    pub proactive: bool,
    pub allow_input: bool,
    pub prefer_app_harness: bool,
}

impl Default for DesktopAutomationContextConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            proactive: true,
            allow_input: true,
            prefer_app_harness: true,
        }
    }
}

pub fn desktop_automation_context_for_prompt(
    config: DesktopAutomationContextConfig,
    prompt: &str,
) -> Option<String> {
    desktop_automation_context_for_prompt_with_availability(
        config,
        prompt,
        cfg!(target_os = "windows"),
    )
}

fn desktop_automation_context_for_prompt_with_availability(
    config: DesktopAutomationContextConfig,
    prompt: &str,
    desktop_tools_available: bool,
) -> Option<String> {
    if !desktop_tools_available || !config.enabled || !config.proactive {
        return None;
    }
    if !looks_like_desktop_automation_prompt(prompt) {
        return None;
    }

    let mut lines = vec![
        "<desktop_automation>".to_string(),
        "Native desktop automation is available for this GUI or visual task.".to_string(),
    ];
    if config.prefer_app_harness {
        lines.push(
            "Prefer app-native automation harnesses when present; use automation_harness_detect on repo roots before generic DAB.".to_string(),
        );
    }
    lines.push(
        "For live Windows app state, inspect first with dab_find_window, dab_window_check, dab_visual_scan, dab_ocr, dab_screenshot, or dab_element_map instead of guessing from shell state.".to_string(),
    );
    if config.allow_input {
        lines.push(
            "Use dab_smart_click, dab_send_keys, dab_navigate, or coordinate clicks only after inspection identifies the target window or element.".to_string(),
        );
    } else {
        lines.push("Input DAB tools are disabled; keep automation read-only.".to_string());
    }
    lines.push("</desktop_automation>".to_string());
    Some(lines.join("\n"))
}

pub fn merge_desktop_automation_context(
    desktop_automation_context: Option<String>,
    mut hook_contexts: Vec<String>,
) -> Vec<String> {
    let Some(desktop_automation_context) = desktop_automation_context else {
        return hook_contexts;
    };
    hook_contexts.retain(|context| !is_desktop_automation_context(context));
    let insert_at = if hook_contexts
        .first()
        .is_some_and(|context| context.contains("<first_moves>"))
    {
        1
    } else {
        0
    };
    hook_contexts.insert(insert_at, desktop_automation_context);
    hook_contexts
}

fn is_desktop_automation_context(text: &str) -> bool {
    text.to_ascii_lowercase().contains("<desktop_automation>")
}

fn looks_like_desktop_automation_prompt(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    [
        "app harness",
        "automation harness",
        "calculator",
        "chrome",
        "click",
        "dab",
        "desktop",
        "gui",
        "notepad",
        "ocr",
        "paint",
        "screenshot",
        "send keys",
        "ui automation",
        "visual",
        "window",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_prompt_gets_desktop_automation_context_when_available() {
        let config = DesktopAutomationContextConfig::default();

        let context = desktop_automation_context_for_prompt_with_availability(
            config,
            "Test Google Chrome with DAB",
            /*desktop_tools_available*/ true,
        )
        .expect("context");

        assert!(context.contains("automation_harness_detect"));
        assert!(context.contains("dab_visual_scan"));
        assert!(context.contains("dab_smart_click"));
    }

    #[test]
    fn non_gui_prompt_skips_desktop_automation_context() {
        let config = DesktopAutomationContextConfig::default();

        assert!(
            desktop_automation_context_for_prompt_with_availability(
                config,
                "Refactor the parser module",
                /*desktop_tools_available*/ true,
            )
            .is_none()
        );
    }

    #[test]
    fn disabled_proactive_mode_suppresses_context() {
        let config = DesktopAutomationContextConfig {
            proactive: false,
            ..DesktopAutomationContextConfig::default()
        };

        assert!(
            desktop_automation_context_for_prompt_with_availability(
                config,
                "Inspect the Paint window",
                /*desktop_tools_available*/ true,
            )
            .is_none()
        );
    }

    #[test]
    fn merge_context_keeps_first_moves_first() {
        let contexts = merge_desktop_automation_context(
            Some("<desktop_automation>\ndab\n</desktop_automation>".to_string()),
            vec![
                "<first_moves>\nreads\n</first_moves>".to_string(),
                "hook context".to_string(),
            ],
        );

        assert_eq!(
            contexts,
            vec![
                "<first_moves>\nreads\n</first_moves>".to_string(),
                "<desktop_automation>\ndab\n</desktop_automation>".to_string(),
                "hook context".to_string(),
            ]
        );
    }
}
