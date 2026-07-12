//! Tool execution observation and telemetry abstractions.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

mod tool_config;

pub use codex_protocol::ToolName;
pub use tool_config::ToolEnvironmentMode;
pub use tool_config::ToolUserShellType;
pub use tool_config::ToolsConfig;
pub use tool_config::ToolsConfigParams;

use codex_protocol::models::DEFAULT_IMAGE_DETAIL;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ImageDetail;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::openai_models::ModelInfo;
use codex_tool_registry_api::ToolExposure;
use codex_tool_registry_api::ToolSpec;
use codex_utils_absolute_path::AbsolutePathBuf;
use serde_json::Value as JsonValue;
use std::borrow::Cow;
use std::future::Future;
use std::path::Path;
use thiserror::Error;

/// Backend implementation selected for shell-command execution.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ShellCommandBackendConfig {
    Classic,
    ZshFork,
}

/// Shell execution mode used by the unified exec tool.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UnifiedExecShellMode {
    Direct,
    ZshFork(ZshForkConfig),
}

/// Paths required to execute commands through the zsh-fork backend.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ZshForkConfig {
    pub shell_zsh_path: AbsolutePathBuf,
    pub main_execve_wrapper_exe: AbsolutePathBuf,
}

impl ZshForkConfig {
    pub fn new(shell_zsh_path: AbsolutePathBuf, main_execve_wrapper_exe: AbsolutePathBuf) -> Self {
        Self {
            shell_zsh_path,
            main_execve_wrapper_exe,
        }
    }

    pub fn shell_zsh_path(&self) -> &Path {
        self.shell_zsh_path.as_path()
    }

    pub fn main_execve_wrapper_exe(&self) -> &Path {
        self.main_execve_wrapper_exe.as_path()
    }
}

/// Origin of a tool call within the current turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolCallSource {
    /// Tool call was requested directly by the model.
    Direct,
    /// Tool call was requested by a code-mode runtime cell.
    CodeMode {
        /// Runtime cell that issued the nested tool request.
        cell_id: String,
        /// Code-mode's per-cell tool invocation id. This is useful for
        /// debugging the JS/runtime bridge, but it is not the Codex tool call id
        /// because the runtime id only needs to be unique within one cell.
        runtime_tool_call_id: String,
    },
}

/// Canonical payload shapes accepted by model-visible tool runtimes.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolPayload {
    Function { arguments: String },
    ToolSearch { arguments: ToolSearchArguments },
    Custom { input: String },
}

impl ToolPayload {
    pub fn log_payload(&self) -> Cow<'_, str> {
        match self {
            ToolPayload::Function { arguments } => Cow::Borrowed(arguments),
            ToolPayload::ToolSearch { arguments } => Cow::Owned(arguments.query.clone()),
            ToolPayload::Custom { input } => Cow::Borrowed(input),
        }
    }
}

/// Domain-owned tool search arguments shared by core and tool runtimes.
///
/// Keeping this shape in `tool-execution-api` avoids leaking protocol model
/// types into the core tool runtime boundary.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ToolSearchArguments {
    pub query: String,
    pub limit: Option<usize>,
}

impl From<ToolSearchArguments> for codex_protocol::models::SearchToolCallParams {
    fn from(value: ToolSearchArguments) -> Self {
        Self {
            query: value.query,
            limit: value.limit,
        }
    }
}

impl From<codex_protocol::models::SearchToolCallParams> for ToolSearchArguments {
    fn from(value: codex_protocol::models::SearchToolCallParams) -> Self {
        Self {
            query: value.query,
            limit: value.limit,
        }
    }
}

/// Error returned while executing a model-visible tool invocation.
#[derive(Debug, Error, PartialEq)]
pub enum FunctionCallError {
    #[error("{0}")]
    RespondToModel(String),
    #[error("Fatal error: {0}")]
    Fatal(String),
}

pub fn can_request_original_image_detail(model_info: &ModelInfo) -> bool {
    model_info.supports_image_detail_original
}

pub fn normalize_output_image_detail(
    model_info: &ModelInfo,
    detail: Option<ImageDetail>,
) -> Option<ImageDetail> {
    match detail {
        Some(ImageDetail::Original) if can_request_original_image_detail(model_info) => {
            Some(ImageDetail::Original)
        }
        Some(ImageDetail::Original) | None => None,
        Some(detail @ (ImageDetail::Auto | ImageDetail::Low | ImageDetail::High)) => Some(detail),
    }
}

