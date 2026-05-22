#![allow(clippy::unwrap_used)]

pub(super) use codex_core_test_runtime::responses;
pub(super) use codex_core_test_runtime::responses::start_mock_server;
pub(super) use codex_core_test_runtime::skip_if_no_network;
pub(super) use codex_core_test_runtime::test_codex::test_codex;
pub(super) use codex_features::Feature;
pub(super) use codex_protocol::config_types::WebSearchMode;
pub(super) use codex_protocol::models::PermissionProfile;
pub(super) use pretty_assertions::assert_eq;
pub(super) use serde_json::Value;
pub(super) use serde_json::json;
pub(super) use std::sync::Arc;

#[allow(clippy::expect_used)]
pub(super) fn find_web_search_tool(body: &Value) -> &Value {
    body["tools"]
        .as_array()
        .expect("request body should include tools array")
        .iter()
        .find(|tool| tool.get("type").and_then(Value::as_str) == Some("web_search"))
        .expect("tools should include a web_search tool")
}
