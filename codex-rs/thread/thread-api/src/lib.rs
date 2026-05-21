//! Core thread identity types.
//!
//! This crate is intentionally narrow: it owns thread identity DTOs that can be
//! shared by session, turn, store, and adapter crates without importing concrete
//! stores or `codex-core`.

use codex_protocol::ThreadId;
use serde::Deserialize;
use serde::Serialize;

/// Stable identity for a Codex thread.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ThreadIdentity {
    pub id: ThreadId,
}

impl ThreadIdentity {
    pub fn new(id: ThreadId) -> Self {
        Self { id }
    }
}
