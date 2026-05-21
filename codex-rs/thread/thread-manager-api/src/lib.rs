//! Thread manager command boundary.
//!
//! This crate owns the command/result DTOs for thread orchestration without
//! depending on concrete sessions, stores, app-server protocol, or `codex-core`.

use std::future::Future;
use std::pin::Pin;

use codex_thread_api::ThreadIdentity;

/// Boxed future used by object-safe thread manager ports.
pub type ThreadManagerFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartThreadCommand {
    pub identity: ThreadIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeThreadCommand {
    pub identity: ThreadIdentity,
}

#[derive(Debug, thiserror::Error)]
pub enum ThreadManagerError {
    #[error("thread manager operation is unsupported: {operation}")]
    Unsupported { operation: &'static str },
}

pub type ThreadManagerResult<T> = Result<T, ThreadManagerError>;

/// Port for starting and resuming threads.
pub trait ThreadManagerPort: Send + Sync {
    fn start_thread<'a>(
        &'a self,
        command: StartThreadCommand,
    ) -> ThreadManagerFuture<'a, ThreadManagerResult<ThreadIdentity>>;

    fn resume_thread<'a>(
        &'a self,
        command: ResumeThreadCommand,
    ) -> ThreadManagerFuture<'a, ThreadManagerResult<ThreadIdentity>>;
}
