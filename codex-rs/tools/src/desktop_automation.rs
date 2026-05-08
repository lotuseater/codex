use crate::ResponsesApiTool;
use crate::ToolSpec;
use codex_tool_schema::AdditionalProperties;
use codex_tool_schema::JsonSchema;
use serde_json::json;
use std::collections::BTreeMap;

const TARGETING: &str = "Target window selector. Prefer `hwnd` from dab_find_window/window_check. Otherwise use `title` or `process`.";

pub fn create_desktop_automation_tools(allow_input: bool) -> Vec<ToolSpec> {
    let mut tools = vec![
        create_tool(
            "automation_harness_detect",
            "Detect app-native GUI automation harnesses in a repo before using generic desktop automation.",
            object_schema(
                [
                    ("root", JsonSchema::string(Some("Directory to scan. Relative paths resolve against the current working directory.".to_string()))),
                    ("max_depth", JsonSchema::integer(Some("Maximum directory depth to scan. Defaults to 5.".to_string()))),
                    ("limit", JsonSchema::integer(Some("Maximum provider candidates to return. Defaults to 80.".to_string()))),
                ],
                &[],
            ),
        ),
        create_tool(
            "dab_find_window",
            "Find live Windows desktop windows by title substring, process name, or visibility.",
            target_schema([
                ("limit", JsonSchema::integer(Some("Maximum windows to return. Defaults to 30.".to_string()))),
                ("include_hidden", JsonSchema::boolean(Some("Include hidden windows. Defaults to false.".to_string()))),
            ]),
        ),
        create_tool(
            "dab_window_check",
            "Validate that a target window still exists and return its current title, process, visibility, and rectangle.",
            target_schema([]),
        ),
        create_tool(
            "dab_screenshot",
            "Capture a screenshot of a target window, or the primary screen when no target is supplied. Returns image content when the model supports images.",
            target_schema([
                ("path", JsonSchema::string(Some("Optional PNG output path. Defaults to a temporary file.".to_string()))),
                ("embed_image", JsonSchema::boolean(Some("Set false to save the screenshot path without embedding base64 image data.".to_string()))),
            ]),
        ),
        create_tool(
            "dab_ocr",
            "Extract visible text from a target window using UI Automation metadata, with an optional screenshot for visual context.",
            target_schema([
                ("path", JsonSchema::string(Some("Optional PNG output path for the screenshot.".to_string()))),
                ("screenshot", JsonSchema::boolean(Some("Set false to skip screenshot and return only text metadata.".to_string()))),
                ("embed_image", JsonSchema::boolean(Some("Set false to save screenshot path without embedding base64 image data.".to_string()))),
                ("max_elements", JsonSchema::integer(Some("Maximum UI Automation elements to inspect. Defaults to 120.".to_string()))),
            ]),
        ),
        create_tool(
            "dab_visual_scan",
            "Inspect a target window with screenshot and Windows UI Automation text/element metadata. Use before GUI decisions or after GUI actions.",
            target_schema([
                ("path", JsonSchema::string(Some("Optional PNG output path for the screenshot.".to_string()))),
                ("screenshot", JsonSchema::boolean(Some("Set false to skip screenshot and return only UI Automation metadata.".to_string()))),
                ("embed_image", JsonSchema::boolean(Some("Set false to save screenshot path without embedding base64 image data.".to_string()))),
                ("max_elements", JsonSchema::integer(Some("Maximum UI Automation elements to return. Defaults to 80.".to_string()))),
            ]),
        ),
        create_tool(
            "dab_element_map",
            "Return visible UI Automation elements for a target window, including text, automation id, control type, and coordinates.",
            target_schema([
                ("max_elements", JsonSchema::integer(Some("Maximum UI Automation elements to return. Defaults to 80.".to_string()))),
            ]),
        ),
        create_tool(
            "dab_terminal_tabs",
            "List visible Windows Terminal tabs and current tab text for a target terminal window.",
            target_schema([
                ("max_elements", JsonSchema::integer(Some("Maximum UI Automation elements to inspect. Defaults to 300.".to_string()))),
            ]),
        ),
    ];

    if allow_input {
        tools.extend([
            create_tool(
                "dab_navigate",
                "Send a named navigation shortcut to a target window, such as command_palette, find, save, open, new_tab, close_tab, address_bar, or terminal.",
                target_schema([
                    ("destination", JsonSchema::string(Some("Named navigation target or SendKeys payload.".to_string()))),
                ]),
            ),
            create_tool(
                "dab_smart_click",
                "Click a UI Automation element in a target window by visible text or automation id. Inspect first when unsure.",
                target_schema([
                    ("text", JsonSchema::string(Some("Visible text or automation id to click.".to_string()))),
                ]),
            ),
            create_tool(
                "dab_click",
                "Click absolute screen coordinates. Prefer dab_smart_click or element coordinates from dab_element_map when available.",
                target_schema([
                    ("x", JsonSchema::integer(Some("Absolute screen X coordinate.".to_string()))),
                    ("y", JsonSchema::integer(Some("Absolute screen Y coordinate.".to_string()))),
                ]),
            ),
            create_tool(
                "dab_drag",
                "Drag the foreground mouse between absolute screen coordinates. Inspect first and target a specific window when possible.",
                target_schema_with_required(
                    [
                        ("x", JsonSchema::integer(Some("Start absolute screen X coordinate.".to_string()))),
                        ("y", JsonSchema::integer(Some("Start absolute screen Y coordinate.".to_string()))),
                        ("end_x", JsonSchema::integer(Some("End absolute screen X coordinate.".to_string()))),
                        ("end_y", JsonSchema::integer(Some("End absolute screen Y coordinate.".to_string()))),
                        ("duration_ms", JsonSchema::integer(Some("Optional drag duration in milliseconds. Defaults to 350.".to_string()))),
                        ("steps", JsonSchema::integer(Some("Optional number of intermediate cursor steps. Defaults to 16.".to_string()))),
                    ],
                    &["x", "y", "end_x", "end_y"],
                ),
            ),
            create_tool(
                "dab_scroll",
                "Scroll the foreground mouse wheel at optional absolute screen coordinates. Inspect first and target a specific window when possible.",
                target_schema([
                    ("x", JsonSchema::integer(Some("Optional absolute screen X coordinate to move to before scrolling.".to_string()))),
                    ("y", JsonSchema::integer(Some("Optional absolute screen Y coordinate to move to before scrolling.".to_string()))),
                    ("amount", JsonSchema::integer(Some("Mouse wheel delta. Negative scrolls down, positive scrolls up. Defaults to -120.".to_string()))),
                ]),
            ),
            create_tool(
                "dab_bg_click",
                "Post a background click to a target window at absolute screen coordinates translated to client coordinates.",
                target_schema([
                    ("x", JsonSchema::integer(Some("Absolute screen X coordinate.".to_string()))),
                    ("y", JsonSchema::integer(Some("Absolute screen Y coordinate.".to_string()))),
                ]),
            ),
            create_tool(
                "dab_send_keys",
                "Send Windows SendKeys text or key chords to a target window after bringing it foreground.",
                target_schema([
                    ("keys", JsonSchema::string(Some("SendKeys payload, for example `hello`, `{ENTER}`, or `^l`.".to_string()))),
                ]),
            ),
            create_tool(
                "dab_terminal_focus",
                "Focus a Windows Terminal window or tab by tab title substring, index, or visible content.",
                target_schema([
                    ("tab_title", JsonSchema::string(Some("Case-insensitive terminal tab title substring to focus.".to_string()))),
                    ("index", JsonSchema::integer(Some("Zero-based terminal tab index to focus.".to_string()))),
                    ("text", JsonSchema::string(Some("Visible terminal content substring to use as a focus hint.".to_string()))),
                    ("max_elements", JsonSchema::integer(Some("Maximum UI Automation elements to inspect. Defaults to 300.".to_string()))),
                ]),
            ),
        ]);
    }

    tools
}

