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
async fn tool_search_indexes_only_enabled_non_app_mcp_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let echo_call_id = "tool-search-echo";
    let image_call_id = "tool-search-image";
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_tool_search_call(
                    echo_call_id,
                    &json!({
                        "query": "Echo back the provided message and include environment data.",
                        "limit": 8,
                    }),
                ),
                ev_tool_search_call(
                    image_call_id,
                    &json!({
                        "query": "Return a single image content block.",
                        "limit": 8,
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

    let rmcp_test_server_bin = stdio_server_bin()?;
    let mut builder =
        configured_builder(apps_server.chatgpt_base_url.clone()).with_config(move |config| {
            let mut servers = config.mcp_servers.get().clone();
            servers.insert(
                "rmcp".to_string(),
                McpServerConfig {
                    transport: McpServerTransportConfig::Stdio {
                        command: rmcp_test_server_bin,
                        args: Vec::new(),
                        env: None,
                        env_vars: Vec::new(),
                        cwd: None,
                    },
                    experimental_environment: None,
                    enabled: true,
                    required: false,
                    disabled_reason: None,
                    startup_timeout_sec: Some(Duration::from_secs(10)),
                    tool_timeout_sec: None,
                    default_tools_approval_mode: None,
                    enabled_tools: Some(vec!["echo".to_string(), "image".to_string()]),
                    disabled_tools: Some(vec!["image".to_string()]),
                    scopes: None,
                    oauth: None,
                    oauth_resource: None,
                    supports_parallel_tool_calls: false,
                    tools: HashMap::new(),
                },
            );
            config
                .mcp_servers
                .set(servers)
                .expect("test mcp servers should accept any configuration");
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "Find the rmcp echo and image tools.",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = mock.requests();
    assert_eq!(requests.len(), 2);

    let first_request_tools = tool_names(&requests[0].body_json());
    assert!(
        first_request_tools
            .iter()
            .any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "first request should advertise tool_search: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools
            .iter()
            .any(|name| name == "mcp__rmcp__echo"),
        "non-app MCP tools should be hidden before search in large-search mode: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools.iter().any(|name| name == "mcp__rmcp__"),
        "non-app MCP namespace should be hidden before search in large-search mode: {first_request_tools:?}"
    );

    let echo_tools = tool_search_output_tools(&requests[1], echo_call_id);
    let echo_output = json!({ "tools": echo_tools });
    let rmcp_echo_tool = namespace_child_tool(&echo_output, "mcp__rmcp__", "echo")
        .expect("tool_search should return rmcp echo as a namespace child tool");
    assert_eq!(
        rmcp_echo_tool.get("type").and_then(Value::as_str),
        Some("function")
    );

    let image_tools = tool_search_output_tools(&requests[1], image_call_id);
    let found_rmcp_image_tool = image_tools
        .iter()
        .filter(|tool| tool.get("name").and_then(Value::as_str) == Some("mcp__rmcp__"))
        .flat_map(|namespace| namespace.get("tools").and_then(Value::as_array))
        .flatten()
        .any(|tool| tool.get("name").and_then(Value::as_str).is_some());
    assert!(
        !found_rmcp_image_tool,
        "disabled non-app MCP tools should not be searchable: {image_tools:?}"
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_surfaced_mcp_tool_errors_are_returned_to_model() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let search_call_id = "tool-search-rmcp-echo";
    let tool_call_id = "rmcp-echo-error";
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_tool_search_call(
                    search_call_id,
                    &json!({
                        "query": "Echo back the provided message and include environment data.",
                        "limit": 8,
                    }),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_function_call_with_namespace(tool_call_id, "mcp__rmcp__", "echo", "{}"),
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

    let rmcp_test_server_bin = stdio_server_bin()?;
    let mut builder =
        configured_builder(apps_server.chatgpt_base_url.clone()).with_config(move |config| {
            config
                .features
                .enable(Feature::ToolSearchAlwaysDeferMcpTools)
                .expect("test config should allow feature update");
            let mut servers = config.mcp_servers.get().clone();
            servers.insert(
                "rmcp".to_string(),
                McpServerConfig {
                    transport: McpServerTransportConfig::Stdio {
                        command: rmcp_test_server_bin,
                        args: Vec::new(),
                        env: None,
                        env_vars: Vec::new(),
                        cwd: None,
                    },
                    experimental_environment: None,
                    enabled: true,
                    required: false,
                    disabled_reason: None,
                    startup_timeout_sec: Some(Duration::from_secs(10)),
                    tool_timeout_sec: None,
                    default_tools_approval_mode: None,
                    enabled_tools: Some(vec!["echo".to_string()]),
                    disabled_tools: None,
                    scopes: None,
                    oauth: None,
                    oauth_resource: None,
                    supports_parallel_tool_calls: false,
                    tools: HashMap::new(),
                },
            );
            config
                .mcp_servers
                .set(servers)
                .expect("test mcp servers should accept any configuration");
        });
    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "Find the rmcp echo tool and call it.".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let EventMsg::McpToolCallEnd(end) = wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::McpToolCallEnd(_))
    })
    .await
    else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };
    assert_eq!(end.call_id, tool_call_id);
    assert!(!end.is_success());
    let tool_error = end
        .result
        .as_ref()
        .expect_err("rmcp echo error should stay in the MCP result");
    assert!(
        tool_error.contains("tool call error:")
            && tool_error.contains("missing field")
            && tool_error.contains("message"),
        "MCP invocation should report the execution failure: {tool_error}"
    );

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = mock.requests();
    assert_eq!(requests.len(), 3);

    let first_request_tools = tool_names(&requests[0].body_json());
    assert!(
        first_request_tools
            .iter()
            .any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "first request should advertise tool_search: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools.iter().any(|name| name == "mcp__rmcp__"),
        "deferred rmcp namespace should not be directly exposed before search: {first_request_tools:?}"
    );

    assert!(
        tool_search_output_has_namespace_child(&requests[1], search_call_id, "mcp__rmcp__", "echo"),
        "tool_search should return the rmcp echo tool"
    );

    let output = requests[2].function_call_output(tool_call_id);
    let output_text = match output.get("output") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        other => panic!("unexpected MCP error output payload: {other:?}"),
    };
    assert!(
        output_text.contains("missing field") && output_text.contains("message"),
        "MCP error output should be model visible: {output_text}"
    );
    assert!(
        !output_text.contains("unsupported call"),
        "search-surfaced MCP calls should not fall through to unsupported call: {output_text}"
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_uses_non_app_mcp_server_instructions_as_namespace_description() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let search_call_id = "tool-search-echo";
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_tool_search_call(
                    search_call_id,
                    &json!({
                        "query": "Echo back the provided message and include environment data.",
                        "limit": 8,
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

    let rmcp_test_server_bin = stdio_server_bin()?;
    let mut builder =
        configured_builder(apps_server.chatgpt_base_url.clone()).with_config(move |config| {
            let mut servers = config.mcp_servers.get().clone();
            servers.insert(
                "rmcp".to_string(),
                McpServerConfig {
                    transport: McpServerTransportConfig::Stdio {
                        command: rmcp_test_server_bin,
                        args: Vec::new(),
                        env: None,
                        env_vars: Vec::new(),
                        cwd: None,
                    },
                    experimental_environment: None,
                    enabled: true,
                    required: false,
                    disabled_reason: None,
                    startup_timeout_sec: Some(Duration::from_secs(10)),
                    tool_timeout_sec: None,
                    default_tools_approval_mode: None,
                    enabled_tools: Some(vec!["echo".to_string()]),
                    disabled_tools: None,
                    scopes: None,
                    oauth: None,
                    oauth_resource: None,
                    supports_parallel_tool_calls: false,
                    tools: HashMap::new(),
                },
            );
            config
                .mcp_servers
                .set(servers)
                .expect("test mcp servers should accept any configuration");
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "Find the rmcp echo tool.",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = mock.requests();
    assert_eq!(requests.len(), 2);

    let tools = tool_search_output_tools(&requests[1], search_call_id);
    let rmcp_namespace = tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some("mcp__rmcp__"))
        .expect("tool_search should return the rmcp namespace");
    assert_eq!(
        rmcp_namespace.get("description").and_then(Value::as_str),
        Some("Use these tools to exercise the rmcp test server.")
    );

    Ok(())
}
