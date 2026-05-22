#![cfg(not(target_os = "windows"))]
#![allow(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]

use anyhow::Result;
use codex_config::types::McpServerConfig;
use codex_config::types::McpServerTransportConfig;
use codex_core_test_runtime::apps_test_server::AppsTestServer;
use codex_core_test_runtime::apps_test_server::CALENDAR_CREATE_EVENT_MCP_APP_RESOURCE_URI;
use codex_core_test_runtime::apps_test_server::CALENDAR_CREATE_EVENT_RESOURCE_URI;
use codex_core_test_runtime::apps_test_server::DIRECT_CALENDAR_CREATE_EVENT_TOOL as CALENDAR_CREATE_TOOL;
use codex_core_test_runtime::apps_test_server::DIRECT_CALENDAR_LIST_EVENTS_TOOL as CALENDAR_LIST_TOOL;
use codex_core_test_runtime::apps_test_server::SEARCH_CALENDAR_CREATE_TOOL;
use codex_core_test_runtime::apps_test_server::SEARCH_CALENDAR_LIST_TOOL;
use codex_core_test_runtime::apps_test_server::SEARCH_CALENDAR_NAMESPACE;
use codex_core_test_runtime::apps_test_server::configure_search_capable_apps;
use codex_core_test_runtime::apps_test_server::configure_search_capable_model;
use codex_core_test_runtime::apps_test_server::recorded_apps_tool_call_by_call_id;
use codex_core_test_runtime::apps_test_server::search_capable_apps_builder as configured_builder;
use codex_core_test_runtime::responses::ResponsesRequest;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_function_call_with_namespace;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::ev_tool_search_call;
use codex_core_test_runtime::responses::mount_sse_once;
use codex_core_test_runtime::responses::mount_sse_sequence;
use codex_core_test_runtime::responses::namespace_child_tool;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::responses::start_mock_server;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::stdio_server_bin;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::wait_for_event;
use codex_features::Feature;
use codex_login::CodexAuth;
use codex_protocol::dynamic_tools::DynamicToolCallOutputContentItem;
use codex_protocol::dynamic_tools::DynamicToolResponse;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::McpInvocation;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

const SEARCH_TOOL_DESCRIPTION_SNIPPETS: [&str; 2] = [
    "You have access to tools from the following sources",
    "- Calendar: Plan events and manage your calendar.",
];
const TOOL_SEARCH_TOOL_NAME: &str = "tool_search";

