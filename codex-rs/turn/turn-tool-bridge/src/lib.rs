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
    pub const fn turn_id(&self) -> &TurnId {
        &self.turn_id
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
    pub const fn turn_id(&self) -> &TurnId {
        &self.turn_id
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

#[cfg(test)]
mod tests {
    use super::*;
    use codex_turn_events::TurnEventKind;

    fn turn_id(value: u64) -> TurnId {
        TurnId::new(format!("turn-{value}"))
    }

    #[test]
    fn tool_request_preserves_payload_and_converts_to_event() {
        let turn_id = turn_id(41);
        let request = ToolRequest::new(turn_id.clone(), "shell", "{\"cmd\":\"pwd\"}");

        assert_eq!(&turn_id, request.turn_id());
        assert_eq!("shell", request.name());
        assert_eq!("{\"cmd\":\"pwd\"}", request.arguments());

        let event = request.into_event();

        assert_eq!(&turn_id, event.turn_id());
        assert_eq!(
            &TurnEventKind::ToolRequested {
                tool_name: "shell".to_string(),
            },
            event.kind()
        );
    }

    #[test]
    fn tool_result_preserves_content_and_converts_to_message_event() {
        let turn_id = turn_id(43);
        let result = ToolResult::new(turn_id.clone(), "stdout");

        assert_eq!(&turn_id, result.turn_id());
        assert_eq!("stdout", result.content());

        let event = result.into_event();

        assert_eq!(&turn_id, event.turn_id());
        assert_eq!(
            &TurnEventKind::Message {
                content: "stdout".to_string(),
            },
            event.kind()
        );
    }

    #[test]
    fn tool_bridge_errors_have_stable_messages() {
        assert_eq!(
            "tool `shell` is unavailable",
            ToolBridgeError::Unavailable {
                tool_name: "shell".to_string(),
            }
            .to_string()
        );
        assert_eq!(
            "tool request rejected: policy blocked",
            ToolBridgeError::Rejected {
                reason: "policy blocked".to_string(),
            }
            .to_string()
        );
    }
}
