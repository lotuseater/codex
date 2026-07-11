use std::collections::HashMap;

use codex_utils_image::PromptImageMode;
use codex_utils_image::data_url_from_bytes;
use codex_utils_image::load_for_prompt_bytes;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::ser::Serializer;
use ts_rs::TS;

use crate::user_input::UserInput;
use codex_utils_image::ImageProcessingError;
use schemars::JsonSchema;

use crate::mcp::CallToolResult;

// fork-local: PermissionProfile and related runtime-permission types live in
// the extracted `codex_permission_types` crate and are re-exported here via the
// `pub use codex_permission_types::...` block below. Upstream defines these
// types inline in this module; the fork intentionally drops the inline copy to
// avoid duplicate definitions while preserving identical behavior through the
// crate (including the richer fork shape: `ActivePermissionProfile` carries the
// `modifications` field, `ActivePermissionProfileModification` adds the
// `AdditionalWritableRoot` variant, and `FileSystemAccessMode` adds the `None`
// access mode).
pub use codex_permission_types::ActivePermissionProfile;
pub use codex_permission_types::ActivePermissionProfileModification;
pub use codex_permission_types::AdditionalPermissionProfile;
pub use codex_permission_types::FileSystemAccessMode;
pub use codex_permission_types::FileSystemPath;
pub use codex_permission_types::FileSystemPermissions;
pub use codex_permission_types::FileSystemSandboxEntry;
pub use codex_permission_types::FileSystemSandboxKind;
pub use codex_permission_types::FileSystemSandboxPolicy;
pub use codex_permission_types::FileSystemSpecialPath;
pub use codex_permission_types::LegacyReadWriteRoots;
pub use codex_permission_types::ManagedFileSystemPermissions;
pub use codex_permission_types::NetworkPermissions;
pub use codex_permission_types::NetworkSandboxPolicy;
pub use codex_permission_types::PermissionProfile;
pub use codex_permission_types::SandboxEnforcement;
pub use codex_permission_types::SandboxPermissions;

/// Reserved identifier for the built-in read-only permission profile.
pub const BUILT_IN_PERMISSION_PROFILE_READ_ONLY: &str = ":read-only";

/// Reserved identifier for the built-in workspace-write permission profile.
pub const BUILT_IN_PERMISSION_PROFILE_WORKSPACE: &str = ":workspace";

