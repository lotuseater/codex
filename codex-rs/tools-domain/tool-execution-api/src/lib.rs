//! Tool execution observation abstractions.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

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
