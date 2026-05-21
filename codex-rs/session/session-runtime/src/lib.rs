//! Minimal session runtime adapters.
//!
//! Concrete Codex session execution belongs in dedicated runtime crates. This
//! crate provides a small placeholder implementation for wiring tests and
//! adapter boundaries that are not yet connected to a runtime.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use codex_session_api::SessionIdentity;
use codex_session_events::SessionEvent;
use codex_session_runtime_api::SessionRuntime;
use codex_session_runtime_api::SessionRuntimeCommand;
use codex_session_runtime_api::SessionRuntimeError;
use codex_session_runtime_api::SessionRuntimeFuture;
use codex_session_runtime_api::SessionRuntimeResult;
use codex_session_runtime_api::SessionRuntimeStatus;

/// Error message returned by the unsupported runtime adapter.
pub const UNSUPPORTED_SESSION_RUNTIME: &str = "session runtime is not wired";

/// Runtime implementation that reports every operation as unavailable.
#[derive(Clone, Debug)]
pub struct UnsupportedSessionRuntime {
    identity: SessionIdentity,
}

impl UnsupportedSessionRuntime {
    /// Creates an unsupported runtime for the given identity.
    pub fn new(identity: SessionIdentity) -> Self {
        Self { identity }
    }
}

impl SessionRuntime for UnsupportedSessionRuntime {
    fn identity(&self) -> &SessionIdentity {
        &self.identity
    }

    fn status<'a>(
        &'a self,
    ) -> SessionRuntimeFuture<'a, SessionRuntimeResult<SessionRuntimeStatus>> {
        Box::pin(async move {
            Err(SessionRuntimeError::Unavailable(
                UNSUPPORTED_SESSION_RUNTIME.to_string(),
            ))
        })
    }

    fn handle_command<'a>(
        &'a self,
        _command: SessionRuntimeCommand,
    ) -> SessionRuntimeFuture<'a, SessionRuntimeResult<()>> {
        Box::pin(async move {
            Err(SessionRuntimeError::Unavailable(
                UNSUPPORTED_SESSION_RUNTIME.to_string(),
            ))
        })
    }

    fn next_event<'a>(
        &'a self,
    ) -> SessionRuntimeFuture<'a, SessionRuntimeResult<Option<SessionEvent>>> {
        Box::pin(async move {
            Err(SessionRuntimeError::Unavailable(
                UNSUPPORTED_SESSION_RUNTIME.to_string(),
            ))
        })
    }
}
