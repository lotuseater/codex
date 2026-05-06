use crate::AdditionalProperties;
use crate::JsonSchema;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use serde_json::json;
use std::collections::BTreeMap;

pub const FIRST_MOVES_PREDICT_TOOL_NAME: &str = "first_moves_predict";
pub const FIRST_MOVES_STATS_TOOL_NAME: &str = "first_moves_stats";

pub fn create_first_moves_tools() -> Vec<ToolSpec> {
    vec![
        create_tool(
            FIRST_MOVES_PREDICT_TOOL_NAME,
            "Predict high-value first file reads and searches for a repo/task without running shell commands.",
            object_schema([
                (
                    "project_root",
                    JsonSchema::string(Some(
                        "Repo root to inspect. Defaults to the current working directory."
                            .to_string(),
                    )),
                ),
                (
                    "prompt",
                    JsonSchema::string(Some(
                        "Task prompt to score against. Defaults to an empty prompt.".to_string(),
                    )),
                ),
                (
                    "max_candidates",
                    JsonSchema::integer(Some(
                        "Maximum candidates to return for this call, capped at 50.".to_string(),
                    )),
                ),
            ]),
        ),
        create_tool(
            FIRST_MOVES_STATS_TOOL_NAME,
            "Report native first-moves prediction/hit telemetry for a repo.",
            object_schema([(
                "project_root",
                JsonSchema::string(Some(
                    "Repo root to inspect. Defaults to the current working directory.".to_string(),
                )),
            )]),
        ),
    ]
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
            "additionalProperties": true
        })),
    })
}

fn object_schema<const N: usize>(fields: [(&'static str, JsonSchema); N]) -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from_iter(
            fields
                .into_iter()
                .map(|(name, schema)| (name.to_string(), schema)),
        ),
        None,
        Some(AdditionalProperties::Boolean(false)),
    )
}
