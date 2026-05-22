//! Shared protocol-neutral value types for turn handling.
//!
//! This crate owns stable identifiers and request/response shapes only. It
//! should not know how a turn is scheduled, stored, executed, or rendered.

#![forbid(unsafe_code)]

use std::fmt;

/// Stable identifier for a single model turn.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TurnId(String);

impl TurnId {
    /// Creates a turn id from the runtime id assigned to the turn.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the runtime id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TurnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// User-visible input that starts a turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnInput {
    turn_id: TurnId,
    prompt: String,
}

impl TurnInput {
    /// Builds a new turn input.
    #[must_use]
    pub fn new(turn_id: TurnId, prompt: impl Into<String>) -> Self {
        Self {
            turn_id,
            prompt: prompt.into(),
        }
    }

    /// Returns the id associated with this input.
    #[must_use]
    pub const fn turn_id(&self) -> &TurnId {
        &self.turn_id
    }

    /// Returns the prompt text for the turn.
    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}

/// High-level terminal status for a turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TurnStatus {
    Succeeded,
    Deferred,
    Rejected,
    Failed,
}

/// User-visible output produced by a turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnOutput {
    turn_id: TurnId,
    status: TurnStatus,
    message: String,
}

impl TurnOutput {
    /// Builds a new turn output.
    #[must_use]
    pub fn new(turn_id: TurnId, status: TurnStatus, message: impl Into<String>) -> Self {
        Self {
            turn_id,
            status,
            message: message.into(),
        }
    }

    /// Returns the id associated with this output.
    #[must_use]
    pub const fn turn_id(&self) -> &TurnId {
        &self.turn_id
    }

    /// Returns the terminal status for the turn.
    #[must_use]
    pub const fn status(&self) -> TurnStatus {
        self.status
    }

    /// Returns the output message for the turn.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn_id(value: u64) -> TurnId {
        TurnId::new(format!("turn-{value}"))
    }

    #[test]
    fn turn_input_preserves_turn_id_and_prompt() {
        let turn_id = turn_id(7);
        let input = TurnInput::new(turn_id.clone(), "summarize this");

        assert_eq!(&turn_id, input.turn_id());
        assert_eq!("summarize this", input.prompt());
    }

    #[test]
    fn turn_output_preserves_status_and_message() {
        let turn_id = turn_id(9);
        let output = TurnOutput::new(turn_id.clone(), TurnStatus::Deferred, "busy");

        assert_eq!(&turn_id, output.turn_id());
        assert_eq!(TurnStatus::Deferred, output.status());
        assert_eq!("busy", output.message());
    }
}
