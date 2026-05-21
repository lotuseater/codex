//! Session event DTOs and event stream boundaries.
//!
//! This crate describes observable session events without coupling to a
//! transport, renderer, or runtime implementation.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use std::future::Future;
use std::pin::Pin;

use codex_session_api::SessionLifecycleState;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Boxed future returned by session event abstractions.
pub type SessionEventFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Result returned by session event abstractions.
pub type SessionEventResult<T> = Result<T, SessionEventError>;

/// Event emitted by session orchestration or runtime code.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionEvent {
    LifecycleChanged { lifecycle: SessionLifecycleState },
    InputAccepted,
    OutputText { text: String },
    Failed { message: String },
}

/// Failure reported while publishing or reading session events.
#[derive(Debug, Error)]
pub enum SessionEventError {
    #[error("session event stream is closed")]
    Closed,

    #[error("session event operation failed: {0}")]
    Failed(String),
}

/// Boundary for publishing session events.
///
/// Implementations decide durability and fan-out behavior, but should preserve
/// event ordering for a single session.
pub trait SessionEventSink: Send + Sync {
    fn publish<'a>(&'a self, event: SessionEvent)
    -> SessionEventFuture<'a, SessionEventResult<()>>;
}

/// Boundary for reading session events.
///
/// Implementations should return `Ok(None)` when the stream is cleanly
/// exhausted and an error when the stream fails unexpectedly.
pub trait SessionEventSource: Send + Sync {
    fn next_event<'a>(&'a self)
    -> SessionEventFuture<'a, SessionEventResult<Option<SessionEvent>>>;
}
