use super::*;
use pretty_assertions::assert_eq;

#[test]
fn codex_error_info_serializes_http_status_code_in_camel_case() {
    let value = CodexErrorInfo::ResponseTooManyFailedAttempts {
        http_status_code: Some(401),
    };

    assert_eq!(
        serde_json::to_value(value).unwrap(),
        json!({
            "responseTooManyFailedAttempts": {
                "httpStatusCode": 401
            }
        })
    );
}

#[test]
fn codex_error_info_serializes_cyber_policy_in_camel_case() {
    assert_eq!(
        serde_json::to_value(CodexErrorInfo::CyberPolicy).unwrap(),
        json!("cyberPolicy")
    );
}

#[test]
fn codex_error_info_serializes_active_turn_not_steerable_turn_kind_in_camel_case() {
    let value = CodexErrorInfo::ActiveTurnNotSteerable {
        turn_kind: NonSteerableTurnKind::Review,
    };

    assert_eq!(
        serde_json::to_value(value).unwrap(),
        json!({
            "activeTurnNotSteerable": {
                "turnKind": "review"
            }
        })
    );
}

#[test]
fn dynamic_tool_response_serializes_content_items() {
    let value = serde_json::to_value(DynamicToolCallResponse {
        content_items: vec![DynamicToolCallOutputContentItem::InputText {
            text: "dynamic-ok".to_string(),
        }],
        success: true,
    })
    .unwrap();

    assert_eq!(
        value,
        json!({
            "contentItems": [
                {
                    "type": "inputText",
                    "text": "dynamic-ok"
                }
            ],
            "success": true,
        })
    );
}

#[test]
fn dynamic_tool_response_serializes_text_and_image_content_items() {
    let value = serde_json::to_value(DynamicToolCallResponse {
        content_items: vec![
            DynamicToolCallOutputContentItem::InputText {
                text: "dynamic-ok".to_string(),
            },
            DynamicToolCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,AAA".to_string(),
            },
        ],
        success: true,
    })
    .unwrap();

    assert_eq!(
        value,
        json!({
            "contentItems": [
                {
                    "type": "inputText",
                    "text": "dynamic-ok"
                },
                {
                    "type": "inputImage",
                    "imageUrl": "data:image/png;base64,AAA"
                }
            ],
            "success": true,
        })
    );
}

#[test]
fn dynamic_tool_spec_deserializes_defer_loading() {
    let value = json!({
        "name": "lookup_ticket",
        "description": "Fetch a ticket",
        "inputSchema": {
            "type": "object",
            "properties": {
                "id": { "type": "string" }
            }
        },
        "deferLoading": true,
    });

    let actual: DynamicToolSpec = serde_json::from_value(value).expect("deserialize");

    assert_eq!(
        actual,
        DynamicToolSpec {
            namespace: None,
            name: "lookup_ticket".to_string(),
            description: "Fetch a ticket".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" }
                }
            }),
            defer_loading: true,
        }
    );
}

#[test]
fn dynamic_tool_spec_legacy_expose_to_context_inverts_to_defer_loading() {
    let value = json!({
        "name": "lookup_ticket",
        "description": "Fetch a ticket",
        "inputSchema": {
            "type": "object",
            "properties": {}
        },
        "exposeToContext": false,
    });

    let actual: DynamicToolSpec = serde_json::from_value(value).expect("deserialize");

    assert!(actual.defer_loading);
}
