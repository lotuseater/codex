mod context;
mod harness;

mod windows;

pub use context::DesktopAutomationContextConfig;
pub use context::desktop_automation_context_for_prompt;
pub use context::merge_desktop_automation_context;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use thiserror::Error;

pub const AUTOMATION_HARNESS_DETECT_TOOL: &str = "automation_harness_detect";
pub const DAB_FIND_WINDOW_TOOL: &str = "dab_find_window";
pub const DAB_WINDOW_CHECK_TOOL: &str = "dab_window_check";
pub const DAB_PREPARE_WINDOW_TOOL: &str = "dab_prepare_window";
pub const DAB_SCREENSHOT_TOOL: &str = "dab_screenshot";
pub const DAB_LOCATE_VISUAL_TOOL: &str = "dab_locate_visual";
pub const DAB_OCR_TOOL: &str = "dab_ocr";
pub const DAB_VISUAL_SCAN_TOOL: &str = "dab_visual_scan";
pub const DAB_ELEMENT_MAP_TOOL: &str = "dab_element_map";
pub const DAB_NAVIGATE_TOOL: &str = "dab_navigate";
pub const DAB_SMART_CLICK_TOOL: &str = "dab_smart_click";
pub const DAB_CLICK_TOOL: &str = "dab_click";
pub const DAB_DRAG_TOOL: &str = "dab_drag";
pub const DAB_SCROLL_TOOL: &str = "dab_scroll";
pub const DAB_BG_CLICK_TOOL: &str = "dab_bg_click";
pub const DAB_SEND_KEYS_TOOL: &str = "dab_send_keys";
pub const DAB_TERMINAL_TABS_TOOL: &str = "dab_terminal_tabs";
pub const DAB_TERMINAL_FOCUS_TOOL: &str = "dab_terminal_focus";

pub const DAB_TOOL_NAMES: &[&str] = &[
    AUTOMATION_HARNESS_DETECT_TOOL,
    DAB_FIND_WINDOW_TOOL,
    DAB_WINDOW_CHECK_TOOL,
    DAB_PREPARE_WINDOW_TOOL,
    DAB_SCREENSHOT_TOOL,
    DAB_LOCATE_VISUAL_TOOL,
    DAB_OCR_TOOL,
    DAB_VISUAL_SCAN_TOOL,
    DAB_ELEMENT_MAP_TOOL,
    DAB_TERMINAL_TABS_TOOL,
    DAB_NAVIGATE_TOOL,
    DAB_SMART_CLICK_TOOL,
    DAB_CLICK_TOOL,
    DAB_DRAG_TOOL,
    DAB_SCROLL_TOOL,
    DAB_BG_CLICK_TOOL,
    DAB_SEND_KEYS_TOOL,
    DAB_TERMINAL_FOCUS_TOOL,
];

#[derive(Debug, Error)]
pub enum DesktopAutomationError {
    #[error("{0}")]
    Unsupported(String),
    #[error("failed to start desktop automation bridge: {0}")]
    Spawn(std::io::Error),
    #[error("desktop automation bridge timed out after {0} seconds")]
    Timeout(u64),
    #[error("desktop automation bridge failed: {0}")]
    Bridge(String),
    #[error("desktop automation bridge returned invalid JSON: {0}")]
    InvalidJson(serde_json::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct DesktopAutomationResult {
    pub ok: bool,
    pub output: Value,
    pub image_url: Option<String>,
}

impl DesktopAutomationResult {
    pub fn text(output: Value) -> Self {
        Self {
            ok: output.get("ok").and_then(Value::as_bool).unwrap_or(true),
            output,
            image_url: None,
        }
    }

    pub fn with_image(output: Value, image_url: Option<String>) -> Self {
        Self {
            ok: output.get("ok").and_then(Value::as_bool).unwrap_or(true),
            output,
            image_url,
        }
    }
}

pub fn is_supported_tool(tool_name: &str) -> bool {
    DAB_TOOL_NAMES.contains(&tool_name)
}

pub fn is_mutating_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        DAB_PREPARE_WINDOW_TOOL
            | DAB_NAVIGATE_TOOL
            | DAB_SMART_CLICK_TOOL
            | DAB_CLICK_TOOL
            | DAB_DRAG_TOOL
            | DAB_SCROLL_TOOL
            | DAB_BG_CLICK_TOOL
            | DAB_SEND_KEYS_TOOL
            | DAB_TERMINAL_FOCUS_TOOL
    )
}

pub fn text_output_value(value: &Value) -> Value {
    let mut output = value.clone();
    remove_image_urls(&mut output);
    output
}

fn remove_image_urls(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("image_url");
            for value in map.values_mut() {
                remove_image_urls(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                remove_image_urls(value);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

pub async fn execute_tool(
    tool_name: &str,
    input: Value,
    cwd: &Path,
) -> Result<DesktopAutomationResult, DesktopAutomationError> {
    match tool_name {
        AUTOMATION_HARNESS_DETECT_TOOL => Ok(harness::detect(input, cwd)),
        tool if is_supported_tool(tool) => windows::execute_dab_tool(tool, input).await,
        other => Err(DesktopAutomationError::Unsupported(format!(
            "unsupported desktop automation tool `{other}`"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn text_output_removes_embedded_image_urls() {
        let output = text_output_value(&json!({
            "ok": true,
            "image_url": "data:image/png;base64,abc",
            "screenshot": {
                "path": "shot.png",
                "image_url": "data:image/png;base64,def"
            },
            "elements": [
                {"name": "Save", "image_url": "data:image/png;base64,ghi"}
            ]
        }));

        assert_eq!(
            output,
            json!({
                "ok": true,
                "screenshot": {
                    "path": "shot.png"
                },
                "elements": [
                    {"name": "Save"}
                ]
            })
        );
    }
}
