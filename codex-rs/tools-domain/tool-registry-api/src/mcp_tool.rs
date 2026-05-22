use serde_json::Value as JsonValue;
use serde_json::json;

pub fn mcp_call_tool_result_output_schema(structured_content_schema: JsonValue) -> JsonValue {
    json!({
        "type": "object",
        "properties": {
            "content": {
                "type": "array",
                "items": {
                    "type": "object"
                }
            },
            "structuredContent": structured_content_schema,
            "isError": {
                "type": "boolean"
            },
            "_meta": {
                "type": "object"
            }
        },
        "required": ["content"],
        "additionalProperties": false
    })
}