fn tool_names(body: &Value) -> Vec<String> {
    body.get("tools")
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| {
                    tool.get("name")
                        .or_else(|| tool.get("type"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn tool_search_description(body: &Value) -> Option<String> {
    body.get("tools")
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter().find_map(|tool| {
                if tool.get("type").and_then(Value::as_str) == Some(TOOL_SEARCH_TOOL_NAME) {
                    tool.get("description")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                } else {
                    None
                }
            })
        })
}

fn tool_search_output_item(request: &ResponsesRequest, call_id: &str) -> Value {
    request.tool_search_output(call_id)
}

fn tool_search_output_tools(request: &ResponsesRequest, call_id: &str) -> Vec<Value> {
    tool_search_output_item(request, call_id)
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn tool_search_output_has_namespace_child(
    request: &ResponsesRequest,
    call_id: &str,
    namespace: &str,
    tool_name: &str,
) -> bool {
    let output = json!({
        "tools": tool_search_output_tools(request, call_id),
    });
    namespace_child_tool(&output, namespace, tool_name).is_some()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_returns_deferred_tools_without_follow_up_tool_injection() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let call_id = "tool-search-1";
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_tool_search_call(
                    call_id,
                    &json!({
                        "query": "create calendar event",
                        "limit": 1,
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
                        "call_id": "calendar-call-1",
                        "name": SEARCH_CALENDAR_CREATE_TOOL,
                        "namespace": SEARCH_CALENDAR_NAMESPACE,
                        "arguments": serde_json::to_string(&json!({
                            "title": "Lunch",
                            "starts_at": "2026-03-10T12:00:00Z"
                        })).expect("serialize calendar args")
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

    let mut builder = configured_builder(apps_server.chatgpt_base_url.clone());
    let test = builder.build(&server).await?;
    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "Find the calendar create tool".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let EventMsg::McpToolCallBegin(begin) = wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::McpToolCallBegin(_))
    })
    .await
    else {
        unreachable!("event guard guarantees McpToolCallBegin");
    };
    assert_eq!(begin.call_id, "calendar-call-1");
    assert_eq!(
        begin.mcp_app_resource_uri.as_deref(),
        Some(CALENDAR_CREATE_EVENT_MCP_APP_RESOURCE_URI)
    );

    let EventMsg::McpToolCallEnd(end) = wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::McpToolCallEnd(_))
    })
    .await
    else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };
    assert_eq!(end.call_id, "calendar-call-1");
    assert_eq!(
        end.mcp_app_resource_uri.as_deref(),
        Some(CALENDAR_CREATE_EVENT_MCP_APP_RESOURCE_URI)
    );
    assert_eq!(
        end.invocation,
        McpInvocation {
            server: "codex_apps".to_string(),
            tool: "calendar_create_event".to_string(),
            arguments: Some(json!({
                "title": "Lunch",
                "starts_at": "2026-03-10T12:00:00Z"
            })),
        }
    );
    assert_eq!(
        end.result
            .as_ref()
            .expect("tool call should succeed")
            .structured_content,
        Some(json!({
            "_codex_apps": {
                "call_id": "calendar-call-1",
                "resource_uri": CALENDAR_CREATE_EVENT_RESOURCE_URI,
                "contains_mcp_source": true,
                "connector_id": "calendar",
            },
        }))
    );

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = mock.requests();
    assert_eq!(requests.len(), 3);
    let first_request_body = requests[0].body_json();

    let apps_tool_call = recorded_apps_tool_call_by_call_id(&server, "calendar-call-1").await;

    assert_eq!(
        apps_tool_call.pointer("/params/_meta/_codex_apps"),
        Some(&json!({
            "call_id": "calendar-call-1",
            "resource_uri": CALENDAR_CREATE_EVENT_RESOURCE_URI,
            "contains_mcp_source": true,
            "connector_id": "calendar",
        }))
    );
    assert_eq!(
        apps_tool_call.pointer("/params/_meta/x-codex-turn-metadata/session_id"),
        Some(&json!(test.session_configured.session_id.to_string()))
    );
    assert_eq!(
        apps_tool_call.pointer("/params/_meta/x-codex-turn-metadata/thread_id"),
        Some(&json!(test.session_configured.thread_id.to_string()))
    );
    assert!(
        apps_tool_call
            .pointer("/params/_meta/x-codex-turn-metadata/turn_id")
            .and_then(Value::as_str)
            .is_some_and(|turn_id| !turn_id.is_empty()),
        "apps tools/call should include turn metadata turn_id: {apps_tool_call:?}"
    );
    assert_eq!(
        apps_tool_call
            .pointer("/params/_meta/x-codex-turn-metadata/model")
            .and_then(Value::as_str),
        Some("gpt-5.4")
    );
    let first_request_reasoning_effort = first_request_body
        .pointer("/reasoning/effort")
        .and_then(Value::as_str)
        .expect("first response request should include reasoning effort");
    assert_eq!(
        apps_tool_call
            .pointer("/params/_meta/x-codex-turn-metadata/reasoning_effort")
            .and_then(Value::as_str),
        Some(first_request_reasoning_effort)
    );
    let mcp_turn_started_at_unix_ms = apps_tool_call
        .pointer("/params/_meta/x-codex-turn-metadata/turn_started_at_unix_ms")
        .and_then(Value::as_i64)
        .expect("apps tools/call should include turn_started_at_unix_ms");
    assert!(
        mcp_turn_started_at_unix_ms > 0,
        "apps tools/call should include a positive turn_started_at_unix_ms: {apps_tool_call:?}"
    );

    let first_request_turn_metadata: Value = serde_json::from_str(
        &requests[0]
            .header("x-codex-turn-metadata")
            .expect("first response request should include turn metadata"),
    )
    .expect("first response request turn metadata should be valid JSON");
    assert_eq!(
        first_request_turn_metadata
            .get("turn_started_at_unix_ms")
            .and_then(Value::as_i64),
        Some(mcp_turn_started_at_unix_ms)
    );

    let first_request_tools = tool_names(&first_request_body);
    assert!(
        first_request_tools
            .iter()
            .any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "first request should advertise tool_search: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools
            .iter()
            .any(|name| name == CALENDAR_CREATE_TOOL),
        "app tools should still be hidden before search: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools
            .iter()
            .any(|name| name == SEARCH_CALENDAR_NAMESPACE),
        "app namespace should still be hidden before search: {first_request_tools:?}"
    );

    let output_item = tool_search_output_item(&requests[1], call_id);
    assert_eq!(
        output_item.get("status").and_then(Value::as_str),
        Some("completed")
    );
    assert_eq!(
        output_item.get("execution").and_then(Value::as_str),
        Some("client")
    );

    let tools = tool_search_output_tools(&requests[1], call_id);
    assert_eq!(
        tools,
        vec![json!({
            "type": "namespace",
            "name": SEARCH_CALENDAR_NAMESPACE,
            "description": "Plan events and manage your calendar.",
            "tools": [
                {
                    "type": "function",
                    "name": SEARCH_CALENDAR_CREATE_TOOL,
                    "description": "Create a calendar event.",
                    "strict": false,
                    "defer_loading": true,
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "starts_at": {"type": "string"},
                            "timezone": {"type": "string"},
                            "title": {"type": "string"},
                        },
                        "required": ["title", "starts_at"],
                        "additionalProperties": false,
                    }
                }
            ]
        })]
    );

    let second_request_tools = tool_names(&requests[1].body_json());
    assert!(
        !second_request_tools
            .iter()
            .any(|name| name == CALENDAR_CREATE_TOOL),
        "follow-up request should rely on tool_search_output history, not tool injection: {second_request_tools:?}"
    );
    assert!(
        !second_request_tools
            .iter()
            .any(|name| name == SEARCH_CALENDAR_NAMESPACE),
        "follow-up request should rely on tool_search_output history, not namespace injection: {second_request_tools:?}"
    );

    let output_item = requests[2].function_call_output("calendar-call-1");
    assert_eq!(
        output_item.get("call_id").and_then(Value::as_str),
        Some("calendar-call-1")
    );

    let third_request_tools = tool_names(&requests[2].body_json());
    assert!(
        !third_request_tools
            .iter()
            .any(|name| name == CALENDAR_CREATE_TOOL),
        "post-tool follow-up should still rely on tool_search_output history, not tool injection: {third_request_tools:?}"
    );
    assert!(
        !third_request_tools
            .iter()
            .any(|name| name == SEARCH_CALENDAR_NAMESPACE),
        "post-tool follow-up should still rely on tool_search_output history, not namespace injection: {third_request_tools:?}"
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_returns_deferred_v1_multi_agent_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "tool-search-spawn-agent";
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_tool_search_call(
                    call_id,
                    &json!({
                        "query": "spawn agent",
                        "limit": 1,
                    }),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex().with_config(configure_search_capable_model);
    let test = builder.build(&server).await?;
    test.submit_turn_with_approval_and_permission_profile(
        "Find the spawn agent tool",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = mock.requests();
    assert_eq!(requests.len(), 2);

    let first_request_body = requests[0].body_json();
    let first_request_tools = tool_names(&first_request_body);
    assert!(
        first_request_tools
            .iter()
            .any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "first request should advertise tool_search: {first_request_tools:?}"
    );
    for tool_name in [
        "spawn_agent",
        "send_input",
        "resume_agent",
        "wait_agent",
        "close_agent",
    ] {
        assert!(
            !first_request_tools.iter().any(|name| name == tool_name),
            "v1 multi-agent tools should be hidden before search: {first_request_tools:?}"
        );
    }
    assert!(
        !first_request_body
            .to_string()
            .contains("Only use `spawn_agent` if and only if"),
        "deferred v1 multi-agent guidance should stay out of initial developer context"
    );

    let tools = tool_search_output_tools(&requests[1], call_id);
    assert!(
        !tools.iter().any(|tool| {
            tool.get("type").and_then(Value::as_str) == Some("function")
                && tool.get("name").and_then(Value::as_str) == Some("spawn_agent")
        }),
        "spawn_agent should be returned as a namespace child, not a flat function: {tools:?}"
    );
    assert!(
        tools.iter().any(|tool| {
            tool.get("type").and_then(Value::as_str) == Some("namespace")
                && tool.get("name").and_then(Value::as_str) == Some("multi_agent_v1")
        }),
        "expected tool_search to return multi_agent_v1 namespace: {tools:?}"
    );
    let output = tool_search_output_item(&requests[1], call_id);
    let spawn_agent = namespace_child_tool(&output, "multi_agent_v1", "spawn_agent")
        .unwrap_or_else(|| {
            panic!("expected tool_search to return multi_agent_v1.spawn_agent: {output:?}")
        });
    assert_eq!(
        spawn_agent.get("defer_loading").and_then(Value::as_bool),
        Some(true)
    );
    let description = spawn_agent
        .get("description")
        .and_then(Value::as_str)
        .expect("spawn_agent description should be present");
    assert!(description.contains("Only use `spawn_agent` if and only if"));
    assert!(description.contains("### Designing delegated subtasks"));

    Ok(())
}
