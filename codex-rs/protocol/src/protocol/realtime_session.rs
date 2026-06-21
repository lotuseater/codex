use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use codex_utils_path_uri::PathUri;

use super::*;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema)]
pub struct TurnEnvironmentSelection {
    pub environment_id: String,
    pub cwd: PathUri,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct W3cTraceContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub traceparent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tracestate: Option<String>,
}

/// Config payload for refreshing MCP servers.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema)]
pub struct McpServerRefreshConfig {
    /// Complete runtime server map after source and thread-scoped resolution.
    pub mcp_servers: Value,
    /// OAuth credential store mode to use with this server snapshot.
    pub mcp_oauth_credentials_store_mode: Value,
    pub auth_keyring_backend_kind: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema, TS)]
pub struct ConversationStartParams {
    /// Whether Codex response handoffs are managed through explicit client append calls.
    pub client_managed_handoffs: bool,
    /// Sends automatic Codex responses as realtime conversation items instead of handoff appends.
    pub codex_responses_as_items: bool,
    /// Optional prefix added to automatic Codex response items when `codex_responses_as_items` is set.
    pub codex_response_item_prefix: Option<String>,
    /// Optional prefix added to automatic V1 Codex commentary sent with
    /// `conversation.handoff.append` when `codex_responses_as_items` is not set. Final answers are
    /// sent without the prefix.
    pub codex_response_handoff_prefix: Option<String>,
    /// Overrides the configured realtime model for this session only.
    pub model: Option<String>,
    /// Selects whether the realtime session should produce text or audio output.
    pub output_modality: RealtimeOutputModality,
    /// Whether to append Codex's startup context to the realtime backend prompt.
    pub include_startup_context: bool,
    #[serde(
        default,
        deserialize_with = "conversation_start_prompt_serde::deserialize",
        serialize_with = "conversation_start_prompt_serde::serialize",
        skip_serializing_if = "Option::is_none"
    )]
    pub prompt: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realtime_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<ConversationStartTransport>,
    /// Overrides the configured realtime protocol version for this session only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<RealtimeConversationVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<RealtimeVoice>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(tag = "type")]
pub enum ConversationStartTransport {
    Websocket,
    Webrtc { sdp: String },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeOutputModality {
    Text,
    Audio,
}

mod conversation_start_prompt_serde {
    use serde::Deserializer;
    use serde::Serializer;

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_with::rust::double_option::deserialize(deserializer)
    }

    pub(crate) fn serialize<S>(
        value: &Option<Option<String>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_with::rust::double_option::serialize(value, serializer)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct RealtimeAudioFrame {
    pub data: String,
    pub sample_rate: u32,
    pub num_channels: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples_per_channel: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct RealtimeTranscriptDelta {
    pub delta: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct RealtimeTranscriptDone {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct RealtimeTranscriptEntry {
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct RealtimeHandoffRequested {
    pub handoff_id: String,
    pub item_id: String,
    pub input_transcript: String,
    pub active_transcript: Vec<RealtimeTranscriptEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct RealtimeNoopRequested {
    pub call_id: String,
    pub item_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct RealtimeInputAudioSpeechStarted {
    pub item_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct RealtimeResponseCancelled {
    pub response_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct RealtimeResponseCreated {
    pub response_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct RealtimeResponseDone {
    pub response_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub enum RealtimeEvent {
    SessionUpdated {
        realtime_session_id: String,
        instructions: Option<String>,
    },
    InputAudioSpeechStarted(RealtimeInputAudioSpeechStarted),
    InputTranscriptDelta(RealtimeTranscriptDelta),
    InputTranscriptDone(RealtimeTranscriptDone),
    OutputTranscriptDelta(RealtimeTranscriptDelta),
    OutputTranscriptDone(RealtimeTranscriptDone),
    AudioOut(RealtimeAudioFrame),
    ResponseCreated(RealtimeResponseCreated),
    ResponseCancelled(RealtimeResponseCancelled),
    ResponseDone(RealtimeResponseDone),
    ConversationItemAdded(Value),
    ConversationItemDone {
        item_id: String,
    },
    HandoffRequested(RealtimeHandoffRequested),
    NoopRequested(RealtimeNoopRequested),
    Error(String),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema, TS)]
pub struct ConversationAudioParams {
    pub frame: RealtimeAudioFrame,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema, TS)]
pub struct ConversationTextParams {
    pub text: String,
    pub role: ConversationTextRole,
}
