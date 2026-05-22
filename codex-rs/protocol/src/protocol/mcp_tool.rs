use std::fmt;
use std::time::Duration;

use codex_utils_absolute_path::AbsolutePathBuf;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

use crate::dynamic_tools::DynamicToolCallOutputContentItem;
use crate::mcp::CallToolResult;
use crate::models::WebSearchAction;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS, PartialEq)]
pub struct McpInvocation {
    /// Name of the MCP server as defined in the config.
    pub server: String,
    /// Name of the tool as given by the MCP server.
    pub tool: String,
    /// Arguments to the tool call.
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS, PartialEq)]
pub struct McpToolCallBeginEvent {
    /// Identifier so this can be paired with the McpToolCallEnd event.
    pub call_id: String,
    pub invocation: McpInvocation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub mcp_app_resource_uri: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS, PartialEq)]
pub struct McpToolCallEndEvent {
    /// Identifier for the corresponding McpToolCallBegin that finished.
    pub call_id: String,
    pub invocation: McpInvocation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub mcp_app_resource_uri: Option<String>,
    #[ts(type = "string")]
    pub duration: Duration,
    /// Result of the tool call. Note this could be an error.
    pub result: Result<CallToolResult, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS, PartialEq)]
pub struct DynamicToolCallResponseEvent {
    /// Identifier for the corresponding DynamicToolCallRequest.
    pub call_id: String,
    /// Turn ID that this dynamic tool call belongs to.
    pub turn_id: String,
    #[serde(default)]
    pub completed_at_ms: i64,
    /// Dynamic tool namespace, when one was provided.
    #[serde(default)]
    pub namespace: Option<String>,
    /// Dynamic tool name.
    pub tool: String,
    /// Dynamic tool call arguments.
    pub arguments: serde_json::Value,
    /// Dynamic tool response content items.
    pub content_items: Vec<DynamicToolCallOutputContentItem>,
    /// Whether the tool call succeeded.
    pub success: bool,
    /// Optional error text when the tool call failed before producing a response.
    pub error: Option<String>,
    /// The duration of the dynamic tool call.
    #[ts(type = "string")]
    pub duration: Duration,
}

impl McpToolCallEndEvent {
    pub fn is_success(&self) -> bool {
        match &self.result {
            Ok(result) => !result.is_error.unwrap_or(false),
            Err(_) => false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS)]
pub struct WebSearchBeginEvent {
    pub call_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS)]
pub struct WebSearchEndEvent {
    pub call_id: String,
    pub query: String,
    pub action: WebSearchAction,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS)]
pub struct ImageGenerationBeginEvent {
    pub call_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS)]
pub struct ImageGenerationEndEvent {
    pub call_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub revised_prompt: Option<String>,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub saved_path: Option<AbsolutePathBuf>,
}


#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS)]
pub struct McpStartupUpdateEvent {
    /// Server name being started.
    pub server: String,
    /// Current startup status.
    pub status: McpStartupStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case", tag = "state")]
#[ts(rename_all = "snake_case", tag = "state")]
pub enum McpStartupStatus {
    Starting,
    Ready,
    Failed { error: String },
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS, Default)]
pub struct McpStartupCompleteEvent {
    pub ready: Vec<String>,
    pub failed: Vec<McpStartupFailure>,
    pub cancelled: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS)]
pub struct McpStartupFailure {
    pub server: String,
    pub error: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum McpAuthStatus {
    Unsupported,
    NotLoggedIn,
    BearerToken,
    OAuth,
}

impl fmt::Display for McpAuthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            McpAuthStatus::Unsupported => "Unsupported",
            McpAuthStatus::NotLoggedIn => "Not logged in",
            McpAuthStatus::BearerToken => "Bearer token",
            McpAuthStatus::OAuth => "OAuth",
        };
        f.write_str(text)
    }
}

