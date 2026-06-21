use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::ThreadMemoryMode;

use crate::CreateThreadParams;
use crate::ResumeThreadParams;
use crate::StoredThread;
use crate::StoredThreadHistory;
use crate::ThreadMetadataPatch;
use crate::ThreadStore;
use crate::ThreadStoreError;
use crate::ThreadStoreResult;

/// Boxed future returned by object-safe thread-store API traits.
pub(crate) type ThreadStoreFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Persistence services required by thread manager implementations.
///
/// Implementations should pass concrete stores and live-thread factories through
/// this bundle so orchestration code does not need to know which storage
/// implementation backs active threads.
#[derive(Clone)]
pub struct ThreadPersistenceServices {
    /// Storage backend used for persisted thread records.
    pub thread_store: Arc<dyn ThreadStore>,

    /// Factory used to create and resume per-thread live persistence handles.
    pub live_thread_factory: Arc<dyn LiveThreadFactory>,
}

impl ThreadPersistenceServices {
    /// Create a persistence services bundle from storage-neutral trait objects.
    pub fn new(
        thread_store: Arc<dyn ThreadStore>,
        live_thread_factory: Arc<dyn LiveThreadFactory>,
    ) -> Self {
        Self {
            thread_store,
            live_thread_factory,
        }
    }
}

/// Live per-thread persistence handle used by sessions while a thread is active.
///
/// Implementations own the concrete persistence behavior. Consumers should use
/// this trait instead of depending on a concrete live-thread type.
pub trait LiveThreadHandle: Send + Sync {
    /// Append rollout items produced by the active session.
    fn append_items<'a>(
        &'a self,
        items: &'a [RolloutItem],
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<()>>;

    /// Persist any buffered rollout data.
    fn persist(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>>;

    /// Flush any pending rollout writes.
    fn flush(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>>;

    /// Shutdown the live handle after a completed session.
    fn shutdown(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>>;

    /// Discard an uncommitted live handle.
    fn discard(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>>;

    /// Load persisted history for this live thread.
    fn load_history(
        &self,
        include_archived: bool,
    ) -> ThreadStoreFuture<'_, ThreadStoreResult<StoredThreadHistory>>;

    /// Read the persisted thread snapshot for this live thread.
    fn read_thread(
        &self,
        include_archived: bool,
        include_history: bool,
    ) -> ThreadStoreFuture<'_, ThreadStoreResult<StoredThread>>;

    /// Apply a metadata patch to this live thread.
    fn update_metadata(
        &self,
        patch: ThreadMetadataPatch,
        include_archived: bool,
    ) -> ThreadStoreFuture<'_, ThreadStoreResult<StoredThread>>;

    /// Update the persisted thread-level memory behavior.
    fn update_memory_mode<'a>(
        &'a self,
        mode: ThreadMemoryMode,
        include_archived: bool,
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<()>> {
        Box::pin(async move {
            self.update_metadata(
                ThreadMetadataPatch {
                    memory_mode: Some(mode),
                    ..Default::default()
                },
                include_archived,
            )
            .await?;
            Ok(())
        })
    }

    /// Persist the active thread and update its thread-level memory behavior.
    fn persist_memory_mode<'a>(
        &'a self,
        mode: ThreadMemoryMode,
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<()>> {
        Box::pin(async move {
            self.persist().await?;
            self.flush().await?;
            self.update_memory_mode(mode, /*include_archived*/ false)
                .await?;
            self.flush().await
        })
    }

    /// Return the local rollout path when the implementation has one.
    fn local_rollout_path(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<Option<PathBuf>>>;
}

/// Factory for live per-thread persistence handles.
///
/// Core/session orchestration depends on this abstraction; concrete storage
/// crates decide how a handle is created or resumed for a specific store.
pub trait LiveThreadFactory: Send + Sync {
    /// Create persistence for a new thread.
    fn create<'a>(
        &'a self,
        thread_store: Arc<dyn ThreadStore>,
        params: CreateThreadParams,
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<Arc<dyn LiveThreadHandle>>>;

    /// Resume persistence for an existing thread.
    fn resume<'a>(
        &'a self,
        thread_store: Arc<dyn ThreadStore>,
        params: ResumeThreadParams,
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<Arc<dyn LiveThreadHandle>>>;
}

/// Live-thread factory for contexts where persistence is intentionally disabled.
#[derive(Default)]
pub struct UnsupportedLiveThreadFactory;

impl UnsupportedLiveThreadFactory {
    /// Create a disabled live-thread factory.
    pub fn new() -> Self {
        Self
    }
}

impl LiveThreadFactory for UnsupportedLiveThreadFactory {
    fn create<'a>(
        &'a self,
        _thread_store: Arc<dyn ThreadStore>,
        _params: CreateThreadParams,
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<Arc<dyn LiveThreadHandle>>> {
        Box::pin(async {
            Err(ThreadStoreError::Unsupported {
                operation: "create live thread",
            })
        })
    }

    fn resume<'a>(
        &'a self,
        _thread_store: Arc<dyn ThreadStore>,
        _params: ResumeThreadParams,
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<Arc<dyn LiveThreadHandle>>> {
        Box::pin(async {
            Err(ThreadStoreError::Unsupported {
                operation: "resume live thread",
            })
        })
    }
}
