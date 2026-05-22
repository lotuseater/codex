use std::future::Future;
use std::pin::Pin;

pub use codex_tool_execution_api::ToolCallSource;

use codex_tool_execution_api::ToolName;

use crate::ExtensionData;

/// Future returned by one tool-lifecycle callback.
pub type ToolLifecycleFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

/// Extension-facing outcome for a finished tool call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolCallOutcome {
    /// The tool returned a normal output.
    Completed {
        /// The tool output's own success marker for telemetry/logging.
        success: bool,
    },
    /// The tool was blocked by host policy before the handler ran.
    Blocked,
    /// The tool did not produce a normal output.
    Failed {
        /// Whether the host reached the tool handler before the failure.
        handler_executed: bool,
    },
    /// The host cancelled the tool before normal completion. Cancellation can
    /// win before the dispatch path accepts the call, so contributors should not
    /// assume a matching start callback exists.
    Aborted,
}

/// Input supplied when the host starts executing one tool call.
pub struct ToolStartInput<'a> {
    /// Store scoped to the host session runtime.
    pub session_store: &'a ExtensionData,
    /// Store scoped to this thread runtime.
    pub thread_store: &'a ExtensionData,
    /// Store scoped to this turn runtime.
    pub turn_store: &'a ExtensionData,
    /// Current turn submission id.
    pub turn_id: &'a str,
    /// Model-visible tool call id.
    pub call_id: &'a str,
    /// Tool name as routed by the host.
    pub tool_name: &'a ToolName,
    /// Source that issued the tool call.
    pub source: ToolCallSource,
}

/// Input supplied when the host finishes executing one tool call.
pub struct ToolFinishInput<'a> {
    /// Store scoped to the host session runtime.
    pub session_store: &'a ExtensionData,
    /// Store scoped to this thread runtime.
    pub thread_store: &'a ExtensionData,
    /// Store scoped to this turn runtime.
    pub turn_store: &'a ExtensionData,
    /// Current turn submission id.
    pub turn_id: &'a str,
    /// Model-visible tool call id.
    pub call_id: &'a str,
    /// Tool name as routed by the host.
    pub tool_name: &'a ToolName,
    /// Source that issued the tool call.
    pub source: ToolCallSource,
    /// Host-observed result of the tool call.
    pub outcome: ToolCallOutcome,
}
