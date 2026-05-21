//! Tool handler abstractions.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// Protocol-neutral tool call input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolCall {
    /// Stable call identifier.
    pub call_id: String,
    /// Tool name selected by the caller.
    pub name: String,
    /// Serialized arguments owned by the concrete tool protocol.
    pub arguments: String,
}

/// Protocol-neutral tool call output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolOutput {
    /// Stable call identifier this output answers.
    pub call_id: String,
    /// Serialized output owned by the concrete tool protocol.
    pub output: String,
}

/// Handles one protocol-neutral tool call.
///
/// Implementations should execute or route a call for the named tool and
/// return output without requiring callers to know the concrete tool backend.
pub trait ToolHandler {
    /// Error type returned by the concrete handler.
    type Error;

    /// Handles a tool call and returns its output.
    fn handle_tool_call(&mut self, call: ToolCall) -> Result<ToolOutput, Self::Error>;
}