pub fn sanitize_original_image_detail(
    can_request_original_image_detail: bool,
    items: &mut [FunctionCallOutputContentItem],
) {
    if can_request_original_image_detail {
        return;
    }

    for item in items {
        if let FunctionCallOutputContentItem::InputImage { detail, .. } = item
            && matches!(detail, Some(ImageDetail::Original))
        {
            *detail = Some(DEFAULT_IMAGE_DETAIL);
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToolCall {
    pub call_id: String,
    pub tool_name: ToolName,
    pub payload: ToolPayload,
}

impl ToolCall {
    pub fn function_arguments(&self) -> Result<&str, FunctionCallError> {
        match &self.payload {
            ToolPayload::Function { arguments } => Ok(arguments),
            _ => Err(FunctionCallError::Fatal(format!(
                "tool {} invoked with incompatible payload",
                self.tool_name
            ))),
        }
    }
}

/// Maximum bytes included in the default telemetry preview.
pub const TELEMETRY_PREVIEW_MAX_BYTES: usize = 2 * 1024;
/// Maximum lines included in the default telemetry preview.
pub const TELEMETRY_PREVIEW_MAX_LINES: usize = 64;
/// Notice appended when the default telemetry preview is truncated.
pub const TELEMETRY_PREVIEW_TRUNCATION_NOTICE: &str = "[... telemetry preview truncated ...]";

/// Truncation policy for model-visible tool output telemetry previews.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TelemetryPreviewPolicy {
    /// Maximum bytes to include before truncating.
    pub max_bytes: usize,
    /// Maximum lines to include before truncating.
    pub max_lines: usize,
    /// Notice appended when content is truncated.
    pub truncation_notice: &'static str,
}

/// Default preview policy used by tool runtimes.
pub const DEFAULT_TELEMETRY_PREVIEW_POLICY: TelemetryPreviewPolicy = TelemetryPreviewPolicy {
    max_bytes: TELEMETRY_PREVIEW_MAX_BYTES,
    max_lines: TELEMETRY_PREVIEW_MAX_LINES,
    truncation_notice: TELEMETRY_PREVIEW_TRUNCATION_NOTICE,
};

impl TelemetryPreviewPolicy {
    /// Builds a telemetry preview for `content` using this policy.
    pub fn preview(&self, content: &str) -> String {
        let truncated_slice = take_bytes_at_char_boundary(content, self.max_bytes);
        let truncated_by_bytes = truncated_slice.len() < content.len();

        let mut preview = String::new();
        let mut lines_iter = truncated_slice.lines();
        for idx in 0..self.max_lines {
            match lines_iter.next() {
                Some(line) => {
                    if idx > 0 {
                        preview.push('\n');
                    }
                    preview.push_str(line);
                }
                None => break,
            }
        }
        let truncated_by_lines = lines_iter.next().is_some();

        if !truncated_by_bytes && !truncated_by_lines {
            return content.to_string();
        }

        if preview.len() < truncated_slice.len()
            && truncated_slice
                .as_bytes()
                .get(preview.len())
                .is_some_and(|byte| *byte == b'\n')
        {
            preview.push('\n');
        }

        if !preview.is_empty() && !preview.ends_with('\n') {
            preview.push('\n');
        }
        preview.push_str(self.truncation_notice);

        preview
    }
}

/// Builds a telemetry preview with the default tool-output policy.
pub fn telemetry_preview(content: &str) -> String {
    DEFAULT_TELEMETRY_PREVIEW_POLICY.preview(content)
}

fn take_bytes_at_char_boundary(content: &str, max_bytes: usize) -> &str {
    if content.len() <= max_bytes {
        return content;
    }

    let mut end = max_bytes;
    while end > 0 && !content.is_char_boundary(end) {
        end -= 1;
    }
    &content[..end]
}

/// Minimal payload context needed to turn tool output into model-facing responses.
///
/// Concrete tool runtimes can keep richer invocation payloads in their own crates,
/// while shared output adapters depend only on the response shape decisions needed
/// at this boundary.
pub trait ToolOutputPayload {
    /// Returns true when the output should be emitted as custom-tool output.
    fn is_custom(&self) -> bool;

    /// Returns true when the output belongs to a tool-search call.
    fn is_tool_search(&self) -> bool;
}

impl ToolOutputPayload for ToolPayload {
    fn is_custom(&self) -> bool {
        matches!(self, ToolPayload::Custom { .. })
    }

    fn is_tool_search(&self) -> bool {
        matches!(self, ToolPayload::ToolSearch { .. })
    }
}

/// Model-facing output contract returned by executable tool runtimes.
pub trait ToolOutput: Send {
    fn log_preview(&self) -> String;

    fn success_for_logging(&self) -> bool;

    fn to_response_item(&self, call_id: &str, payload: &dyn ToolOutputPayload)
    -> ResponseInputItem;

    /// Returns the tool call id exposed to `PostToolUse` hooks for this output.
    fn post_tool_use_id(&self, call_id: &str) -> String {
        call_id.to_string()
    }

    /// Returns the tool input exposed to `PostToolUse` hooks for this output.
    fn post_tool_use_input(&self, _payload: &dyn ToolOutputPayload) -> Option<JsonValue> {
        None
    }

    /// Returns the stable value exposed to `PostToolUse` hooks for this tool output.
    ///
    /// Tool handlers decide whether a tool participates in `PostToolUse`, but
    /// this method lets the output type own any conversion from model-facing
    /// response content to hook-facing data. Returning `None` means the output
    /// should not produce a post-use hook payload, not merely that the tool had
    /// empty output.
    fn post_tool_use_response(
        &self,
        _call_id: &str,
        _payload: &dyn ToolOutputPayload,
    ) -> Option<JsonValue> {
        None
    }

    fn code_mode_result(&self, payload: &dyn ToolOutputPayload) -> JsonValue {
        response_input_to_code_mode_result(self.to_response_item("", payload))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct JsonToolOutput {
    value: JsonValue,
    success: Option<bool>,
}

impl JsonToolOutput {
    pub fn new(value: JsonValue) -> Self {
        Self {
            value,
            success: Some(true),
        }
    }

    pub fn with_success(value: JsonValue, success: Option<bool>) -> Self {
        Self { value, success }
    }
}

impl ToolOutput for JsonToolOutput {
    fn log_preview(&self) -> String {
        telemetry_preview(&self.value.to_string())
    }

    fn success_for_logging(&self) -> bool {
        self.success.unwrap_or(true)
    }

    fn to_response_item(
        &self,
        call_id: &str,
        payload: &dyn ToolOutputPayload,
    ) -> ResponseInputItem {
        let output = FunctionCallOutputPayload {
            body: FunctionCallOutputBody::Text(self.value.to_string()),
            success: self.success,
        };

        if payload.is_custom() {
            return ResponseInputItem::CustomToolCallOutput {
                call_id: call_id.to_string(),
                name: None,
                output,
            };
        }

        ResponseInputItem::FunctionCallOutput {
            call_id: call_id.to_string(),
            output,
        }
    }

    fn post_tool_use_response(
        &self,
        _call_id: &str,
        _payload: &dyn ToolOutputPayload,
    ) -> Option<JsonValue> {
        Some(self.value.clone())
    }

    fn code_mode_result(&self, _payload: &dyn ToolOutputPayload) -> JsonValue {
        self.value.clone()
    }
}

impl ToolOutput for codex_protocol::mcp::CallToolResult {
    fn log_preview(&self) -> String {
        telemetry_preview(
            &self
                .content
                .iter()
                .map(JsonValue::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    fn success_for_logging(&self) -> bool {
        !self.is_error.unwrap_or(false)
    }

    fn to_response_item(
        &self,
        call_id: &str,
        _payload: &dyn ToolOutputPayload,
    ) -> ResponseInputItem {
        ResponseInputItem::McpToolCallOutput {
            call_id: call_id.to_string(),
            output: self.clone(),
        }
    }

    fn code_mode_result(&self, _payload: &dyn ToolOutputPayload) -> JsonValue {
        serde_json::to_value(self).unwrap_or_else(|err| {
            JsonValue::String(format!("failed to serialize mcp result: {err}"))
        })
    }
}

impl<T> ToolOutput for Box<T>
where
    T: ToolOutput + ?Sized,
{
    fn log_preview(&self) -> String {
        (**self).log_preview()
    }

    fn success_for_logging(&self) -> bool {
        (**self).success_for_logging()
    }

    fn to_response_item(
        &self,
        call_id: &str,
        payload: &dyn ToolOutputPayload,
    ) -> ResponseInputItem {
        (**self).to_response_item(call_id, payload)
    }

    fn post_tool_use_id(&self, call_id: &str) -> String {
        (**self).post_tool_use_id(call_id)
    }

    fn post_tool_use_input(&self, payload: &dyn ToolOutputPayload) -> Option<JsonValue> {
        (**self).post_tool_use_input(payload)
    }

    fn post_tool_use_response(
        &self,
        call_id: &str,
        payload: &dyn ToolOutputPayload,
    ) -> Option<JsonValue> {
        (**self).post_tool_use_response(call_id, payload)
    }

    fn code_mode_result(&self, payload: &dyn ToolOutputPayload) -> JsonValue {
        (**self).code_mode_result(payload)
    }
}

/// Shared runtime contract for model-visible tools.
///
/// Implementations keep the model-visible spec tied to the executable runtime.
/// Host crates can layer routing, hooks, telemetry, or other orchestration on
/// top without reopening the spec/runtime split.
pub trait ToolExecutor<Invocation>: Send + Sync {
    type Output: ToolOutput + 'static;

    /// The concrete tool name handled by this runtime instance.
    fn tool_name(&self) -> ToolName;

    fn spec(&self) -> Option<ToolSpec> {
        None
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::Direct
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        false
    }

    fn handle(
        &self,
        invocation: Invocation,
    ) -> impl Future<Output = Result<Self::Output, FunctionCallError>> + Send;
}

fn response_input_to_code_mode_result(response: ResponseInputItem) -> JsonValue {
    match response {
        ResponseInputItem::Message { content, .. } => content_items_to_code_mode_result(
            &content
                .into_iter()
                .map(|item| match item {
                    codex_protocol::models::ContentItem::InputText { text }
                    | codex_protocol::models::ContentItem::OutputText { text } => {
                        FunctionCallOutputContentItem::InputText { text }
                    }
                    codex_protocol::models::ContentItem::InputImage { image_url, detail } => {
                        FunctionCallOutputContentItem::InputImage {
                            image_url,
                            detail: detail.or(Some(DEFAULT_IMAGE_DETAIL)),
                        }
                    }
                })
                .collect::<Vec<_>>(),
        ),
        ResponseInputItem::FunctionCallOutput { output, .. }
        | ResponseInputItem::CustomToolCallOutput { output, .. } => match output.body {
            FunctionCallOutputBody::Text(text) => JsonValue::String(text),
            FunctionCallOutputBody::ContentItems(items) => {
                content_items_to_code_mode_result(&items)
            }
        },
        ResponseInputItem::ToolSearchOutput { tools, .. } => JsonValue::Array(tools),
        ResponseInputItem::McpToolCallOutput { output, .. } => serde_json::to_value(output)
            .unwrap_or_else(|err| {
                JsonValue::String(format!("failed to serialize mcp result: {err}"))
            }),
    }
}

fn content_items_to_code_mode_result(items: &[FunctionCallOutputContentItem]) -> JsonValue {
    JsonValue::String(
        items
            .iter()
            .filter_map(|item| match item {
                FunctionCallOutputContentItem::InputText { text } if !text.trim().is_empty() => {
                    Some(text.clone())
                }
                FunctionCallOutputContentItem::InputImage { image_url, .. }
                    if !image_url.trim().is_empty() =>
                {
                    Some(image_url.clone())
                }
                FunctionCallOutputContentItem::InputText { .. }
                | FunctionCallOutputContentItem::InputImage { .. }
                | FunctionCallOutputContentItem::EncryptedContent { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// Lifecycle state for a tool execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolExecutionStatus {
    /// Execution has been accepted but has not started.
    Queued,
    /// Execution is currently running.
    Running,
    /// Execution completed successfully.
    Completed,
    /// Execution ended in an error.
    Failed,
}

/// Protocol-neutral event emitted during tool execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolExecutionEvent {
    /// Stable tool call identifier.
    pub call_id: String,
    /// Current execution status.
    pub status: ToolExecutionStatus,
    /// Optional status detail supplied by the executor.
    pub message: Option<String>,
}

/// Observes tool execution lifecycle events.
///
/// Implementations should record, forward, or transform execution events
/// without owning the concrete tool execution backend.
pub trait ToolExecutionObserver {
    /// Records one tool execution event.
    fn record_tool_execution_event(&mut self, event: ToolExecutionEvent);
}
