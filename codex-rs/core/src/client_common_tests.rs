use codex_api::OpenAiVerbosity;
use codex_api::ResponsesApiRequest;
use codex_api::TextControls;
use codex_api::create_text_param_for_request;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ImageDetail;
use pretty_assertions::assert_eq;

use super::*;

fn prompt_with_image_outputs() -> Prompt {
    Prompt {
        input: vec![
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputImage {
                    image_url: "https://example.com/image.png".to_string(),
                    detail: Some(ImageDetail::Original),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::FunctionCallOutput {
                id: None,
                call_id: "function-call".to_string(),
                output: FunctionCallOutputPayload::from_content_items(vec![
                    FunctionCallOutputContentItem::InputImage {
                        image_url: "data:image/png;base64,function".to_string(),
                        detail: Some(ImageDetail::High),
                    },
                ]),
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::CustomToolCallOutput {
                id: None,
                call_id: "custom-call".to_string(),
                name: None,
                output: FunctionCallOutputPayload::from_content_items(vec![
                    FunctionCallOutputContentItem::InputImage {
                        image_url: "data:image/png;base64,custom".to_string(),
                        detail: Some(ImageDetail::Auto),
                    },
                ]),
                internal_chat_message_metadata_passthrough: None,
            },
        ],
        ..Default::default()
    }
}

#[test]
fn responses_lite_request_copies_strip_image_details() {
    let prompt = prompt_with_image_outputs();
    let original = prompt.input.clone();

    let stripped = prompt.get_formatted_input_for_request(/*use_responses_lite*/ true);

    assert_eq!(
        stripped,
        vec![
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputImage {
                    image_url: "https://example.com/image.png".to_string(),
                    detail: None,
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::FunctionCallOutput {
                id: None,
                call_id: "function-call".to_string(),
                output: FunctionCallOutputPayload::from_content_items(vec![
                    FunctionCallOutputContentItem::InputImage {
                        image_url: "data:image/png;base64,function".to_string(),
                        detail: None,
                    },
                ]),
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::CustomToolCallOutput {
                id: None,
                call_id: "custom-call".to_string(),
                name: None,
                output: FunctionCallOutputPayload::from_content_items(vec![
                    FunctionCallOutputContentItem::InputImage {
                        image_url: "data:image/png;base64,custom".to_string(),
                        detail: None,
                    },
                ]),
                internal_chat_message_metadata_passthrough: None,
            },
        ]
    );
    assert_eq!(prompt.input, original);
    assert_eq!(
        prompt.get_formatted_input_for_request(/*use_responses_lite*/ false),
        original
    );
}

#[test]
fn serializes_text_verbosity_when_set() {
    let input: Vec<ResponseItem> = vec![];
    let tools: Vec<serde_json::Value> = vec![];
    let req = ResponsesApiRequest {
        model: "gpt-5.4".to_string(),
        instructions: "i".to_string(),
        input,
        tools: Some(tools),
        tool_choice: "auto".to_string(),
        parallel_tool_calls: true,
        reasoning: None,
        store: false,
        stream: true,
        include: vec![],
        prompt_cache_key: None,
        service_tier: None,
        text: Some(TextControls {
            verbosity: Some(OpenAiVerbosity::Low),
            format: None,
        }),
        client_metadata: None,
    };

    let v = serde_json::to_value(&req).expect("json");
    assert_eq!(
        v.get("text")
            .and_then(|t| t.get("verbosity"))
            .and_then(|s| s.as_str()),
        Some("low")
    );
}

#[test]
fn serializes_text_schema_with_strict_format() {
    let input: Vec<ResponseItem> = vec![];
    let tools: Vec<serde_json::Value> = vec![];
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "answer": {"type": "string"}
        },
        "required": ["answer"],
    });
    let text_controls = create_text_param_for_request(
        /*verbosity*/ None,
        &Some(schema.clone()),
        /*output_schema_strict*/ true,
    )
    .expect("text controls");

    let req = ResponsesApiRequest {
        model: "gpt-5.4".to_string(),
        instructions: "i".to_string(),
        input,
        tools: Some(tools),
        tool_choice: "auto".to_string(),
        parallel_tool_calls: true,
        reasoning: None,
        store: false,
        stream: true,
        include: vec![],
        prompt_cache_key: None,
        service_tier: None,
        text: Some(text_controls),
        client_metadata: None,
    };

    let v = serde_json::to_value(&req).expect("json");
    let text = v.get("text").expect("text field");
    assert!(text.get("verbosity").is_none());
    let format = text.get("format").expect("format field");

    assert_eq!(
        format.get("name"),
        Some(&serde_json::Value::String("codex_output_schema".into()))
    );
    assert_eq!(
        format.get("type"),
        Some(&serde_json::Value::String("json_schema".into()))
    );
    assert_eq!(format.get("strict"), Some(&serde_json::Value::Bool(true)));
    assert_eq!(format.get("schema"), Some(&schema));
}

#[test]
fn serializes_text_schema_with_non_strict_format() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "answer": {"type": "string"},
            "rationale": {"type": "string"}
        },
        "required": ["answer"],
        "additionalProperties": false
    });
    let text_controls = create_text_param_for_request(
        /*verbosity*/ None,
        &Some(schema.clone()),
        /*output_schema_strict*/ false,
    )
    .expect("text controls");

    let format = text_controls.format.expect("format field");
    assert!(!format.strict);
    assert_eq!(format.schema, schema);
}