fn create_tool(name: &str, description: &str, parameters: JsonSchema) -> ToolSpec {
    ToolSpec::Function(ResponsesApiTool {
        name: name.to_string(),
        description: description.to_string(),
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: Some(json!({
            "type": "object",
            "properties": {
                "ok": { "type": "boolean" }
            },
            "required": ["ok"],
            "additionalProperties": true
        })),
    })
}

fn target_schema<const N: usize>(extra: [(&'static str, JsonSchema); N]) -> JsonSchema {
    target_schema_with_required(extra, &[])
}

fn target_schema_with_required<const N: usize>(
    extra: [(&'static str, JsonSchema); N],
    required: &[&str],
) -> JsonSchema {
    let mut fields = BTreeMap::from([
        (
            "hwnd".to_string(),
            JsonSchema::string(Some(TARGETING.to_string())),
        ),
        (
            "title".to_string(),
            JsonSchema::string(Some("Case-insensitive title substring.".to_string())),
        ),
        (
            "process".to_string(),
            JsonSchema::string(Some("Case-insensitive process name substring.".to_string())),
        ),
    ]);
    for (name, schema) in extra {
        fields.insert(name.to_string(), schema);
    }
    JsonSchema::object(
        fields,
        (!required.is_empty()).then(|| required.iter().map(|item| (*item).to_string()).collect()),
        Some(AdditionalProperties::Boolean(false)),
    )
}

fn object_schema<const N: usize>(
    fields: [(&'static str, JsonSchema); N],
    required: &[&str],
) -> JsonSchema {
    JsonSchema::object(
        fields
            .into_iter()
            .map(|(name, schema)| (name.to_string(), schema))
            .collect(),
        (!required.is_empty()).then(|| required.iter().map(|item| (*item).to_string()).collect()),
        Some(AdditionalProperties::Boolean(false)),
    )
}
