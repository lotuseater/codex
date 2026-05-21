//! Abstract session runtime boundary.
//!
//! This crate owns runtime command/result DTOs and object-safe traits without
//! depending on concrete session execution, transport, or UI crates.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use std::future::Future;
use std::pin::Pin;

use codex_session_api::SessionIdentity;
use codex_session_api::SessionLifecycleState;
use codex_session_events::SessionEvent;
use codex_session_input::SessionInput;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Boxed future returned by session runtime abstractions.
pub type SessionRuntimeFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Result returned by session runtime abstractions.
pub type SessionRuntimeResult<T> = Result<T, SessionRuntimeError>;

/// Command accepted by a session runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionRuntimeCommand {
    SubmitInput { input: SessionInput },
    Shutdown,
}

/// Minimal status reported by a session runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionRuntimeStatus {
    pub identity: SessionIdentity,
    pub lifecycle: SessionLifecycleState,
}

impl SessionRuntimeStatus {
    /// Creates a runtime status value.
    pub fn new(identity: SessionIdentity, lifecycle: SessionLifecycleState) -> Self {
        Self {
            identity,
            lifecycle,
        }
    }
}

/// Failure reported by a session runtime.
#[derive(Debug, Error)]
pub enum SessionRuntimeError {
    #[error("session runtime is unavailable: {0}")]
    Unavailable(String),

    #[error("session runtime rejected command: {0}")]
    Rejected(String),

    #[error("session runtime failed: {0}")]
    Failed(String),
}

/// Boundary for driving an active session runtime.
///
/// Implementations own the execution model for one active session. They should
/// emit observable events through `next_event` and report command failures
/// through `handle_command`.
pub trait SessionRuntime: Send + Sync {
    fn identity(&self) -> &SessionIdentity;

    fn status<'a>(&'a self)
    -> SessionRuntimeFuture<'a, SessionRuntimeResult<SessionRuntimeStatus>>;

    fn handle_command<'a>(
        &'a self,
        command: SessionRuntimeCommand,
    ) -> SessionRuntimeFuture<'a, SessionRuntimeResult<()>>;

    fn next_event<'a>(
        &'a self,
    ) -> SessionRuntimeFuture<'a, SessionRuntimeResult<Option<SessionEvent>>>;
}