#[test]
fn omits_text_when_not_set() {
    let input: Vec<ResponseItem> = vec![];
    let tools: Vec<serde_json::Value> = vec![];
    let req = ResponsesApiRequest {
        model: "gpt-5.4".to_string(),
        instructions: "i".to_string(),
        input,
        tools: Some(tools),
        tool_choice: "auto".to_string(),
        parallel_tool_calls: true,
        reasoning: None,
        store: false,
        stream: true,
        include: vec![],
        prompt_cache_key: None,
        service_tier: None,
        text: None,
        client_metadata: None,
    };

    let v = serde_json::to_value(&req).expect("json");
    assert!(v.get("text").is_none());
}

#[test]
fn serializes_flex_service_tier_when_set() {
    let req = ResponsesApiRequest {
        model: "gpt-5.4".to_string(),
        instructions: "i".to_string(),
        input: vec![],
        tools: Some(vec![]),
        tool_choice: "auto".to_string(),
        parallel_tool_calls: true,
        reasoning: None,
        store: false,
        stream: true,
        include: vec![],
        prompt_cache_key: None,
        service_tier: Some(ServiceTier::Flex.to_string()),
        text: None,
        client_metadata: None,
    };

    let v = serde_json::to_value(&req).expect("json");
    assert_eq!(
        v.get("service_tier").and_then(|tier| tier.as_str()),
        Some("flex")
    );
}

#[test]
fn reserializes_shell_outputs_for_function_and_custom_tool_calls() {
    let raw_output = r#"{"output":"hello","metadata":{"exit_code":0,"duration_seconds":0.5}}"#;
    let expected_output = "Exit code: 0\nWall time: 0.5 seconds\nOutput:\nhello";
    let mut items = vec![
        ResponseItem::FunctionCall {
            id: None,
            name: "shell".to_string(),
            namespace: None,
            arguments: "{}".to_string(),
            call_id: "call-1".to_string(),
        },
        ResponseItem::FunctionCallOutput {
            call_id: "call-1".to_string(),
            output: FunctionCallOutputPayload::from_text(raw_output.to_string()),
        },
        ResponseItem::CustomToolCall {
            id: None,
            status: None,
            call_id: "call-2".to_string(),
            name: "apply_patch".to_string(),
            input: "*** Begin Patch".to_string(),
        },
        ResponseItem::CustomToolCallOutput {
            call_id: "call-2".to_string(),
            name: None,
            output: FunctionCallOutputPayload::from_text(raw_output.to_string()),
        },
    ];

    reserialize_shell_outputs(&mut items);

    assert_eq!(
        items,
        vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "shell".to_string(),
                namespace: None,
                arguments: "{}".to_string(),
                call_id: "call-1".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-1".to_string(),
                output: FunctionCallOutputPayload::from_text(expected_output.to_string()),
            },
            ResponseItem::CustomToolCall {
                id: None,
                status: None,
                call_id: "call-2".to_string(),
                name: "apply_patch".to_string(),
                input: "*** Begin Patch".to_string(),
            },
            ResponseItem::CustomToolCallOutput {
                call_id: "call-2".to_string(),
                name: None,
                output: FunctionCallOutputPayload::from_text(expected_output.to_string()),
            },
        ]
    );
}

