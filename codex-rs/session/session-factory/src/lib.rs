//! Session factory boundary.
//!
//! This crate describes how callers create or open session runtimes without
//! depending on a concrete runtime implementation or storage backend.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use std::future::Future;
use std::pin::Pin;

use codex_session_api::SessionIdentity;
use codex_session_runtime_api::SessionRuntime;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Boxed future returned by session factory abstractions.
pub type SessionFactoryFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Result returned by session factory abstractions.
pub type SessionFactoryResult<T> = Result<T, SessionFactoryError>;

/// Request to create a new runtime session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub identity: SessionIdentity,
}

impl CreateSessionRequest {
    /// Creates a request to create a session runtime.
    pub fn new(identity: SessionIdentity) -> Self {
        Self { identity }
    }
}

/// Request to open an existing runtime session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OpenSessionRequest {
    pub identity: SessionIdentity,
}

impl OpenSessionRequest {
    /// Creates a request to open a session runtime.
    pub fn new(identity: SessionIdentity) -> Self {
        Self { identity }
    }
}

/// Failure reported while creating or opening session runtimes.
#[derive(Debug, Error)]
pub enum SessionFactoryError {
    #[error("session factory is unavailable: {0}")]
    Unavailable(String),

    #[error("session factory failed: {0}")]
    Failed(String),
}

/// Boundary for creating or opening session runtimes.
///
/// Implementations should construct concrete runtime handles and keep storage,
/// policy, and transport details behind this boundary.
pub trait SessionFactory: Send + Sync {
    fn create_session<'a>(
        &'a self,
        request: CreateSessionRequest,
    ) -> SessionFactoryFuture<'a, SessionFactoryResult<Box<dyn SessionRuntime>>>;

    fn open_session<'a>(
        &'a self,
        request: OpenSessionRequest,
    ) -> SessionFactoryFuture<'a, SessionFactoryResult<Box<dyn SessionRuntime>>>;
}
