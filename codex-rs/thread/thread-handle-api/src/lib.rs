//! Abstract active-thread handle boundary.
//!
//! Concrete live-thread persistence implementations live outside this crate.
//! Session and turn crates should depend on this port instead of store-specific
//! handle types.

use std::future::Future;
use std::pin::Pin;

use codex_thread_api::ThreadIdentity;

/// Boxed future used by object-safe thread handle ports.
pub type ThreadHandleFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, thiserror::Error)]
pub enum ThreadHandleError {
    #[error("thread handle operation is unsupported: {operation}")]
    Unsupported { operation: &'static str },
}

pub type ThreadHandleResult<T> = Result<T, ThreadHandleError>;

/// Object-safe handle for an active thread.
pub trait ThreadHandle: Send + Sync {
    fn identity(&self) -> ThreadIdentity;

    fn persist(&self) -> ThreadHandleFuture<'_, ThreadHandleResult<()>>;

    fn flush(&self) -> ThreadHandleFuture<'_, ThreadHandleResult<()>>;

    fn shutdown(&self) -> ThreadHandleFuture<'_, ThreadHandleResult<()>>;
}

/// Factory for active-thread handles.
pub trait ThreadHandleFactory: Send + Sync {
    fn open<'a>(
        &'a self,
        identity: ThreadIdentity,
    ) -> ThreadHandleFuture<'a, ThreadHandleResult<Box<dyn ThreadHandle>>>;
}
