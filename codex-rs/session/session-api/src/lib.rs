//! Core session identity and lifecycle types.
//!
//! This crate is intentionally narrow: it owns stable session DTOs that can be
//! shared by runtime, policy, factory, and adapter crates without importing
//! concrete stores, transports, runtimes, or UI crates.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use codex_protocol::SessionId;
use serde::Deserialize;
use serde::Serialize;

/// Settings from the previously completed turn that influence the next turn.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreviousTurnSettings {
    pub model: String,
    pub realtime_active: Option<bool>,
    pub comp_hash: Option<String>,
}

impl PreviousTurnSettings {
    pub fn new(model: String, realtime_active: Option<bool>) -> Self {
        Self {
            model,
            realtime_active,
            comp_hash: None,
        }
    }
}

/// Stable identity for a Codex session.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SessionIdentity {
    pub id: SessionId,
}

impl SessionIdentity {
    /// Creates a session identity from a protocol session id.
    pub fn new(id: SessionId) -> Self {
        Self { id }
    }
}

/// Coarse lifecycle state for a session.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionLifecycleState {
    Created,
    Active,
    Draining,
    Completed,
    Failed,
}

impl SessionLifecycleState {
    /// Returns true when the session can accept new input.
    pub fn accepts_input(self) -> bool {
        matches!(self, Self::Created | Self::Active)
    }

    /// Returns true when the session has reached a terminal state.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

/// Identity plus lifecycle information for callers that only need a summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionDescriptor {
    pub identity: SessionIdentity,
    pub lifecycle: SessionLifecycleState,
}

impl SessionDescriptor {
    /// Creates a session descriptor from the identity and lifecycle state.
    pub fn new(identity: SessionIdentity, lifecycle: SessionLifecycleState) -> Self {
        Self {
            identity,
            lifecycle,
        }
    }
}
