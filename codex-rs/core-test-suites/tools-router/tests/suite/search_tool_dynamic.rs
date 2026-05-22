#![cfg(not(target_os = "windows"))]
#![allow(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]

use crate::search_tool_support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_returns_deferred_dynamic_tool_and_routes_follow_up_call() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let search_call_id = "tool-search-1";
    let dynamic_call_id = "dyn-search-call-1";
    let tool_name = "automation_update";
    let tool_description = "Create, update, view, or delete recurring automations.";
    let tool_args = json!({ "mode": "create" });
    let tool_call_arguments = serde_json::to_string(&tool_args)?;
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_tool_search_call(
                    search_call_id,
                    &json!({
                        "query": "recurring automations",
                        "limit": 8,
                    }),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                json!({
                    "type": "response.output_item.done",
                    "item": {
                        "type": "function_call",
                        "call_id": dynamic_call_id,
                        "namespace": "codex_app",
                        "name": tool_name,
                        "arguments": tool_call_arguments,
                    }
                }),
                ev_completed("resp-2"),
            ]),
            sse(vec![
                ev_response_created("resp-3"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-3"),
            ]),
        ],
    )
    .await;

    let input_schema = json!({
        "type": "object",
        "properties": {
            "mode": { "type": "string" },
        },
        "required": ["mode"],
        "additionalProperties": false,
    });
    let dynamic_tool = DynamicToolSpec {
        namespace: Some("codex_app".to_string()),
        name: tool_name.to_string(),
        description: tool_description.to_string(),
        input_schema: input_schema.clone(),
        defer_loading: true,
    };

    let mut builder = test_codex().with_config(configure_search_capable_model);
    let base_test = builder.build(&server).await?;
    let new_thread = base_test
        .thread_manager
        .start_thread_with_tools(
            base_test.config.clone(),
            vec![dynamic_tool],
            /*persist_extended_history*/ false,
        )
        .await?;
    let mut test = base_test;
    test.codex = new_thread.thread;
    test.session_configured = new_thread.session_configured;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "Use the automation tool".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let EventMsg::DynamicToolCallRequest(request) = wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::DynamicToolCallRequest(_))
    })
    .await
    else {
        unreachable!("event guard guarantees DynamicToolCallRequest");
    };
    assert_eq!(request.call_id, dynamic_call_id);
    assert_eq!(request.namespace.as_deref(), Some("codex_app"));
    assert_eq!(request.tool, tool_name);
    assert_eq!(request.arguments, tool_args);

    test.codex
        .submit(Op::DynamicToolResponse {
            id: request.call_id,
            response: DynamicToolResponse {
                content_items: vec![DynamicToolCallOutputContentItem::InputText {
                    text: "dynamic-search-ok".to_string(),
                }],
                success: true,
            },
        })
        .await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = mock.requests();
    assert_eq!(requests.len(), 3);

    let first_request_body = requests[0].body_json();
    let first_request_tools = tool_names(&first_request_body);
    assert!(
        first_request_tools
            .iter()
            .any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "first request should advertise tool_search: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools.iter().any(|name| name == tool_name),
        "deferred dynamic tool should be hidden before search: {first_request_tools:?}"
    );

    let tools = tool_search_output_tools(&requests[1], search_call_id);
    assert_eq!(
        tools,
        vec![json!({
            "type": "namespace",
            "name": "codex_app",
            "description": "Tools in the codex_app namespace.",
            "tools": [{
                "type": "function",
                "name": tool_name,
                "description": tool_description,
                "strict": false,
                "defer_loading": true,
                "parameters": input_schema,
            }],
        })]
    );

    let second_request_body = requests[1].body_json();
    let second_request_tools = tool_names(&second_request_body);
    assert!(
        !second_request_tools.iter().any(|name| name == tool_name),
        "follow-up request should rely on tool_search_output history, not tool injection: {second_request_tools:?}"
    );

    let output = requests[2]
        .function_call_output(dynamic_call_id)
        .get("output")
        .cloned()
        .expect("dynamic tool output should be present");
    let payload: FunctionCallOutputPayload = serde_json::from_value(output)?;
    assert_eq!(
        payload,
        FunctionCallOutputPayload::from_text("dynamic-search-ok".to_string())
    );

    let third_request_body = requests[2].body_json();
    let third_request_tools = tool_names(&third_request_body);
    assert!(
        !third_request_tools.iter().any(|name| name == tool_name),
        "post-tool follow-up should rely on tool_search_output history, not tool injection: {third_request_tools:?}"
    );

    Ok(())
}
