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
async fn tool_search_matches_mcp_tools_by_distinct_name_description_and_schema_terms() -> Result<()>
{
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let query_cases = [
        ("tool-search-mcp-raw-name", "calendar_timezone_option_99"),
        ("tool-search-mcp-description", "uploaded document"),
        ("tool-search-mcp-schema", "starts_at"),
    ];
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(std::iter::once(ev_response_created("resp-1"))
                .chain(query_cases.into_iter().map(|(call_id, query)| {
                    ev_tool_search_call(
                        call_id,
                        &json!({
                            "query": query,
                            "limit": 8,
                        }),
                    )
                }))
                .chain(std::iter::once(ev_completed("resp-1")))
                .collect()),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = configured_builder(apps_server.chatgpt_base_url.clone());
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "Search for calendar tooling.",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = mock.requests();
    assert_eq!(requests.len(), 2);

    assert!(
        tool_search_output_has_namespace_child(
            &requests[1],
            "tool-search-mcp-raw-name",
            SEARCH_CALENDAR_NAMESPACE,
            "_timezone_option_99"
        ),
        "expected raw MCP tool-name query to surface _timezone_option_99: {:?}",
        tool_search_output_tools(&requests[1], "tool-search-mcp-raw-name")
    );
    assert!(
        tool_search_output_has_namespace_child(
            &requests[1],
            "tool-search-mcp-description",
            SEARCH_CALENDAR_NAMESPACE,
            "_extract_text"
        ),
        "expected MCP description query to surface _extract_text: {:?}",
        tool_search_output_tools(&requests[1], "tool-search-mcp-description")
    );
    assert!(
        tool_search_output_has_namespace_child(
            &requests[1],
            "tool-search-mcp-schema",
            SEARCH_CALENDAR_NAMESPACE,
            SEARCH_CALENDAR_CREATE_TOOL
        ),
        "expected MCP schema query to surface {SEARCH_CALENDAR_CREATE_TOOL}: {:?}",
        tool_search_output_tools(&requests[1], "tool-search-mcp-schema")
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_matches_dynamic_tools_by_name_description_namespace_and_schema_terms()
-> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let query_cases = [
        ("tool-search-dynamic-name", "quasar_ping_beacon"),
        ("tool-search-dynamic-spaces", "quasar ping beacon"),
        ("tool-search-dynamic-description", "saffron metronome"),
        ("tool-search-dynamic-namespace", "orbit_ops"),
        ("tool-search-dynamic-schema", "chrono_spec"),
    ];
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(std::iter::once(ev_response_created("resp-1"))
                .chain(query_cases.into_iter().map(|(call_id, query)| {
                    ev_tool_search_call(
                        call_id,
                        &json!({
                            "query": query,
                            "limit": 8,
                        }),
                    )
                }))
                .chain(std::iter::once(ev_completed("resp-1")))
                .collect()),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let dynamic_tool = DynamicToolSpec {
        namespace: Some("orbit_ops".to_string()),
        name: "quasar_ping_beacon".to_string(),
        description: "Trigger the saffron metronome workflow for reminder follow-ups.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "chrono_spec": { "type": "string" },
                "targetThreadId": { "type": "string" },
            },
            "required": ["chrono_spec"],
            "additionalProperties": false,
        }),
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
                text: "Search for the dynamic tool".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = mock.requests();
    assert_eq!(requests.len(), 2);

    for call_id in [
        "tool-search-dynamic-name",
        "tool-search-dynamic-spaces",
        "tool-search-dynamic-description",
        "tool-search-dynamic-namespace",
        "tool-search-dynamic-schema",
    ] {
        assert!(
            tool_search_output_has_namespace_child(
                &requests[1],
                call_id,
                "orbit_ops",
                "quasar_ping_beacon"
            ),
            "expected query {call_id} to surface the quasar_ping_beacon tool: {:?}",
            tool_search_output_tools(&requests[1], call_id)
        );
    }

    Ok(())
}