/// Reserved identifier for the built-in full-access permission profile.
pub const BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS: &str = ":danger-full-access";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseInputItem {
    Message {
        role: String,
        content: Vec<ContentItem>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        phase: Option<MessagePhase>,
    },
    FunctionCallOutput {
        call_id: String,
        #[ts(as = "FunctionCallOutputBody")]
        #[schemars(with = "FunctionCallOutputBody")]
        output: FunctionCallOutputPayload,
    },
    McpToolCallOutput {
        call_id: String,
        output: CallToolResult,
    },
    CustomToolCallOutput {
        call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        name: Option<String>,
        #[ts(as = "FunctionCallOutputBody")]
        #[schemars(with = "FunctionCallOutputBody")]
        output: FunctionCallOutputPayload,
    },
    ToolSearchOutput {
        call_id: String,
        status: String,
        execution: String,
        #[ts(type = "unknown[]")]
        tools: Vec<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentItem {
    InputText {
        text: String,
    },
    InputImage {
        image_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        detail: Option<ImageDetail>,
    },
    OutputText {
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMessageInputContent {
    InputText { text: String },
    EncryptedContent { encrypted_content: String },
}

/// Returns the locally readable text when an agent message is entirely plaintext.
pub fn plaintext_agent_message_content(content: &[AgentMessageInputContent]) -> Option<String> {
    let mut text_parts = Vec::with_capacity(content.len());
    for part in content {
        match part {
            AgentMessageInputContent::InputText { text } => text_parts.push(text.as_str()),
            AgentMessageInputContent::EncryptedContent { .. } => return None,
        }
    }

    let text = text_parts.join("\n");
    (!text.trim().is_empty()).then_some(text)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    Auto,
    Low,
    High,
    Original,
}

pub const DEFAULT_IMAGE_DETAIL: ImageDetail = ImageDetail::High;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
/// Classifies an assistant message as interim commentary or final answer text.
///
/// Providers do not emit this consistently, so callers must treat `None` as
/// "phase unknown" and keep compatibility behavior for legacy models.
pub enum MessagePhase {
    /// Mid-turn assistant text (for example preamble/progress narration).
    ///
    /// Additional tool calls or assistant output may follow before turn
    /// completion.
    Commentary,
    /// The assistant's terminal answer text for the current turn.
    FinalAnswer,
}

/// Internal Responses API passthrough metadata copied into underlying chat messages.
///
/// Responses API strongly types this payload. Do not modify it without first getting API
/// approval and making the corresponding Responses API change.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema, TS)]
pub struct InternalChatMessageMetadataPassthrough {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub turn_id: Option<String>,
}

impl InternalChatMessageMetadataPassthrough {
    pub(crate) fn set_turn_id_if_missing(metadata: &mut Option<Self>, turn_id: &str) {
        if turn_id.is_empty()
            || metadata
                .as_ref()
                .and_then(|metadata| metadata.turn_id.as_deref())
                .is_some_and(|turn_id| !turn_id.is_empty())
        {
            return;
        }
        metadata.get_or_insert_with(Self::default).turn_id = Some(turn_id.to_string());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseItem {
    #[schemars(skip)]
    #[ts(skip)]
    AdditionalTools {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        role: String,
        tools: Vec<serde_json::Value>,
    },
    Message {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        role: String,
        content: Vec<ContentItem>,
        // Optional output-message phase (for example: "commentary", "final_answer").
        // Availability varies by provider/model, so downstream consumers must
        // preserve fallback behavior when this is absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        phase: Option<MessagePhase>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    AgentMessage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        author: String,
        recipient: String,
        content: Vec<AgentMessageInputContent>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    Reasoning {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        summary: Vec<ReasoningItemReasoningSummary>,
        #[serde(default, skip_serializing_if = "should_serialize_reasoning_content")]
        #[ts(optional)]
        content: Option<Vec<ReasoningItemContent>>,
        encrypted_content: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    LocalShellCall {
        /// Legacy id field retained for compatibility with older payloads.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        /// Set when using the Responses API.
        call_id: Option<String>,
        status: LocalShellStatus,
        action: LocalShellAction,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    FunctionCall {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        namespace: Option<String>,
        // The Responses API returns the function call arguments as a *string* that contains
        // JSON, not as an already‑parsed object. We keep it as a raw string here and let
        // Session::handle_function_call parse it into a Value.
        arguments: String,
        call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    ToolSearchCall {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        call_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        status: Option<String>,
        execution: String,
        #[ts(type = "unknown")]
        arguments: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    // NOTE: The `output` field for `function_call_output` uses a dedicated payload type with
    // custom serialization. On the wire it is either:
    //   - a plain string (`content`)
    //   - an array of structured content items (`content_items`)
    // We keep this behavior centralized in `FunctionCallOutputPayload`.
    FunctionCallOutput {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        call_id: String,
        #[ts(as = "FunctionCallOutputBody")]
        #[schemars(with = "FunctionCallOutputBody")]
        output: FunctionCallOutputPayload,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    CustomToolCall {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        status: Option<String>,

        call_id: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        namespace: Option<String>,
        input: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    // `custom_tool_call_output.output` uses the same wire encoding as
    // `function_call_output.output` so freeform tools can return either plain
    // text or structured content items.
    CustomToolCallOutput {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        name: Option<String>,
        #[ts(as = "FunctionCallOutputBody")]
        #[schemars(with = "FunctionCallOutputBody")]
        output: FunctionCallOutputPayload,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    ToolSearchOutput {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        call_id: Option<String>,
        status: String,
        execution: String,
        #[ts(type = "unknown[]")]
        tools: Vec<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    // Emitted by the Responses API when the agent triggers a web search.
    // Example payload (from SSE `response.output_item.done`):
    // {
    //   "id":"ws_...",
    //   "type":"web_search_call",
    //   "status":"completed",
    //   "action": {"type":"search","query":"weather: San Francisco, CA"}
    // }
    WebSearchCall {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        status: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        action: Option<WebSearchAction>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    // Emitted by the Responses API when the agent triggers image generation.
    // Example payload:
    // {
    //   "id":"ig_123",
    //   "type":"image_generation_call",
    //   "status":"completed",
    //   "revised_prompt":"A gray tabby cat hugging an otter...",
    //   "result":"..."
    // }
    ImageGenerationCall {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        status: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        revised_prompt: Option<String>,
        result: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    #[serde(alias = "compaction_summary")]
    Compaction {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        encrypted_content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    // Compaction triggers are request controls, not durable response items.
    CompactionTrigger {},
    ContextCompaction {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        encrypted_content: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    },
    #[serde(other)]
    Other,
}

impl ResponseItem {
    /// Returns whether this item is an ordinary user-role message.
    pub fn is_user_message(&self) -> bool {
        matches!(self, Self::Message { role, .. } if role == "user")
    }

    /// Returns the non-empty Responses API item ID, if present.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::AdditionalTools { id, .. }
            | Self::Message { id, .. }
            | Self::AgentMessage { id, .. }
            | Self::LocalShellCall { id, .. }
            | Self::FunctionCall { id, .. }
            | Self::ToolSearchCall { id, .. }
            | Self::FunctionCallOutput { id, .. }
            | Self::CustomToolCall { id, .. }
            | Self::CustomToolCallOutput { id, .. }
            | Self::ToolSearchOutput { id, .. }
            | Self::WebSearchCall { id, .. }
            | Self::Reasoning { id, .. }
            | Self::ImageGenerationCall { id, .. }
            | Self::Compaction { id, .. }
            | Self::ContextCompaction { id, .. } => id.as_deref().filter(|id| !id.is_empty()),
            Self::CompactionTrigger { .. } | Self::Other => None,
        }
    }

    /// Sets or clears the Responses API item ID for variants that carry one.
    pub fn set_id(&mut self, new_id: Option<String>) {
        match self {
            Self::AdditionalTools { id, .. }
            | Self::Message { id, .. }
            | Self::AgentMessage { id, .. }
            | Self::LocalShellCall { id, .. }
            | Self::FunctionCall { id, .. }
            | Self::ToolSearchCall { id, .. }
            | Self::FunctionCallOutput { id, .. }
            | Self::CustomToolCall { id, .. }
            | Self::CustomToolCallOutput { id, .. }
            | Self::ToolSearchOutput { id, .. }
            | Self::WebSearchCall { id, .. }
            | Self::Reasoning { id, .. }
            | Self::ImageGenerationCall { id, .. }
            | Self::Compaction { id, .. }
            | Self::ContextCompaction { id, .. } => *id = new_id,
            Self::CompactionTrigger { .. } | Self::Other => {}
        }
    }

    /// Returns the non-empty turn ID stamped onto this item, if present.
    pub fn turn_id(&self) -> Option<&str> {
        self.internal_chat_message_metadata_passthrough()
            .and_then(|metadata| metadata.turn_id.as_deref())
            .filter(|turn_id| !turn_id.is_empty())
    }

    /// Stamps the item with `turn_id` unless it already has a non-empty turn ID.
    pub fn set_turn_id_if_missing(&mut self, turn_id: &str) {
        let Some(metadata) = self.internal_chat_message_metadata_passthrough_mut() else {
            return;
        };
        InternalChatMessageMetadataPassthrough::set_turn_id_if_missing(metadata, turn_id);
    }

    /// Removes internal chat message metadata passthrough before sending to a provider that does
    /// not accept it.
    pub fn clear_internal_chat_message_metadata_passthrough(&mut self) {
        if let Some(metadata) = self.internal_chat_message_metadata_passthrough_mut() {
            *metadata = None;
        }
    }

    fn internal_chat_message_metadata_passthrough(
        &self,
    ) -> Option<&InternalChatMessageMetadataPassthrough> {
        match self {
            Self::Message {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::AgentMessage {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::Reasoning {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::LocalShellCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::FunctionCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::ToolSearchCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::FunctionCallOutput {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::CustomToolCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::CustomToolCallOutput {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::ToolSearchOutput {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::WebSearchCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::ImageGenerationCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::Compaction {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::ContextCompaction {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            } => metadata.as_ref(),
            Self::CompactionTrigger { .. } | Self::AdditionalTools { .. } | Self::Other => None,
        }
    }

    fn internal_chat_message_metadata_passthrough_mut(
        &mut self,
    ) -> Option<&mut Option<InternalChatMessageMetadataPassthrough>> {
        match self {
            Self::Message {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::AgentMessage {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::Reasoning {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::LocalShellCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::FunctionCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::ToolSearchCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::FunctionCallOutput {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::CustomToolCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::CustomToolCallOutput {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::ToolSearchOutput {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::WebSearchCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::ImageGenerationCall {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::Compaction {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            }
            | Self::ContextCompaction {
                internal_chat_message_metadata_passthrough: metadata,
                ..
            } => Some(metadata),
            Self::CompactionTrigger { .. } | Self::AdditionalTools { .. } | Self::Other => None,
        }
    }
}

pub const BASE_INSTRUCTIONS_DEFAULT: &str = include_str!("prompts/base_instructions/default.md");

/// Base instructions for the model in a thread. Corresponds to the `instructions` field in the ResponsesAPI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(rename = "base_instructions", rename_all = "snake_case")]
pub struct BaseInstructions {
    pub text: String,
}

impl Default for BaseInstructions {
    fn default() -> Self {
        Self {
            text: BASE_INSTRUCTIONS_DEFAULT.to_string(),
        }
    }
}

const MAX_RENDERED_PREFIXES: usize = 100;
const MAX_ALLOW_PREFIX_TEXT_BYTES: usize = 5000;
const TRUNCATED_MARKER: &str = "...\n[Some commands were truncated]";

pub fn format_allow_prefixes(prefixes: Vec<Vec<String>>) -> Option<String> {
    let mut truncated = false;
    if prefixes.len() > MAX_RENDERED_PREFIXES {
        truncated = true;
    }

    let mut prefixes = prefixes;
    prefixes.sort_by(|a, b| {
        a.len()
            .cmp(&b.len())
            .then_with(|| prefix_combined_str_len(a).cmp(&prefix_combined_str_len(b)))
            .then_with(|| a.cmp(b))
    });

    let full_text = prefixes
        .into_iter()
        .take(MAX_RENDERED_PREFIXES)
        .map(|prefix| format!("- {}", render_command_prefix(&prefix)))
        .collect::<Vec<_>>()
        .join("\n");

    // truncate to last UTF8 char
    let mut output = full_text;
    let byte_idx = output
        .char_indices()
        .nth(MAX_ALLOW_PREFIX_TEXT_BYTES)
        .map(|(i, _)| i);
    if let Some(byte_idx) = byte_idx {
        truncated = true;
        output = output[..byte_idx].to_string();
    }

    if truncated {
        Some(format!("{output}{TRUNCATED_MARKER}"))
    } else {
        Some(output)
    }
}

fn prefix_combined_str_len(prefix: &[String]) -> usize {
    prefix.iter().map(String::len).sum()
}

fn render_command_prefix(prefix: &[String]) -> String {
    let tokens = prefix
        .iter()
        .map(|token| serde_json::to_string(token).unwrap_or_else(|_| format!("{token:?}")))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{tokens}]")
}

fn should_serialize_reasoning_content(content: &Option<Vec<ReasoningItemContent>>) -> bool {
    match content {
        Some(content) => !content
            .iter()
            .any(|c| matches!(c, ReasoningItemContent::ReasoningText { .. })),
        None => false,
    }
}

fn local_image_error_placeholder(
    path: &std::path::Path,
    error: impl std::fmt::Display,
) -> ContentItem {
    ContentItem::InputText {
        text: format!(
            "Codex could not read the local image at `{}`: {}",
            path.display(),
            error
        ),
    }
}

pub const VIEW_IMAGE_TOOL_NAME: &str = "view_image";

const IMAGE_OPEN_TAG: &str = "<image>";
const IMAGE_CLOSE_TAG: &str = "</image>";
const LOCAL_IMAGE_OPEN_TAG_PREFIX: &str = "<image name=";
const LOCAL_IMAGE_OPEN_TAG_SUFFIX: &str = ">";
const LOCAL_IMAGE_CLOSE_TAG: &str = IMAGE_CLOSE_TAG;

pub fn image_open_tag_text() -> String {
    IMAGE_OPEN_TAG.to_string()
}

pub fn image_close_tag_text() -> String {
    IMAGE_CLOSE_TAG.to_string()
}

pub fn local_image_label_text(label_number: usize) -> String {
    format!("[Image #{label_number}]")
}

pub fn local_image_open_tag_text_with_path(label_number: usize, path: &std::path::Path) -> String {
    let label = local_image_label_text(label_number);
    let path = path.display();
    format!("{LOCAL_IMAGE_OPEN_TAG_PREFIX}{label} path=\"{path}\"{LOCAL_IMAGE_OPEN_TAG_SUFFIX}")
}

pub fn is_local_image_open_tag_text(text: &str) -> bool {
    text.strip_prefix(LOCAL_IMAGE_OPEN_TAG_PREFIX)
        .is_some_and(|rest| rest.ends_with(LOCAL_IMAGE_OPEN_TAG_SUFFIX))
}

pub fn is_local_image_close_tag_text(text: &str) -> bool {
    is_image_close_tag_text(text)
}

pub fn is_image_open_tag_text(text: &str) -> bool {
    text == IMAGE_OPEN_TAG
}

pub fn is_image_close_tag_text(text: &str) -> bool {
    text == IMAGE_CLOSE_TAG
}

fn invalid_image_error_placeholder(
    path: &std::path::Path,
    error: impl std::fmt::Display,
) -> ContentItem {
    ContentItem::InputText {
        text: format!(
            "Image located at `{}` is invalid: {}",
            path.display(),
            error
        ),
    }
}

fn unsupported_image_error_placeholder(path: &std::path::Path, mime: &str) -> ContentItem {
    ContentItem::InputText {
        text: format!(
            "Codex cannot attach image at `{}`: unsupported image `{}`.",
            path.display(),
            mime
        ),
    }
}

pub fn local_image_content_items_with_label_number(
    path: &std::path::Path,
    file_bytes: Vec<u8>,
    label_number: Option<usize>,
    detail: ImageDetail,
) -> Vec<ContentItem> {
    let mode = match detail {
        ImageDetail::Original => PromptImageMode::Original,
        ImageDetail::Auto | ImageDetail::Low | ImageDetail::High => PromptImageMode::ResizeToFit,
    };
    match load_for_prompt_bytes(path, file_bytes, mode) {
        Ok(image) => local_image_content_items(path, image.into_data_url(), label_number, detail),
        Err(err) => match &err {
            ImageProcessingError::Read { .. }
            | ImageProcessingError::Encode { .. }
            | ImageProcessingError::InvalidDataUrl { .. }
            | ImageProcessingError::ImageTooLarge { .. } => {
                vec![local_image_error_placeholder(path, &err)]
            }
            ImageProcessingError::Decode { .. } if err.is_invalid_image() => {
                vec![invalid_image_error_placeholder(path, &err)]
            }
            ImageProcessingError::Decode { .. } => {
                vec![local_image_error_placeholder(path, &err)]
            }
            ImageProcessingError::UnsupportedImageFormat { mime } => {
                vec![unsupported_image_error_placeholder(path, mime)]
            }
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalImagePreparation {
    Process,
    Defer,
}

fn local_image_content_items(
    path: &std::path::Path,
    image_url: String,
    label_number: Option<usize>,
    detail: ImageDetail,
) -> Vec<ContentItem> {
    let mut items = Vec::with_capacity(3);
    if let Some(label_number) = label_number {
        items.push(ContentItem::InputText {
            text: local_image_open_tag_text_with_path(label_number, path),
        });
    }
    items.push(ContentItem::InputImage {
        image_url,
        detail: Some(detail),
    });
    if label_number.is_some() {
        items.push(ContentItem::InputText {
            text: LOCAL_IMAGE_CLOSE_TAG.to_string(),
        });
    }
    items
}

impl From<ResponseInputItem> for ResponseItem {
    fn from(item: ResponseInputItem) -> Self {
        match item {
            ResponseInputItem::Message {
                role,
                content,
                phase,
            } => Self::Message {
                role,
                content,
                id: None,
                phase,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseInputItem::FunctionCallOutput { call_id, output } => Self::FunctionCallOutput {
                id: None,
                call_id,
                output,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseInputItem::McpToolCallOutput { call_id, output } => {
                let output = output.into_function_call_output_payload();
                Self::FunctionCallOutput {
                    id: None,
                    call_id,
                    output,
                    internal_chat_message_metadata_passthrough: None,
                }
            }
            ResponseInputItem::CustomToolCallOutput {
                call_id,
                name,
                output,
            } => Self::CustomToolCallOutput {
                id: None,
                call_id,
                name,
                output,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseInputItem::ToolSearchOutput {
                call_id,
                status,
                execution,
                tools,
            } => Self::ToolSearchOutput {
                call_id: Some(call_id),
                status,
                execution,
                tools,
                id: None,
                internal_chat_message_metadata_passthrough: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum LocalShellStatus {
    Completed,
    InProgress,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LocalShellAction {
    Exec(LocalShellExecAction),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
pub struct LocalShellExecAction {
    pub command: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub working_directory: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "ResponsesApiWebSearchAction")]
pub enum WebSearchAction {
    Search {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        query: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        queries: Option<Vec<String>>,
    },
    OpenPage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        url: Option<String>,
    },
    FindInPage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        pattern: Option<String>,
    },

    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReasoningItemReasoningSummary {
    SummaryText { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReasoningItemContent {
    ReasoningText { text: String },
    Text { text: String },
}

impl From<Vec<UserInput>> for ResponseInputItem {
    fn from(items: Vec<UserInput>) -> Self {
        Self::from_user_input(items, LocalImagePreparation::Process)
    }
}

impl ResponseInputItem {
    pub fn from_user_input(
        items: Vec<UserInput>,
        local_image_preparation: LocalImagePreparation,
    ) -> Self {
        let mut image_index = 0;
        Self::Message {
            role: "user".to_string(),
            content: items
                .into_iter()
                .flat_map(|c| match c {
                    UserInput::Text { text, .. } => vec![ContentItem::InputText { text }],
                    UserInput::Image {
                        image_url, detail, ..
                    } => {
                        image_index += 1;
                        vec![
                            ContentItem::InputText {
                                text: image_open_tag_text(),
                            },
                            ContentItem::InputImage {
                                image_url,
                                detail: Some(detail.unwrap_or(DEFAULT_IMAGE_DETAIL)),
                            },
                            ContentItem::InputText {
                                text: image_close_tag_text(),
                            },
                        ]
                    }
                    UserInput::LocalImage { path, detail, .. } => {
                        image_index += 1;
                        match std::fs::read(&path) {
                            Ok(file_bytes) => match local_image_preparation {
                                LocalImagePreparation::Process => {
                                    local_image_content_items_with_label_number(
                                        &path,
                                        file_bytes,
                                        Some(image_index),
                                        detail.unwrap_or(DEFAULT_IMAGE_DETAIL),
                                    )
                                }
                                LocalImagePreparation::Defer => local_image_content_items(
                                    &path,
                                    data_url_from_bytes(
                                        &codex_utils_image::sniff_image_mime(&file_bytes),
                                        &file_bytes,
                                    ),
                                    Some(image_index),
                                    detail.unwrap_or(DEFAULT_IMAGE_DETAIL),
                                ),
                            },
                            Err(err) => vec![local_image_error_placeholder(&path, err)],
                        }
                    }
                    UserInput::Skill { .. } | UserInput::Mention { .. } => Vec::new(), // Tool bodies are injected later in core
                })
                .collect::<Vec<ContentItem>>(),
            phase: None,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
pub struct SearchToolCallParams {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub limit: Option<usize>,
}

/// If the `name` of a `ResponseItem::FunctionCall` is `shell_command`, the
/// `arguments` field should deserialize to this struct.
#[derive(Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
pub struct ShellCommandToolCallParams {
    pub command: String,
    pub workdir: Option<String>,

    /// Whether to run the shell with login shell semantics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<bool>,
    /// This is the maximum time in milliseconds that the command is allowed to run.
    #[serde(alias = "timeout")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub sandbox_permissions: Option<SandboxPermissions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub prefix_rule: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub additional_permissions: Option<AdditionalPermissionProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
}

/// Responses API compatible content items that can be returned by a tool call.
/// This is a subset of ContentItem with the types we support as function call outputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FunctionCallOutputContentItem {
    // Do not rename, these are serialized and used directly in the responses API.
    InputText {
        text: String,
    },
    // Do not rename, these are serialized and used directly in the responses API.
    InputImage {
        image_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        detail: Option<ImageDetail>,
    },
    EncryptedContent {
        encrypted_content: String,
    },
}

/// Converts structured function-call output content into plain text for
/// human-readable surfaces.
///
/// This conversion is intentionally lossy:
/// - only `input_text` items are included
/// - image items are ignored
///
/// We use this helper where callers still need a string representation (for
/// example telemetry previews or legacy string-only output paths) while keeping
/// the original multimodal `content_items` as the authoritative payload sent to
/// the model.
pub fn function_call_output_content_items_to_text(
    content_items: &[FunctionCallOutputContentItem],
) -> Option<String> {
    let text_segments = content_items
        .iter()
        .filter_map(|item| match item {
            FunctionCallOutputContentItem::InputText { text } if !text.trim().is_empty() => {
                Some(text.as_str())
            }
            FunctionCallOutputContentItem::InputText { .. }
            | FunctionCallOutputContentItem::InputImage { .. }
            | FunctionCallOutputContentItem::EncryptedContent { .. } => None,
        })
        .collect::<Vec<_>>();

    if text_segments.is_empty() {
        None
    } else {
        Some(text_segments.join("\n"))
    }
}

impl From<crate::dynamic_tools::DynamicToolCallOutputContentItem>
    for FunctionCallOutputContentItem
{
    fn from(item: crate::dynamic_tools::DynamicToolCallOutputContentItem) -> Self {
        match item {
            crate::dynamic_tools::DynamicToolCallOutputContentItem::InputText { text } => {
                Self::InputText { text }
            }
            crate::dynamic_tools::DynamicToolCallOutputContentItem::InputImage { image_url } => {
                Self::InputImage {
                    image_url,
                    detail: Some(DEFAULT_IMAGE_DETAIL),
                }
            }
        }
    }
}

/// The payload we send back to OpenAI when reporting a tool call result.
///
/// `body` serializes directly as the wire value for `function_call_output.output`.
/// `success` remains internal metadata for downstream handling.
#[derive(Debug, Default, Clone, PartialEq, JsonSchema, TS)]
pub struct FunctionCallOutputPayload {
    pub body: FunctionCallOutputBody,
    pub success: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(untagged)]
pub enum FunctionCallOutputBody {
    Text(String),
    ContentItems(Vec<FunctionCallOutputContentItem>),
}

impl FunctionCallOutputBody {
    /// Best-effort conversion of a function-call output body to plain text for
    /// human-readable surfaces.
    ///
    /// This conversion is intentionally lossy when the body contains content
    /// items: image entries are dropped and text entries are joined with
    /// newlines.
    pub fn to_text(&self) -> Option<String> {
        match self {
            Self::Text(content) => Some(content.clone()),
            Self::ContentItems(items) => function_call_output_content_items_to_text(items),
        }
    }
}

impl Default for FunctionCallOutputBody {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

impl FunctionCallOutputPayload {
    pub fn from_text(content: String) -> Self {
        Self {
            body: FunctionCallOutputBody::Text(content),
            success: None,
        }
    }

    pub fn from_content_items(content_items: Vec<FunctionCallOutputContentItem>) -> Self {
        Self {
            body: FunctionCallOutputBody::ContentItems(content_items),
            success: None,
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        match &self.body {
            FunctionCallOutputBody::Text(content) => Some(content),
            FunctionCallOutputBody::ContentItems(_) => None,
        }
    }

    pub fn text_content_mut(&mut self) -> Option<&mut String> {
        match &mut self.body {
            FunctionCallOutputBody::Text(content) => Some(content),
            FunctionCallOutputBody::ContentItems(_) => None,
        }
    }

    pub fn content_items(&self) -> Option<&[FunctionCallOutputContentItem]> {
        match &self.body {
            FunctionCallOutputBody::Text(_) => None,
            FunctionCallOutputBody::ContentItems(items) => Some(items),
        }
    }

    pub fn content_items_mut(&mut self) -> Option<&mut Vec<FunctionCallOutputContentItem>> {
        match &mut self.body {
            FunctionCallOutputBody::Text(_) => None,
            FunctionCallOutputBody::ContentItems(items) => Some(items),
        }
    }
}

// `function_call_output.output` is encoded as either:
//   - an array of structured content items
//   - a plain string
impl Serialize for FunctionCallOutputPayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.body {
            FunctionCallOutputBody::Text(content) => serializer.serialize_str(content),
            FunctionCallOutputBody::ContentItems(items) => items.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for FunctionCallOutputPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let body = FunctionCallOutputBody::deserialize(deserializer)?;
        Ok(FunctionCallOutputPayload {
            body,
            success: None,
        })
    }
}

impl CallToolResult {
    pub fn from_result(result: Result<Self, String>) -> Self {
        match result {
            Ok(result) => result,
            Err(error) => Self::from_error_text(error),
        }
    }

    pub fn from_error_text(text: String) -> Self {
        Self {
            content: vec![serde_json::json!({
                "type": "text",
                "text": text,
            })],
            structured_content: None,
            is_error: Some(true),
            meta: None,
        }
    }

    pub fn success(&self) -> bool {
        self.is_error != Some(true)
    }

    pub fn as_function_call_output_payload(&self) -> FunctionCallOutputPayload {
        if let Some(structured_content) = &self.structured_content
            && !structured_content.is_null()
        {
            match serde_json::to_string(structured_content) {
                Ok(serialized_structured_content) => {
                    return FunctionCallOutputPayload {
                        body: FunctionCallOutputBody::Text(serialized_structured_content),
                        success: Some(self.success()),
                    };
                }
                Err(err) => {
                    return FunctionCallOutputPayload {
                        body: FunctionCallOutputBody::Text(err.to_string()),
                        success: Some(false),
                    };
                }
            }
        }

        let serialized_content = match serde_json::to_string(&self.content) {
            Ok(serialized_content) => serialized_content,
            Err(err) => {
                return FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(err.to_string()),
                    success: Some(false),
                };
            }
        };

        let content_items = convert_mcp_content_to_items(&self.content);

        let body = match content_items {
            Some(content_items) => FunctionCallOutputBody::ContentItems(content_items),
            None => FunctionCallOutputBody::Text(serialized_content),
        };

        FunctionCallOutputPayload {
            body,
            success: Some(self.success()),
        }
    }

    pub fn into_function_call_output_payload(self) -> FunctionCallOutputPayload {
        self.as_function_call_output_payload()
    }
}

fn convert_mcp_content_to_items(
    contents: &[serde_json::Value],
) -> Option<Vec<FunctionCallOutputContentItem>> {
    const CODEX_IMAGE_DETAIL_META_KEY: &str = "codex/imageDetail";

    #[derive(serde::Deserialize)]
    #[serde(tag = "type")]
    enum McpContent {
        #[serde(rename = "text")]
        Text { text: String },
        #[serde(rename = "image")]
        Image {
            data: String,
            #[serde(rename = "mimeType", alias = "mime_type")]
            mime_type: Option<String>,
            #[serde(rename = "_meta", default)]
            meta: Option<serde_json::Value>,
        },
        #[serde(other)]
        Unknown,
    }

    let mut saw_image = false;
    let mut items = Vec::with_capacity(contents.len());

    for content in contents {
        let item = match serde_json::from_value::<McpContent>(content.clone()) {
            Ok(McpContent::Text { text }) => FunctionCallOutputContentItem::InputText { text },
            Ok(McpContent::Image {
                data,
                mime_type,
                meta,
            }) => {
                saw_image = true;
                let image_url = if data.starts_with("data:") {
                    data
                } else {
                    let mime_type = mime_type.unwrap_or_else(|| "application/octet-stream".into());
                    format!("data:{mime_type};base64,{data}")
                };
                FunctionCallOutputContentItem::InputImage {
                    image_url,
                    detail: meta
                        .as_ref()
                        .and_then(serde_json::Value::as_object)
                        .and_then(|meta| meta.get(CODEX_IMAGE_DETAIL_META_KEY))
                        .and_then(serde_json::Value::as_str)
                        .and_then(|detail| match detail {
                            "auto" => Some(ImageDetail::Auto),
                            "low" => Some(ImageDetail::Low),
                            "high" => Some(ImageDetail::High),
                            "original" => Some(ImageDetail::Original),
                            _ => None,
                        })
                        .or(Some(DEFAULT_IMAGE_DETAIL)),
                }
            }
            Ok(McpContent::Unknown) | Err(_) => FunctionCallOutputContentItem::InputText {
                text: serde_json::to_string(content).unwrap_or_else(|_| "<content>".to_string()),
            },
        };
        items.push(item);
    }

    if saw_image { Some(items) } else { None }
}

// Implement Display so callers can treat the payload like a plain string when logging or doing
// trivial substring checks in tests (existing tests call `.contains()` on the output). For
// `ContentItems`, Display emits a JSON representation.

impl std::fmt::Display for FunctionCallOutputPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.body {
            FunctionCallOutputBody::Text(content) => f.write_str(content),
            FunctionCallOutputBody::ContentItems(items) => {
                let content = serde_json::to_string(items).unwrap_or_default();
                f.write_str(content.as_str())
            }
        }
    }
}

// (Moved event mapping logic into codex-core to avoid coupling protocol to UI-facing events.)

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use codex_execpolicy::Policy;
    use codex_permission_types::FileSystemAccessMode;
    use codex_permission_types::FileSystemPath;
    use codex_permission_types::FileSystemSandboxEntry;
    use codex_permission_types::FileSystemSandboxPolicy;
    use codex_permission_types::FileSystemSpecialPath;
    use codex_permission_types::NetworkSandboxPolicy;
    use codex_permission_types::SandboxPolicy;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use pretty_assertions::assert_eq;
    use std::num::NonZeroUsize;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn plaintext_agent_message_content_rejects_mixed_encrypted_content() {
        let content = vec![
            AgentMessageInputContent::InputText {
                text: "Message Type: MESSAGE\nPayload:\n".to_string(),
            },
            AgentMessageInputContent::EncryptedContent {
                encrypted_content: "encrypted-payload".to_string(),
            },
        ];

        assert_eq!(plaintext_agent_message_content(&content), None);
    }

    #[test]
    fn response_input_message_conversion_preserves_phase() {
        let item = ResponseItem::from(ResponseInputItem::Message {
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: "still working".to_string(),
            }],
            phase: Some(MessagePhase::Commentary),
        });

        assert_eq!(
            item,
            ResponseItem::Message {
                id: None,
                role: "assistant".to_string(),
                content: vec![ContentItem::OutputText {
                    text: "still working".to_string(),
                }],
                phase: Some(MessagePhase::Commentary),
                internal_chat_message_metadata_passthrough: None,
            }
        );
    }

    #[test]
    fn response_item_passthrough_metadata_round_trips_and_stamps_turn_ids() -> Result<()> {
        let mut item =
            response_item_with_passthrough_metadata(Some(passthrough_metadata("turn-1")));
        let round_trip: ResponseItem = serde_json::from_value(serde_json::to_value(&item)?)?;
        assert_eq!(round_trip, item);

        let unknown_metadata: ResponseItem = serde_json::from_value(serde_json::json!({
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "hello"}],
            "internal_chat_message_metadata_passthrough": {
                "turn_id": "turn-1",
                "other": "ignored",
            },
        }))?;
        assert_eq!(unknown_metadata, item);

        item.set_turn_id_if_missing("turn-2");
        assert_eq!(item.turn_id(), Some("turn-1"));

        let mut empty_turn_id =
            response_item_with_passthrough_metadata(Some(passthrough_metadata("")));
        empty_turn_id.set_turn_id_if_missing("turn-1");
        assert_eq!(empty_turn_id.turn_id(), Some("turn-1"));

        let mut missing_turn_id = response_item_with_passthrough_metadata(
            /*internal_chat_message_metadata_passthrough*/ None,
        );
        missing_turn_id.set_turn_id_if_missing("");
        missing_turn_id.set_turn_id_if_missing("turn-1");
        assert_eq!(missing_turn_id.turn_id(), Some("turn-1"));

        let mut other = ResponseItem::Other;
        other.set_turn_id_if_missing("turn-1");
        assert_eq!(other.turn_id(), None);
        Ok(())
    }

    #[test]
    fn response_item_id_getter_and_setter() {
        let mut item = response_item_with_passthrough_metadata(
            /*internal_chat_message_metadata_passthrough*/ None,
        );
        assert_eq!(item.id(), None);

        item.set_id(Some("msg_test".to_string()));

        assert_eq!(item.id(), Some("msg_test"));

        item.set_id(/*new_id*/ None);

        assert_eq!(item.id(), None);

        let mut additional_tools = ResponseItem::AdditionalTools {
            id: None,
            role: "developer".to_string(),
            tools: Vec::new(),
        };
        additional_tools.set_id(Some("at_test".to_string()));
        assert_eq!(additional_tools.id(), Some("at_test"));
    }

    fn response_item_with_passthrough_metadata(
        internal_chat_message_metadata_passthrough: Option<InternalChatMessageMetadataPassthrough>,
    ) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: "hello".to_string(),
            }],
            phase: None,
            internal_chat_message_metadata_passthrough,
        }
    }

    fn passthrough_metadata(turn_id: &str) -> InternalChatMessageMetadataPassthrough {
        InternalChatMessageMetadataPassthrough {
            turn_id: Some(turn_id.to_string()),
        }
    }

    #[test]
    fn image_detail_roundtrips_all_wire_values() -> Result<()> {
        assert_eq!(
            serde_json::from_str::<ImageDetail>("\"auto\"")?,
            ImageDetail::Auto
        );
        assert_eq!(
            serde_json::from_str::<ImageDetail>("\"low\"")?,
            ImageDetail::Low
        );
        assert_eq!(serde_json::to_string(&ImageDetail::Auto)?, "\"auto\"");
        assert_eq!(serde_json::to_string(&ImageDetail::Low)?, "\"low\"");

        let content_item: ContentItem = serde_json::from_value(serde_json::json!({
            "type": "input_image",
            "image_url": "data:image/png;base64,abc",
            "detail": "auto",
        }))?;

        assert_eq!(
            content_item,
            ContentItem::InputImage {
                image_url: "data:image/png;base64,abc".to_string(),
                detail: Some(ImageDetail::Auto),
            }
        );

        Ok(())
    }

    #[test]
    fn sandbox_permissions_helpers_match_documented_semantics() {
        let cases = [
            (SandboxPermissions::UseDefault, false, false, false),
            (SandboxPermissions::RequireEscalated, true, true, false),
            (
                SandboxPermissions::WithAdditionalPermissions,
                false,
                true,
                true,
            ),
        ];

        for (
            sandbox_permissions,
            requires_escalated_permissions,
            requests_sandbox_override,
            uses_additional_permissions,
        ) in cases
        {
            assert_eq!(
                sandbox_permissions.requires_escalated_permissions(),
                requires_escalated_permissions
            );
            assert_eq!(
                sandbox_permissions.requests_sandbox_override(),
                requests_sandbox_override
            );
            assert_eq!(
                sandbox_permissions.uses_additional_permissions(),
                uses_additional_permissions
            );
        }
    }

    #[test]
    fn convert_mcp_content_to_items_preserves_data_urls() {
        let contents = vec![serde_json::json!({
            "type": "image",
            "data": "data:image/png;base64,Zm9v",
            "mimeType": "image/png",
        })];

        let items = convert_mcp_content_to_items(&contents).expect("expected image items");
        assert_eq!(
            items,
            vec![FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,Zm9v".to_string(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            }]
        );
    }

    #[test]
    fn response_item_parses_image_generation_call() {
        let item = serde_json::from_value::<ResponseItem>(serde_json::json!({
            "id": "ig_123",
            "type": "image_generation_call",
            "status": "completed",
            "revised_prompt": "A small blue square",
            "result": "Zm9v",
        }))
        .expect("image generation item should deserialize");

        assert_eq!(
            item,
            ResponseItem::ImageGenerationCall {
                id: Some("ig_123".to_string()),
                status: "completed".to_string(),
                revised_prompt: Some("A small blue square".to_string()),
                result: "Zm9v".to_string(),
                internal_chat_message_metadata_passthrough: None,
            }
        );
    }

    #[test]
    fn response_item_parses_image_generation_call_without_revised_prompt() {
        let item = serde_json::from_value::<ResponseItem>(serde_json::json!({
            "id": "ig_123",
            "type": "image_generation_call",
            "status": "completed",
            "result": "Zm9v",
        }))
        .expect("image generation item should deserialize");

        assert_eq!(
            item,
            ResponseItem::ImageGenerationCall {
                id: Some("ig_123".to_string()),
                status: "completed".to_string(),
                revised_prompt: None,
                result: "Zm9v".to_string(),
                internal_chat_message_metadata_passthrough: None,
            }
        );
    }

    #[test]
    fn additional_permission_profile_is_empty_when_all_fields_are_none() {
        assert_eq!(AdditionalPermissionProfile::default().is_empty(), true);
    }

    #[test]
    fn additional_permission_profile_is_not_empty_when_field_is_present_but_nested_empty() {
        let permission_profile = AdditionalPermissionProfile {
            network: Some(NetworkPermissions { enabled: None }),
            file_system: None,
        };
        assert_eq!(permission_profile.is_empty(), false);
    }

    #[test]
    fn permission_profile_round_trip_preserves_glob_scan_max_depth() {
        let mut file_system_sandbox_policy =
            FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
                path: FileSystemPath::GlobPattern {
                    pattern: "**/*.env".to_string(),
                },
                access: FileSystemAccessMode::None,
            }]);
        file_system_sandbox_policy.glob_scan_max_depth = Some(2);

        let permission_profile = PermissionProfile::from_runtime_permissions(
            &file_system_sandbox_policy,
            NetworkSandboxPolicy::Restricted,
        );

        assert_eq!(
            permission_profile.file_system_sandbox_policy(),
            file_system_sandbox_policy
        );
    }

    #[test]
    fn permission_profile_deserializes_legacy_rollout_shape() -> Result<()> {
        let legacy = serde_json::json!({
            "network": {
                "enabled": true,
            },
            "file_system": {
                "entries": [{
                    "path": {
                        "type": "special",
                        "value": {
                            "kind": "root",
                        },
                    },
                    "access": "write",
                }],
                "glob_scan_max_depth": 2,
            },
        });

        let permission_profile: PermissionProfile = serde_json::from_value(legacy)?;

        assert_eq!(
            permission_profile,
            PermissionProfile::Managed {
                file_system: ManagedFileSystemPermissions::Restricted {
                    entries: vec![FileSystemSandboxEntry {
                        path: FileSystemPath::Special {
                            value: FileSystemSpecialPath::Root,
                        },
                        access: FileSystemAccessMode::Write,
                    }],
                    glob_scan_max_depth: NonZeroUsize::new(2),
                },
                network: NetworkSandboxPolicy::Enabled,
            }
        );
        Ok(())
    }

    #[test]
    fn permission_profile_presets_match_legacy_defaults() {
        assert_eq!(
            PermissionProfile::read_only(),
            PermissionProfile::from_legacy_sandbox_policy(&SandboxPolicy::new_read_only_policy())
        );
        assert_eq!(
            PermissionProfile::workspace_write(),
            PermissionProfile::from_legacy_sandbox_policy(
                &SandboxPolicy::new_workspace_write_policy()
            )
        );
    }

    #[test]
    fn permission_profile_round_trip_preserves_disabled_sandbox() -> Result<()> {
        let cwd = tempdir()?;
        let permission_profile =
            PermissionProfile::from_legacy_sandbox_policy(&SandboxPolicy::DangerFullAccess);

        assert_eq!(permission_profile, PermissionProfile::Disabled);
        assert_eq!(
            permission_profile.to_legacy_sandbox_policy(cwd.path())?,
            SandboxPolicy::DangerFullAccess
        );
        assert_eq!(
            permission_profile.to_runtime_permissions(),
            (
                FileSystemSandboxPolicy::unrestricted(),
                NetworkSandboxPolicy::Enabled
            )
        );
        Ok(())
    }

    #[test]
    fn disabled_permission_profile_ignores_runtime_network_policy() {
        let permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::Disabled,
            &FileSystemSandboxPolicy::unrestricted(),
            NetworkSandboxPolicy::Restricted,
        );

        assert_eq!(permission_profile, PermissionProfile::Disabled);
    }

    #[test]
    fn permission_profile_from_runtime_permissions_preserves_external_sandbox() {
        let permission_profile = PermissionProfile::from_runtime_permissions(
            &FileSystemSandboxPolicy::external_sandbox(),
            NetworkSandboxPolicy::Restricted,
        );

        assert_eq!(
            permission_profile,
            PermissionProfile::External {
                network: NetworkSandboxPolicy::Restricted,
            }
        );
        assert_eq!(
            PermissionProfile::from_runtime_permissions_with_enforcement(
                SandboxEnforcement::Managed,
                &FileSystemSandboxPolicy::external_sandbox(),
                NetworkSandboxPolicy::Restricted,
            ),
            permission_profile,
        );
    }

    #[test]
    fn permission_profile_from_runtime_permissions_preserves_unrestricted_managed_network() {
        let permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::External,
            &FileSystemSandboxPolicy::unrestricted(),
            NetworkSandboxPolicy::Restricted,
        );

        assert_eq!(
            permission_profile,
            PermissionProfile::Managed {
                file_system: ManagedFileSystemPermissions::Unrestricted,
                network: NetworkSandboxPolicy::Restricted,
            },
            "the legacy ExternalSandbox projection must not hide a split unrestricted filesystem policy"
        );
        assert_eq!(
            permission_profile.to_runtime_permissions(),
            (
                FileSystemSandboxPolicy::unrestricted(),
                NetworkSandboxPolicy::Restricted,
            )
        );
    }

    #[test]
    fn permission_profile_round_trip_preserves_external_sandbox() -> Result<()> {
        let cwd = tempdir()?;
        let sandbox_policy = SandboxPolicy::ExternalSandbox {
            network_access: crate::protocol::NetworkAccess::Restricted,
        };
        let permission_profile = PermissionProfile::from_legacy_sandbox_policy(&sandbox_policy);

        assert_eq!(
            permission_profile,
            PermissionProfile::External {
                network: NetworkSandboxPolicy::Restricted,
            }
        );
        assert_eq!(
            permission_profile.to_legacy_sandbox_policy(cwd.path())?,
            sandbox_policy
        );
        assert_eq!(
            permission_profile.to_runtime_permissions(),
            (
                FileSystemSandboxPolicy::external_sandbox(),
                NetworkSandboxPolicy::Restricted
            )
        );
        Ok(())
    }

    #[test]
    fn file_system_permissions_with_glob_scan_depth_uses_canonical_json() -> Result<()> {
        let path = AbsolutePathBuf::try_from(PathBuf::from(if cfg!(windows) {
            r"C:\tmp\allowed"
        } else {
            "/tmp/allowed"
        }))
        .expect("absolute path");
        let file_system_permissions = FileSystemPermissions {
            entries: vec![FileSystemSandboxEntry {
                path: FileSystemPath::Path { path },
                access: FileSystemAccessMode::Read,
            }],
            glob_scan_max_depth: NonZeroUsize::new(2),
        };

        let serialized = serde_json::to_value(&file_system_permissions)?;

        assert_eq!(serialized.get("read"), None);
        assert_eq!(serialized.get("write"), None);
        assert_eq!(
            serialized.get("glob_scan_max_depth"),
            Some(&serde_json::json!(2))
        );
        assert!(serialized.get("entries").is_some());
        assert_eq!(
            serde_json::from_value::<FileSystemPermissions>(serialized)?,
            file_system_permissions
        );
        Ok(())
    }

    #[test]
    fn file_system_permissions_rejects_zero_glob_scan_depth() {
        serde_json::from_value::<FileSystemPermissions>(serde_json::json!({
            "entries": [],
            "glob_scan_max_depth": 0,
        }))
        .expect_err("zero glob scan depth should fail deserialization");
    }

    #[test]
    fn convert_mcp_content_to_items_builds_data_urls_when_missing_prefix() {
        let contents = vec![serde_json::json!({
            "type": "image",
            "data": "Zm9v",
            "mimeType": "image/png",
        })];

        let items = convert_mcp_content_to_items(&contents).expect("expected image items");
        assert_eq!(
            items,
            vec![FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,Zm9v".to_string(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            }]
        );
    }

    #[test]
    fn convert_mcp_content_to_items_returns_none_without_images() {
        let contents = vec![serde_json::json!({
            "type": "text",
            "text": "hello",
        })];

        assert_eq!(convert_mcp_content_to_items(&contents), None);
    }

    #[test]
    fn function_call_output_content_items_to_text_joins_text_segments() {
        let content_items = vec![
            FunctionCallOutputContentItem::InputText {
                text: "line 1".to_string(),
            },
            FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,AAA".to_string(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
            FunctionCallOutputContentItem::InputText {
                text: "line 2".to_string(),
            },
        ];

        let text = function_call_output_content_items_to_text(&content_items);
        assert_eq!(text, Some("line 1\nline 2".to_string()));
    }

    #[test]
    fn function_call_output_content_items_to_text_ignores_blank_text_and_images() {
        let content_items = vec![
            FunctionCallOutputContentItem::InputText {
                text: "   ".to_string(),
            },
            FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,AAA".to_string(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
            FunctionCallOutputContentItem::EncryptedContent {
                encrypted_content: "enc_opaque".to_string(),
            },
        ];

        let text = function_call_output_content_items_to_text(&content_items);
        assert_eq!(text, None);
    }

    #[test]
    fn function_call_output_body_to_text_returns_plain_text_content() {
        let body = FunctionCallOutputBody::Text("ok".to_string());
        let text = body.to_text();
        assert_eq!(text, Some("ok".to_string()));
    }

    #[test]
    fn function_call_output_body_to_text_uses_content_item_fallback() {
        let body = FunctionCallOutputBody::ContentItems(vec![
            FunctionCallOutputContentItem::InputText {
                text: "line 1".to_string(),
            },
            FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,AAA".to_string(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
        ]);

        let text = body.to_text();
        assert_eq!(text, Some("line 1".to_string()));
    }

    #[test]
    fn function_call_deserializes_optional_namespace() {
        let item: ResponseItem = serde_json::from_value(serde_json::json!({
            "type": "function_call",
            "name": "mcp__codex_apps__gmail_get_recent_emails",
            "namespace": "mcp__codex_apps__gmail",
            "arguments": "{\"top_k\":5}",
            "call_id": "call-1",
        }))
        .expect("function_call should deserialize");

        assert_eq!(
            item,
            ResponseItem::FunctionCall {
                id: None,
                name: "mcp__codex_apps__gmail_get_recent_emails".to_string(),
                namespace: Some("mcp__codex_apps__gmail".to_string()),
                arguments: "{\"top_k\":5}".to_string(),
                call_id: "call-1".to_string(),
                internal_chat_message_metadata_passthrough: None,
            }
        );
    }

    #[test]
    fn render_command_prefix_list_sorts_by_len_then_total_len_then_alphabetical() {
        let prefixes = vec![
            vec!["b".to_string(), "zz".to_string()],
            vec!["aa".to_string()],
            vec!["b".to_string()],
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["a".to_string()],
            vec!["b".to_string(), "a".to_string()],
        ];

        let output = format_allow_prefixes(prefixes).expect("rendered list");
        assert_eq!(
            output,
            r#"- ["a"]
- ["b"]
- ["aa"]
- ["b", "a"]
- ["b", "zz"]
- ["a", "b", "c"]"#
                .to_string(),
        );
    }

    #[test]
    fn render_command_prefix_list_limits_output_to_max_prefixes() {
        let prefixes = (0..(MAX_RENDERED_PREFIXES + 5))
            .map(|i| vec![format!("{i:03}")])
            .collect::<Vec<_>>();

        let output = format_allow_prefixes(prefixes).expect("rendered list");
        assert_eq!(output.ends_with(TRUNCATED_MARKER), true);
        eprintln!("output: {output}");
        assert_eq!(output.lines().count(), MAX_RENDERED_PREFIXES + 1);
    }

    #[test]
    fn format_allow_prefixes_limits_output() {
        let mut exec_policy = Policy::empty();
        for i in 0..200 {
            exec_policy
                .add_prefix_rule(
                    &[format!("tool-{i:03}"), "x".repeat(500)],
                    codex_execpolicy::Decision::Allow,
                )
                .expect("add rule");
        }

        let output =
            format_allow_prefixes(exec_policy.get_allowed_prefixes()).expect("formatted prefixes");
        assert!(
            output.len() <= MAX_ALLOW_PREFIX_TEXT_BYTES + TRUNCATED_MARKER.len(),
            "output length exceeds expected limit: {output}",
        );
    }

    #[test]
    fn serializes_success_as_plain_string() -> Result<()> {
        let item = ResponseInputItem::FunctionCallOutput {
            call_id: "call1".into(),
            output: FunctionCallOutputPayload::from_text("ok".into()),
        };

        let json = serde_json::to_string(&item)?;
        let v: serde_json::Value = serde_json::from_str(&json)?;

        // Success case -> output should be a plain string
        assert_eq!(v.get("output").unwrap().as_str().unwrap(), "ok");
        Ok(())
    }

    #[test]
    fn serializes_failure_as_string() -> Result<()> {
        let item = ResponseInputItem::FunctionCallOutput {
            call_id: "call1".into(),
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::Text("bad".into()),
                success: Some(false),
            },
        };

        let json = serde_json::to_string(&item)?;
        let v: serde_json::Value = serde_json::from_str(&json)?;

        assert_eq!(v.get("output").unwrap().as_str().unwrap(), "bad");
        Ok(())
    }

    #[test]
    fn serializes_image_outputs_as_array() -> Result<()> {
        let call_tool_result = CallToolResult {
            content: vec![
                serde_json::json!({"type":"text","text":"caption"}),
                serde_json::json!({"type":"image","data":"BASE64","mimeType":"image/png"}),
            ],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        };

        let payload = call_tool_result.into_function_call_output_payload();
        assert_eq!(payload.success, Some(true));
        let Some(items) = payload.content_items() else {
            panic!("expected content items");
        };
        let items = items.to_vec();
        assert_eq!(
            items,
            vec![
                FunctionCallOutputContentItem::InputText {
                    text: "caption".into(),
                },
                FunctionCallOutputContentItem::InputImage {
                    image_url: "data:image/png;base64,BASE64".into(),
                    detail: Some(DEFAULT_IMAGE_DETAIL),
                },
            ]
        );

        let item = ResponseInputItem::FunctionCallOutput {
            call_id: "call1".into(),
            output: payload,
        };

        let json = serde_json::to_string(&item)?;
        let v: serde_json::Value = serde_json::from_str(&json)?;

        let output = v.get("output").expect("output field");
        assert!(output.is_array(), "expected array output");

        Ok(())
    }

    #[test]
    fn serializes_custom_tool_image_outputs_as_array() -> Result<()> {
        let item = ResponseInputItem::CustomToolCallOutput {
            call_id: "call1".into(),
            name: None,
            output: FunctionCallOutputPayload::from_content_items(vec![
                FunctionCallOutputContentItem::InputImage {
                    image_url: "data:image/png;base64,BASE64".into(),
                    detail: Some(DEFAULT_IMAGE_DETAIL),
                },
            ]),
        };

        let json = serde_json::to_string(&item)?;
        let v: serde_json::Value = serde_json::from_str(&json)?;

        let output = v.get("output").expect("output field");
        assert!(output.is_array(), "expected array output");

        Ok(())
    }

    #[test]
    fn serializes_encrypted_function_output_content_as_array() -> Result<()> {
        let item = ResponseInputItem::FunctionCallOutput {
            call_id: "call1".into(),
            output: FunctionCallOutputPayload::from_content_items(vec![
                FunctionCallOutputContentItem::EncryptedContent {
                    encrypted_content: "enc_opaque".into(),
                },
            ]),
        };

        let json = serde_json::to_value(&item)?;
        assert_eq!(
            json,
            serde_json::json!({
                "type": "function_call_output",
                "call_id": "call1",
                "output": [
                    {
                        "type": "encrypted_content",
                        "encrypted_content": "enc_opaque",
                    }
                ],
            })
        );

        Ok(())
    }

    #[test]
    fn preserves_existing_image_data_urls() -> Result<()> {
        let call_tool_result = CallToolResult {
            content: vec![serde_json::json!({
                "type": "image",
                "data": "data:image/png;base64,BASE64",
                "mimeType": "image/png"
            })],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        };

        let payload = call_tool_result.into_function_call_output_payload();
        let Some(items) = payload.content_items() else {
            panic!("expected content items");
        };
        let items = items.to_vec();
        assert_eq!(
            items,
            vec![FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,BASE64".into(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            }]
        );

        Ok(())
    }

    #[test]
    fn preserves_original_detail_metadata_on_mcp_images() -> Result<()> {
        let call_tool_result = CallToolResult {
            content: vec![serde_json::json!({
                "type": "image",
                "data": "BASE64",
                "mimeType": "image/png",
                "_meta": {
                    "codex/imageDetail": "original",
                },
            })],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        };

        let payload = call_tool_result.into_function_call_output_payload();
        let Some(items) = payload.content_items() else {
            panic!("expected content items");
        };
        let items = items.to_vec();
        assert_eq!(
            items,
            vec![FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,BASE64".into(),
                detail: Some(ImageDetail::Original),
            }]
        );

        Ok(())
    }

    #[test]
    fn preserves_standard_detail_metadata_on_mcp_images() -> Result<()> {
        let call_tool_result = CallToolResult {
            content: vec![serde_json::json!({
                "type": "image",
                "data": "BASE64",
                "mimeType": "image/png",
                "_meta": {
                    "codex/imageDetail": "high",
                },
            })],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        };

        let payload = call_tool_result.into_function_call_output_payload();
        let Some(items) = payload.content_items() else {
            panic!("expected content items");
        };
        let items = items.to_vec();
        assert_eq!(
            items,
            vec![FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,BASE64".into(),
                detail: Some(ImageDetail::High),
            }]
        );

        Ok(())
    }

    #[test]
    fn deserializes_array_payload_into_items() -> Result<()> {
        let json = r#"[
            {"type": "input_text", "text": "note"},
            {"type": "input_image", "image_url": "data:image/png;base64,XYZ"}
        ]"#;

        let payload: FunctionCallOutputPayload = serde_json::from_str(json)?;

        assert_eq!(payload.success, None);
        let expected_items = vec![
            FunctionCallOutputContentItem::InputText {
                text: "note".into(),
            },
            FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,XYZ".into(),
                detail: None,
            },
        ];
        assert_eq!(
            payload.body,
            FunctionCallOutputBody::ContentItems(expected_items.clone())
        );
        assert_eq!(
            serde_json::to_string(&payload)?,
            serde_json::to_string(&expected_items)?
        );

        Ok(())
    }

    #[test]
    fn deserializes_encrypted_array_payload_into_items() -> Result<()> {
        let json = r#"[
            {"type": "encrypted_content", "encrypted_content": "enc_opaque"}
        ]"#;

        let payload: FunctionCallOutputPayload = serde_json::from_str(json)?;
        let expected_items = vec![FunctionCallOutputContentItem::EncryptedContent {
            encrypted_content: "enc_opaque".into(),
        }];

        assert_eq!(payload.success, None);
        assert_eq!(
            payload.body,
            FunctionCallOutputBody::ContentItems(expected_items.clone())
        );
        assert_eq!(
            serde_json::to_string(&payload)?,
            serde_json::to_string(&expected_items)?
        );

        Ok(())
    }

    #[test]
    fn deserializes_compaction_alias() -> Result<()> {
        let json = r#"{"type":"compaction_summary","encrypted_content":"abc"}"#;

        let item: ResponseItem = serde_json::from_str(json)?;

        assert_eq!(
            item,
            ResponseItem::Compaction {
                id: None,
                encrypted_content: "abc".into(),
                internal_chat_message_metadata_passthrough: None,
            }
        );
        Ok(())
    }

    #[test]
    fn deserializes_context_compaction() -> Result<()> {
        let json = r#"{"type":"context_compaction","encrypted_content":"abc"}"#;

        let item: ResponseItem = serde_json::from_str(json)?;

        assert_eq!(
            item,
            ResponseItem::ContextCompaction {
                id: None,
                encrypted_content: Some("abc".into()),
                internal_chat_message_metadata_passthrough: None,
            }
        );
        Ok(())
    }

    #[test]
    fn serializes_context_compaction_trigger_without_payload() -> Result<()> {
        let item = ResponseItem::ContextCompaction {
            encrypted_content: None,
            id: None,
            metadata: None,
        };

        assert_eq!(
            serde_json::to_value(item)?,
            serde_json::json!({
                "type": "context_compaction",
            })
        );
        Ok(())
    }

    #[test]
    fn serializes_compaction_trigger_without_payload() -> Result<()> {
        let item = ResponseItem::CompactionTrigger {};

        assert_eq!(
            serde_json::to_value(item)?,
            serde_json::json!({
                "type": "compaction_trigger",
            })
        );
        Ok(())
    }

    #[test]
    fn deserializes_compaction_trigger_without_payload() -> Result<()> {
        let json = r#"{"type":"compaction_trigger"}"#;

        let item: ResponseItem = serde_json::from_str(json)?;

        assert_eq!(item, ResponseItem::CompactionTrigger {});
        Ok(())
    }

    #[test]
    fn deserializes_legacy_ghost_snapshot_as_other() -> Result<()> {
        let json = r#"{
            "type":"ghost_snapshot",
            "ghost_commit":{
                "id":"ghost-1",
                "parent":null,
                "preexisting_untracked_files":[],
                "preexisting_untracked_dirs":[]
            }
        }"#;

        let item: ResponseItem = serde_json::from_str(json)?;

        assert_eq!(item, ResponseItem::Other);
        Ok(())
    }

    #[test]
    fn roundtrips_web_search_call_actions() -> Result<()> {
        let cases = vec![
            (
                r#"{
                    "type": "web_search_call",
                    "status": "completed",
                    "action": {
                        "type": "search",
                        "query": "weather seattle",
                        "queries": ["weather seattle", "seattle weather now"]
                    }
                }"#,
                None,
                Some(WebSearchAction::Search {
                    query: Some("weather seattle".into()),
                    queries: Some(vec!["weather seattle".into(), "seattle weather now".into()]),
                }),
                Some("completed".into()),
            ),
            (
                r#"{
                    "type": "web_search_call",
                    "status": "open",
                    "action": {
                        "type": "open_page",
                        "url": "https://example.com"
                    }
                }"#,
                None,
                Some(WebSearchAction::OpenPage {
                    url: Some("https://example.com".into()),
                }),
                Some("open".into()),
            ),
            (
                r#"{
                    "type": "web_search_call",
                    "status": "in_progress",
                    "action": {
                        "type": "find_in_page",
                        "url": "https://example.com/docs",
                        "pattern": "installation"
                    }
                }"#,
                None,
                Some(WebSearchAction::FindInPage {
                    url: Some("https://example.com/docs".into()),
                    pattern: Some("installation".into()),
                }),
                Some("in_progress".into()),
            ),
            (
                r#"{
                    "type": "web_search_call",
                    "status": "in_progress",
                    "id": "ws_partial"
                }"#,
                Some("ws_partial".into()),
                None,
                Some("in_progress".into()),
            ),
        ];

        for (json_literal, expected_id, expected_action, expected_status) in cases {
            let parsed: ResponseItem = serde_json::from_str(json_literal)?;
            let expected = ResponseItem::WebSearchCall {
                id: expected_id.clone(),
                status: expected_status.clone(),
                action: expected_action.clone(),
                internal_chat_message_metadata_passthrough: None,
            };
            assert_eq!(parsed, expected);

            let serialized = serde_json::to_value(&parsed)?;
            let expected_serialized: serde_json::Value = serde_json::from_str(json_literal)?;
            assert_eq!(serialized, expected_serialized);
        }

        Ok(())
    }

    #[test]
    fn serializes_image_user_input_without_tags() -> Result<()> {
        let image_url = "data:image/png;base64,abc".to_string();

        let item = ResponseInputItem::from(vec![UserInput::Image {
            image_url: image_url.clone(),
        }]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                let expected = vec![ContentItem::InputImage {
                    image_url,
                    detail: Some(DEFAULT_IMAGE_DETAIL),
                }];
                assert_eq!(content, expected);
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn image_user_input_preserves_requested_detail() -> Result<()> {
        let image_url = "data:image/png;base64,abc".to_string();

        let item = ResponseInputItem::from(vec![UserInput::Image {
            image_url: image_url.clone(),
            detail: Some(ImageDetail::Original),
        }]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                assert_eq!(
                    content.first(),
                    Some(&ContentItem::InputImage {
                        image_url,
                        detail: Some(ImageDetail::Original),
                    })
                );
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn tool_search_call_roundtrips() -> Result<()> {
        let parsed: ResponseItem = serde_json::from_str(
            r#"{
                "type": "tool_search_call",
                "call_id": "search-1",
                "execution": "client",
                "arguments": {
                    "query": "calendar create",
                    "limit": 1
                }
            }"#,
        )?;

        assert_eq!(
            parsed,
            ResponseItem::ToolSearchCall {
                id: None,
                call_id: Some("search-1".to_string()),
                status: None,
                execution: "client".to_string(),
                arguments: serde_json::json!({
                    "query": "calendar create",
                    "limit": 1,
                }),
                internal_chat_message_metadata_passthrough: None,
            }
        );

        assert_eq!(
            serde_json::to_value(&parsed)?,
            serde_json::json!({
                "type": "tool_search_call",
                "call_id": "search-1",
                "execution": "client",
                "arguments": {
                    "query": "calendar create",
                    "limit": 1,
                }
            })
        );

        Ok(())
    }

    #[test]
    fn tool_search_output_roundtrips() -> Result<()> {
        let input = ResponseInputItem::ToolSearchOutput {
            call_id: "search-1".to_string(),
            status: "completed".to_string(),
            execution: "client".to_string(),
            tools: vec![serde_json::json!({
                "type": "function",
                "name": "mcp__codex_apps__calendar_create_event",
                "description": "Create a calendar event.",
                "defer_loading": true,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"}
                    },
                    "required": ["title"],
                    "additionalProperties": false,
                }
            })],
        };
        assert_eq!(
            ResponseItem::from(input.clone()),
            ResponseItem::ToolSearchOutput {
                id: None,
                call_id: Some("search-1".to_string()),
                status: "completed".to_string(),
                execution: "client".to_string(),
                tools: vec![serde_json::json!({
                    "type": "function",
                    "name": "mcp__codex_apps__calendar_create_event",
                    "description": "Create a calendar event.",
                    "defer_loading": true,
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "title": {"type": "string"}
                        },
                        "required": ["title"],
                        "additionalProperties": false,
                    }
                })],
                internal_chat_message_metadata_passthrough: None,
            }
        );

        assert_eq!(
            serde_json::to_value(input)?,
            serde_json::json!({
                "type": "tool_search_output",
                "call_id": "search-1",
                "status": "completed",
                "execution": "client",
                "tools": [{
                    "type": "function",
                    "name": "mcp__codex_apps__calendar_create_event",
                    "description": "Create a calendar event.",
                    "defer_loading": true,
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "title": {"type": "string"}
                        },
                        "required": ["title"],
                        "additionalProperties": false,
                    }
                }]
            })
        );

        Ok(())
    }

    #[test]
    fn tool_search_server_items_allow_null_call_id() -> Result<()> {
        let parsed_call: ResponseItem = serde_json::from_str(
            r#"{
                "type": "tool_search_call",
                "execution": "server",
                "call_id": null,
                "status": "completed",
                "arguments": {
                    "paths": ["crm"]
                }
            }"#,
        )?;
        assert_eq!(
            parsed_call,
            ResponseItem::ToolSearchCall {
                id: None,
                call_id: None,
                status: Some("completed".to_string()),
                execution: "server".to_string(),
                arguments: serde_json::json!({
                    "paths": ["crm"],
                }),
                internal_chat_message_metadata_passthrough: None,
            }
        );

        let parsed_output: ResponseItem = serde_json::from_str(
            r#"{
                "type": "tool_search_output",
                "execution": "server",
                "call_id": null,
                "status": "completed",
                "tools": []
            }"#,
        )?;
        assert_eq!(
            parsed_output,
            ResponseItem::ToolSearchOutput {
                id: None,
                call_id: None,
                status: "completed".to_string(),
                execution: "server".to_string(),
                tools: vec![],
                internal_chat_message_metadata_passthrough: None,
            }
        );

        Ok(())
    }

    #[test]
    fn mixed_remote_and_local_images_share_label_sequence() -> Result<()> {
        let image_url = "data:image/png;base64,abc".to_string();
        let dir = tempdir()?;
        let local_path = dir.path().join("local.png");
        // A tiny valid PNG (1x1) so this test doesn't depend on cross-crate file paths, which
        // break under Bazel sandboxing.
        const TINY_PNG_BYTES: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 11, 73, 68, 65, 84, 120, 156, 99, 96, 0, 2,
            0, 0, 5, 0, 1, 122, 94, 171, 63, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ];
        std::fs::write(&local_path, TINY_PNG_BYTES)?;

        let item = ResponseInputItem::from(vec![
            UserInput::Image {
                image_url: image_url.clone(),
                detail: None,
            },
            UserInput::LocalImage {
                path: local_path.clone(),
                detail: None,
            },
        ]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                assert_eq!(
                    content.first(),
                    Some(&ContentItem::InputImage {
                        image_url,
                        detail: Some(DEFAULT_IMAGE_DETAIL),
                    })
                );
                assert_eq!(
                    content.get(1),
                    Some(&ContentItem::InputText {
                        text: local_image_open_tag_text_with_path(
                            /*label_number*/ 2,
                            &local_path
                        ),
                    })
                );
                assert!(matches!(
                    content.get(2),
                    Some(ContentItem::InputImage { .. })
                ));
                assert_eq!(
                    content.get(3),
                    Some(&ContentItem::InputText {
                        text: image_close_tag_text(),
                    })
                );
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn local_image_open_tag_preserves_path() {
        assert_eq!(
            local_image_open_tag_text_with_path(
                /*label_number*/ 1,
                std::path::Path::new(r#"/tmp/a&"<b>.png"#),
            ),
            r#"<image name=[Image #1] path="/tmp/a&"<b>.png">"#
        );
    }

    #[test]
    fn local_image_user_input_preserves_requested_detail() -> Result<()> {
        let dir = tempdir()?;
        let local_path = dir.path().join("local.png");
        std::fs::write(&local_path, TINY_PNG_BYTES)?;

        let item = ResponseInputItem::from(vec![UserInput::LocalImage {
            path: local_path,
            detail: Some(ImageDetail::Original),
        }]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                assert!(matches!(
                    content.get(1),
                    Some(ContentItem::InputImage {
                        detail: Some(ImageDetail::Original),
                        ..
                    })
                ));
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn local_image_read_error_adds_placeholder() -> Result<()> {
        let dir = tempdir()?;
        let missing_path = dir.path().join("missing-image.png");

        let item = ResponseInputItem::from(vec![UserInput::LocalImage {
            path: missing_path.clone(),
            detail: None,
        }]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    ContentItem::InputText { text } => {
                        let display_path = missing_path.display().to_string();
                        assert!(
                            text.contains(&display_path),
                            "placeholder should mention missing path: {text}"
                        );
                        assert!(
                            text.contains("could not read"),
                            "placeholder should mention read issue: {text}"
                        );
                    }
                    other => panic!("expected placeholder text but found {other:?}"),
                }
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn local_image_non_image_adds_placeholder() -> Result<()> {
        let dir = tempdir()?;
        let json_path = dir.path().join("example.json");
        std::fs::write(&json_path, br#"{"hello":"world"}"#)?;

        let item = ResponseInputItem::from(vec![UserInput::LocalImage {
            path: json_path.clone(),
            detail: None,
        }]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    ContentItem::InputText { text } => {
                        assert!(
                            text.contains("unsupported image `application/json`"),
                            "placeholder should mention unsupported image MIME: {text}"
                        );
                        assert!(
                            text.contains(&json_path.display().to_string()),
                            "placeholder should mention path: {text}"
                        );
                    }
                    other => panic!("expected placeholder text but found {other:?}"),
                }
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn local_image_unsupported_image_format_adds_placeholder() -> Result<()> {
        let dir = tempdir()?;
        let svg_path = dir.path().join("example.svg");
        std::fs::write(
            &svg_path,
            br#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"></svg>"#,
        )?;

        let item = ResponseInputItem::from(vec![UserInput::LocalImage {
            path: svg_path.clone(),
            detail: None,
        }]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                assert_eq!(content.len(), 1);
                let expected = format!(
                    "Codex cannot attach image at `{}`: unsupported image `image/svg+xml`.",
                    svg_path.display()
                );
                match &content[0] {
                    ContentItem::InputText { text } => assert_eq!(text, &expected),
                    other => panic!("expected placeholder text but found {other:?}"),
                }
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }
}
