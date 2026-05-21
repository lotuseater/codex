//! Serializable session state DTOs.
//!
//! This crate owns persisted or transferred state shapes without depending on
//! any concrete runtime, store, transport, or UI implementation.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use std::collections::BTreeMap;

use codex_session_api::SessionIdentity;
use codex_session_api::SessionLifecycleState;
use serde::Deserialize;
use serde::Serialize;

/// Compact snapshot of session state for storage or handoff.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionStateSnapshot {
    pub identity: SessionIdentity,
    pub lifecycle: SessionLifecycleState,
    pub metadata: BTreeMap<String, String>,
    pub pending_input_count: usize,
}

impl SessionStateSnapshot {
    /// Creates a snapshot with empty metadata and no pending inputs.
    pub fn new(identity: SessionIdentity, lifecycle: SessionLifecycleState) -> Self {
        Self {
            identity,
            lifecycle,
            metadata: BTreeMap::new(),
            pending_input_count: 0,
        }
    }
}
