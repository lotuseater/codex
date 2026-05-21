//! Session policy request and decision boundary.
//!
//! Runtime and adapter crates can use this crate to ask policy code whether a
//! session operation should proceed without depending on concrete policy engines.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use std::future::Future;
use std::pin::Pin;

use codex_session_api::SessionIdentity;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Boxed future returned by session policy abstractions.
pub type SessionPolicyFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Result returned by session policy abstractions.
pub type SessionPolicyResult<T> = Result<T, SessionPolicyError>;

/// Session operation being evaluated by policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPolicyAction {
    Create,
    Open,
    SubmitInput,
    Shutdown,
}

/// Policy evaluation request for a session operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionPolicyRequest {
    pub identity: SessionIdentity,
    pub action: SessionPolicyAction,
}

impl SessionPolicyRequest {
    /// Creates a policy request for the identity and operation.
    pub fn new(identity: SessionIdentity, action: SessionPolicyAction) -> Self {
        Self { identity, action }
    }
}

/// Decision returned by session policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionPolicyDecision {
    pub allowed: bool,
    pub reason: Option<String>,
}

impl SessionPolicyDecision {
    /// Creates an allowed policy decision.
    pub fn allow() -> Self {
        Self {
            allowed: true,
            reason: None,
        }
    }

    /// Creates a denied policy decision with a reason.
    pub fn deny(reason: String) -> Self {
        Self {
            allowed: false,
            reason: Some(reason),
        }
    }
}

/// Failure reported while evaluating session policy.
#[derive(Debug, Error)]
pub enum SessionPolicyError {
    #[error("session policy evaluation failed: {0}")]
    Failed(String),
}

/// Boundary for evaluating session policy.
///
/// Implementations should be deterministic for a single request and should
/// avoid mutating session runtime state directly.
pub trait SessionPolicy: Send + Sync {
    fn evaluate<'a>(
        &'a self,
        request: SessionPolicyRequest,
    ) -> SessionPolicyFuture<'a, SessionPolicyResult<SessionPolicyDecision>>;
}
