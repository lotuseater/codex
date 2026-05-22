#![allow(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]

pub(crate) use anyhow::Result;
pub(crate) use codex_config::types::McpServerConfig;
pub(crate) use codex_config::types::McpServerTransportConfig;
pub(crate) use codex_core_test_runtime::apps_test_server::AppsTestServer;
pub(crate) use codex_core_test_runtime::apps_test_server::CALENDAR_CREATE_EVENT_MCP_APP_RESOURCE_URI;
pub(crate) use codex_core_test_runtime::apps_test_server::CALENDAR_CREATE_EVENT_RESOURCE_URI;
pub(crate) use codex_core_test_runtime::apps_test_server::DIRECT_CALENDAR_CREATE_EVENT_TOOL as CALENDAR_CREATE_TOOL;
pub(crate) use codex_core_test_runtime::apps_test_server::DIRECT_CALENDAR_LIST_EVENTS_TOOL as CALENDAR_LIST_TOOL;
pub(crate) use codex_core_test_runtime::apps_test_server::SEARCH_CALENDAR_CREATE_TOOL;
pub(crate) use codex_core_test_runtime::apps_test_server::SEARCH_CALENDAR_LIST_TOOL;
pub(crate) use codex_core_test_runtime::apps_test_server::SEARCH_CALENDAR_NAMESPACE;
pub(crate) use codex_core_test_runtime::apps_test_server::configure_search_capable_apps;
pub(crate) use codex_core_test_runtime::apps_test_server::configure_search_capable_model;
pub(crate) use codex_core_test_runtime::apps_test_server::recorded_apps_tool_call_by_call_id;
pub(crate) use codex_core_test_runtime::apps_test_server::search_capable_apps_builder as configured_builder;
pub(crate) use codex_core_test_runtime::responses::ResponsesRequest;
pub(crate) use codex_core_test_runtime::responses::ev_assistant_message;
pub(crate) use codex_core_test_runtime::responses::ev_completed;
pub(crate) use codex_core_test_runtime::responses::ev_function_call_with_namespace;
pub(crate) use codex_core_test_runtime::responses::ev_response_created;
pub(crate) use codex_core_test_runtime::responses::ev_tool_search_call;
pub(crate) use codex_core_test_runtime::responses::mount_sse_once;
pub(crate) use codex_core_test_runtime::responses::mount_sse_sequence;
pub(crate) use codex_core_test_runtime::responses::namespace_child_tool;
pub(crate) use codex_core_test_runtime::responses::sse;
pub(crate) use codex_core_test_runtime::responses::start_mock_server;
pub(crate) use codex_core_test_runtime::skip_if_no_network;
pub(crate) use codex_core_test_runtime::stdio_server_bin;
pub(crate) use codex_core_test_runtime::test_codex::test_codex;
pub(crate) use codex_core_test_runtime::wait_for_event;
pub(crate) use codex_features::Feature;
pub(crate) use codex_login::CodexAuth;
pub(crate) use codex_protocol::dynamic_tools::DynamicToolCallOutputContentItem;
pub(crate) use codex_protocol::dynamic_tools::DynamicToolResponse;
pub(crate) use codex_protocol::dynamic_tools::DynamicToolSpec;
pub(crate) use codex_protocol::models::FunctionCallOutputPayload;
pub(crate) use codex_protocol::models::PermissionProfile;
pub(crate) use codex_protocol::protocol::AskForApproval;
pub(crate) use codex_protocol::protocol::EventMsg;
pub(crate) use codex_protocol::protocol::McpInvocation;
pub(crate) use codex_protocol::protocol::Op;
pub(crate) use codex_protocol::user_input::UserInput;
pub(crate) use pretty_assertions::assert_eq;
pub(crate) use serde_json::Value;
pub(crate) use serde_json::json;
pub(crate) use std::collections::HashMap;
pub(crate) use std::time::Duration;

pub(crate) const SEARCH_TOOL_DESCRIPTION_SNIPPETS: [&str; 2] = [
    "You have access to tools from the following sources",
    "- Calendar: Plan events and manage your calendar.",
];
pub(crate) const TOOL_SEARCH_TOOL_NAME: &str = "tool_search";

pub(crate) fn tool_names(body: &Value) -> Vec<String> {
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

pub(crate) fn tool_search_description(body: &Value) -> Option<String> {
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

pub(crate) fn tool_search_output_item(request: &ResponsesRequest, call_id: &str) -> Value {
    request.tool_search_output(call_id)
}

pub(crate) fn tool_search_output_tools(request: &ResponsesRequest, call_id: &str) -> Vec<Value> {
    tool_search_output_item(request, call_id)
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn tool_search_output_has_namespace_child(
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
