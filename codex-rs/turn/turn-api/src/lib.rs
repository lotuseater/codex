//! Shared protocol-neutral value types for turn handling.
//!
//! This crate owns stable identifiers and request/response shapes only. It
//! should not know how a turn is scheduled, stored, executed, or rendered.

#![forbid(unsafe_code)]

use std::fmt;
use std::num::NonZeroU64;

/// Stable identifier for a single model turn.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TurnId(NonZeroU64);

impl TurnId {
    /// Creates a turn id from a known non-zero value.
    #[must_use]
    pub const fn from_non_zero(value: NonZeroU64) -> Self {
        Self(value)
    }

    /// Returns the underlying numeric id.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for TurnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
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
    pub const fn turn_id(&self) -> TurnId {
        self.turn_id
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
    pub const fn turn_id(&self) -> TurnId {
        self.turn_id
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