#[test]
fn sanitizes_invalid_tool_search_output_tool_types() {
    let prompt = Prompt {
        input: vec![ResponseItem::ToolSearchOutput {
            call_id: Some("call-1".to_string()),
            status: "completed".to_string(),
            execution: "client".to_string(),
            tools: vec![
                serde_json::json!({
                    "type": "prompt_reduction",
                    "name": "bad_history_tool",
                }),
                serde_json::json!({
                    "type": "function",
                    "name": "valid_tool",
                    "description": "valid",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                    },
                }),
            ],
        }],
        ..Default::default()
    };

    let formatted = prompt.get_formatted_input();

    let [ResponseItem::ToolSearchOutput { tools, .. }] = formatted.as_slice() else {
        panic!("expected tool search output");
    };
    assert_eq!(
        tools,
        &vec![serde_json::json!({
            "type": "function",
            "name": "valid_tool",
            "description": "valid",
            "parameters": {
                "type": "object",
                "properties": {},
            },
        })]
    );
}

#[test]
fn preserves_tool_search_outputs_with_no_valid_tool_types() {
    let prompt = Prompt {
        input: vec![
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: "keep me".to_string(),
                }],
                phase: None,
            },
            ResponseItem::ToolSearchOutput {
                call_id: Some("call-1".to_string()),
                status: "completed".to_string(),
                execution: "client".to_string(),
                tools: vec![serde_json::json!({
                    "type": "prompt_reduction",
                    "name": "bad_history_tool",
                })],
            },
        ],
        ..Default::default()
    };

    let formatted = prompt.get_formatted_input();

    assert_eq!(
        formatted,
        vec![
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: "keep me".to_string(),
                }],
                phase: None,
            },
            ResponseItem::ToolSearchOutput {
                call_id: Some("call-1".to_string()),
                status: "completed".to_string(),
                execution: "client".to_string(),
                tools: Vec::new(),
            },
        ]
    );
}

#[test]
fn preserves_tool_search_call_output_pair_after_sanitizing_reduced_output() {
    let prompt = Prompt {
        input: vec![
            ResponseItem::ToolSearchCall {
                id: None,
                call_id: Some("call-1".to_string()),
                status: None,
                execution: "client".to_string(),
                arguments: serde_json::json!({"query": "tool"}),
            },
            ResponseItem::ToolSearchOutput {
                call_id: Some("call-1".to_string()),
                status: "completed".to_string(),
                execution: "client".to_string(),
                tools: vec![serde_json::json!({
                    "type": "prompt_reduction",
                    "name": "reduced_tool_search_output",
                })],
            },
        ],
        ..Default::default()
    };

    let formatted = prompt.get_formatted_input();

    let [
        ResponseItem::ToolSearchCall { call_id, .. },
        ResponseItem::ToolSearchOutput {
            call_id: output_call_id,
            tools,
            ..
        },
    ] = formatted.as_slice()
    else {
        panic!("expected tool search call/output pair");
    };
    assert_eq!(call_id.as_deref(), Some("call-1"));
    assert_eq!(output_call_id.as_deref(), Some("call-1"));
    assert!(tools.is_empty());
}
