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
            "Use dab_smart_click, dab_send_keys, dab_navigate, dab_drag, dab_scroll, dab_terminal_tabs, dab_terminal_focus, or coordinate clicks only after inspection identifies the target window or element.".to_string(),
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
    // Lower-case for ASCII needles; the raw prompt is kept for Cyrillic needles
    // (Rust's `to_ascii_lowercase` doesn't fold Cyrillic case, so we just match
    // on substring as-typed — Ukrainian nouns are usually written lower-case in
    // prompts already, and any case mismatch is a minor recall miss, not an FP).
    let lower = prompt.to_ascii_lowercase();
    const ASCII_NEEDLES: &[&str] = &[
        // --- original 19 ---
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
        "powershell",
        "screenshot",
        "send keys",
        "terminal",
        "ui automation",
        "visual",
        "vs code",
        "window",
        "youtube",
        // --- browsers (avoid bare "edge" — collides with "edge case") ---
        "browser",
        "firefox",
        "microsoft edge",
        "edge browser",
        // --- ms apps ---
        "outlook",
        "teams",
        "copilot",
        "explorer",
        "file manager",
        "control panel",
        "task manager",
        "settings app",
        // --- third-party apps ---
        "slack",
        "discord",
        "spotify",
        "zoom app",
        "telegram",
        "vlc",
        "obs",
        // --- mouse verbs ---
        "drag",
        "hover",
        "right click",
        "right-click",
        "double click",
        "double-click",
        "scroll",
        // --- keyboard verbs ---
        "type into",
        "type text",
        "keyboard shortcut",
        "shortcut key",
        "press key",
        "win+",
        // --- visual verbs ---
        "snapshot",
        "capture screen",
        "on screen",
        "on my screen",
        "active window",
        "foreground window",
        // --- window verbs ---
        "minimize",
        "minimise",
        "maximize",
        "maximise",
        "resize window",
        "move window",
        "focus window",
        // --- ui primitives ---
        "system tray",
        "taskbar",
        "tooltip",
        "menu bar",
        "context menu",
        "dialog box",
        "accessibility tree",
        "ui tree",
        "uia",
        // --- generic verbs ---
        "automate",
        "automation",
        // --- win-specific ---
        "windows 11",
        "win11",
        "clipboard",
        "winrt",
    ];

    const CYRILLIC_NEEDLES: &[&str] = &[
        // --- nouns (substrings cover declensions: вікно/вікні/вікна, екран/екрані, тощо) ---
        "вікно",
        "вікні",
        "вікна",
        "вкладк", // вкладка/вкладці/вкладку
        "скріншот",
        "знімок екрана",
        "екран",
        "робочий стіл",
        "термінал",
        "застосунк", // застосунок/застосунку/застосунка
        "програм",   // програма/програми/програмою
        // --- common short locative phrases the user actually uses ---
        "на екрані",
        "у вікні",
        "в вікні",
        "у відкритому вікні",
        "у вкладці",
        "в активному вікні",
        // --- verbs (stems cover most conjugations) ---
        "автоматиз", // автоматизуй/автоматизуйте/автоматизація
        "натисн",    // натисни/натисніть/натиснути
        "клікн",     // клікни/клікніть/клікнути
        "клік",      // клік (noun)
        "перетягн",  // перетягни/перетягніть/перетягнути
        "перетягти",
        "введи",
        "введіть",
        "зроби", // for "зроби скріншот"
        "зробіть",
        "правий клік",
        "подвійний клік",
        "згорн",   // згорни/згорніть
        "розгорн", // розгорни/розгорніть
        "ocr",     // already ascii but explicit
    ];

    if ASCII_NEEDLES.iter().any(|needle| lower.contains(needle)) {
        return true;
    }
    // For Cyrillic, use full Unicode lowercase so capital first letters
    // ("Робочий стіл", "Натисніть") still match the lowercase needles.
    let unicode_lower = prompt.to_lowercase();
    CYRILLIC_NEEDLES
        .iter()
        .any(|needle| unicode_lower.contains(needle))
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
        assert!(context.contains("dab_drag"));
        assert!(context.contains("dab_terminal_focus"));
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
    fn extended_keywords_trigger_context() {
        let config = DesktopAutomationContextConfig::default();
        let prompts = [
            // --- English ---
            "Open Outlook and click compose",
            "Drive Firefox to log into the dashboard",
            "Right click the system tray icon",
            "Drag the file into the Slack window",
            "Take a snapshot of the current state",
            "Type into the search box of the Settings app",
            "Use a keyboard shortcut to open the run dialog",
            "What's on the active window right now?",
            "Show me the taskbar",
            "Click in the system tray",
            "Automate the build pipeline through the UI",
            "Run this on Windows 11",
            // --- Ukrainian, ти-form ---
            "Перетягни цей файл — потрібен скріншот вікна",
            "Що зараз на екрані?",
            "Робочий стіл захаращений, прибери непотрібні іконки",
            "Зроби скріншот активного вікна",
            "Натисни кнопку у відкритому вікні",
            "Що у вкладці браузера зараз показано?",
            "Клікни в активному вікні Outlook",
            // --- Ukrainian, ви-form (user's preferred) ---
            "Зробіть скріншот вікна Edge",
            "Натисніть кнопку Save у застосунку",
            "Перетягніть файл у VS Code",
            "Введіть пароль у поле логіну",
            "Покажіть, що зараз у вікні Slack",
            "Згорніть вікно Outlook, розгорніть Teams",
            "Автоматизуйте надсилання листа з Outlook",
        ];
        for prompt in prompts {
            let result = desktop_automation_context_for_prompt_with_availability(
                config, prompt, /*desktop_tools_available*/ true,
            );
            assert!(
                result.is_some(),
                "expected desktop automation context for: {prompt:?}"
            );
        }
    }

    #[test]
    fn non_desktop_words_dont_falsely_trigger() {
        let config = DesktopAutomationContextConfig::default();
        let false_positives = [
            "Refactor the parser module",                   // baseline
            "Fix the edge case in the alignment math",      // "edge" alone — should NOT fire
            "The form factor of the device matters",        // "form" alone — not in needles
            "Tighten the input validation in the parser",   // "input" — not in needles
            "Document the menu structure of this language", // "menu" — only "menu bar" / "context menu" trigger
            "Compile the dialog grammar",                   // "dialog" — only "dialog box" triggers
            "Press release announcement for the v2 launch", // "press" — only "press key" triggers
            "Add a button to the button shop",              // "button" — not in needles
        ];
        for prompt in false_positives {
            let result = desktop_automation_context_for_prompt_with_availability(
                config, prompt, /*desktop_tools_available*/ true,
            );
            assert!(
                result.is_none(),
                "did NOT expect desktop automation context for: {prompt:?}"
            );
        }
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
