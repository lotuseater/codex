//! Session input DTOs and input queue boundary.
//!
//! Concrete queues, transports, and runtime adapters live outside this crate.
//! Implementations should preserve input ordering for each session.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Boxed future returned by session input abstractions.
pub type SessionInputFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Result returned by session input abstractions.
pub type SessionInputResult<T> = Result<T, SessionInputError>;

/// User or system input submitted to a session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionInput {
    UserText { text: String },
    SystemText { text: String },
}

/// Failure reported by a session input queue.
#[derive(Debug, Error)]
pub enum SessionInputError {
    #[error("session input queue is closed")]
    Closed,

    #[error("session input was rejected: {0}")]
    Rejected(String),
}

/// Boundary for submitting ordered input into a session.
///
/// Implementations are expected to enqueue accepted input durably enough for
/// their runtime model and return only after rejection can no longer be reported
/// synchronously.
pub trait SessionInputQueue: Send + Sync {
    fn push<'a>(&'a self, input: SessionInput) -> SessionInputFuture<'a, SessionInputResult<()>>;
}
