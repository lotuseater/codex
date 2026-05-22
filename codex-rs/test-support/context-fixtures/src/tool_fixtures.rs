use codex_tool_execution_api::JsonToolOutput;
use codex_tool_registry_api::JsonSchema;
use codex_tool_registry_api::ResponsesApiTool;
use serde_json::Value;

pub fn deferred_responses_api_tool(name: impl Into<String>) -> ResponsesApiTool {
    ResponsesApiTool {
        name: name.into(),
        description: String::new(),
        strict: false,
        defer_loading: Some(true),
        parameters: JsonSchema::object(
            /*properties*/ Default::default(),
            /*required*/ None,
            /*additional_properties*/ None,
        ),
        output_schema: None,
    }
}

pub fn json_tool_output(value: Value) -> JsonToolOutput {
    JsonToolOutput::new(value)
}
