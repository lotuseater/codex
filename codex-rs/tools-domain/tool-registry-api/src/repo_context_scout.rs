use crate::ResponsesApiTool;
use crate::ToolSpec;
use codex_tool_schema::AdditionalProperties;
use codex_tool_schema::JsonSchema;
use serde_json::json;
use std::collections::BTreeMap;

pub const REPO_CONTEXT_SCOUT_TOOL_NAME: &str = "repo_context_scout";

pub fn create_repo_context_scout_tool() -> ToolSpec {
    ToolSpec::Function(ResponsesApiTool {
        name: REPO_CONTEXT_SCOUT_TOOL_NAME.to_string(),
        description: "Build or query a bounded repo context scout packet with changed-area and path-ranked hints.".to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            BTreeMap::from([
                (
                    "project_root".to_string(),
                    JsonSchema::string(Some(
                        "Repo root to inspect. Defaults to the current working directory."
                            .to_string(),
                    )),
                ),
                (
                    "prompt".to_string(),
                    JsonSchema::string(Some(
                        "Task prompt used to rank candidate files.".to_string(),
                    )),
                ),
                (
                    "max_tokens".to_string(),
                    JsonSchema::integer(Some(
                        "Maximum approximate tokens in the scout packet.".to_string(),
                    )),
                ),
                (
                    "mode".to_string(),
                    JsonSchema::string_enum(
                        vec![json!("scout"), json!("status"), json!("refresh")],
                        Some("Operation mode. Defaults to scout.".to_string()),
                    ),
                ),
            ]),
            None,
            Some(AdditionalProperties::Boolean(false)),
        ),
        output_schema: Some(json!({
            "type": "object",
            "additionalProperties": true
        })),
    })
}
