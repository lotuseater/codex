//! Tool invocation boundary for turn execution.
//!
//! This crate owns the small adapter surface between a turn loop and whichever
//! tool runtime supplies concrete tool implementations.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

use codex_turn_api::TurnId;
use codex_turn_events::TurnEvent;
use codex_turn_events::TurnEventKind;

/// Request to invoke a tool for a turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolRequest {
    arguments: String,
    name: String,
    turn_id: TurnId,
}

impl ToolRequest {
    /// Builds a new tool invocation request.
    #[must_use]
    pub fn new(turn_id: TurnId, name: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            arguments: arguments.into(),
            name: name.into(),
            turn_id,
        }
    }

    /// Returns the id of the turn that requested this tool.
    #[must_use]
    pub const fn turn_id(&self) -> TurnId {
        self.turn_id
    }

    /// Returns the requested tool name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the serialized tool arguments.
    #[must_use]
    pub fn arguments(&self) -> &str {
        &self.arguments
    }

    /// Converts this request into an event.
    #[must_use]
    pub fn into_event(self) -> TurnEvent {
        TurnEvent::new(
            self.turn_id,
            TurnEventKind::ToolRequested {
                tool_name: self.name,
            },
        )
    }
}

/// Result returned by a tool runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolResult {
    content: String,
    turn_id: TurnId,
}

impl ToolResult {
    /// Builds a new tool result.
    #[must_use]
    pub fn new(turn_id: TurnId, content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            turn_id,
        }
    }

    /// Returns the id of the turn that received this result.
    #[must_use]
    pub const fn turn_id(&self) -> TurnId {
        self.turn_id
    }

    /// Returns the tool output content.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Converts this result into a message event.
    #[must_use]
    pub fn into_event(self) -> TurnEvent {
        TurnEvent::new(
            self.turn_id,
            TurnEventKind::Message {
                content: self.content,
            },
        )
    }
}

/// Error returned by a tool runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolBridgeError {
    Unavailable { tool_name: String },
    Rejected { reason: String },
}

impl fmt::Display for ToolBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { tool_name } => write!(f, "tool `{tool_name}` is unavailable"),
            Self::Rejected { reason } => write!(f, "tool request rejected: {reason}"),
        }
    }
}

impl Error for ToolBridgeError {}

/// Result type for tool bridge implementations.
pub type ToolBridgeResult = Result<ToolResult, ToolBridgeError>;

/// Invokes tools requested by a turn loop.
///
/// Implementations should translate the neutral request into the concrete tool
/// runtime and return output that can be recorded as turn events.
pub trait TurnToolBridge {
    fn invoke(&mut self, request: ToolRequest) -> ToolBridgeResult;
}
