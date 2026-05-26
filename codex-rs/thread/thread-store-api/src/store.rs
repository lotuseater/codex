use async_trait::async_trait;
use codex_protocol::ThreadId;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use std::any::Any;

use crate::AppendThreadItemsParams;
use crate::ArchiveThreadParams;
use crate::CreateThreadParams;
use crate::ItemPage;
use crate::ListItemsParams;
use crate::ListThreadsParams;
use crate::ListTurnsParams;
use crate::LoadThreadHistoryParams;
use crate::ReadThreadByRolloutPathParams;
use crate::ReadThreadDynamicToolsParams;
use crate::ReadThreadParams;
use crate::ResumeThreadParams;
use crate::SearchThreadsParams;
use crate::StoredThread;
use crate::StoredThreadHistory;
use crate::ThreadPage;
use crate::ThreadSearchPage;
use crate::ThreadStoreError;
use crate::ThreadStoreResult;
use crate::TurnPage;
use crate::UpdateThreadMetadataParams;

/// Storage-neutral thread persistence boundary.
#[async_trait]
pub trait ThreadStore: Any + Send + Sync {
    /// Return this store as [`Any`] for implementation-owned escape hatches.
    fn as_any(&self) -> &dyn Any;

    /// Creates a new live thread.
    async fn create_thread(&self, params: CreateThreadParams) -> ThreadStoreResult<()>;

    /// Reopens an existing thread for live appends.
    async fn resume_thread(&self, params: ResumeThreadParams) -> ThreadStoreResult<()>;

    /// Appends canonical rollout items to a live thread.
    ///
    /// This is the raw history API. It does not infer metadata from item contents. Callers that
    /// need metadata updates should call [`ThreadStore::update_thread_metadata`] with explicit
    /// metadata facts prepared above the store.
    async fn append_items(&self, params: AppendThreadItemsParams) -> ThreadStoreResult<()>;

    /// Materializes the thread if persistence is lazy, then persists all queued items.
    async fn persist_thread(&self, thread_id: ThreadId) -> ThreadStoreResult<()>;

    /// Flushes all queued items and returns once they are durable/readable.
    async fn flush_thread(&self, thread_id: ThreadId) -> ThreadStoreResult<()>;

    /// Flushes pending items and closes the live thread writer.
    async fn shutdown_thread(&self, thread_id: ThreadId) -> ThreadStoreResult<()>;

    /// Discards the live thread writer without forcing pending in-memory items to become durable.
    ///
    /// Core calls this when session initialization fails after a live writer has been created.
    /// Implementations should release any live writer resources for the thread while preserving
    /// already-durable thread data.
    async fn discard_thread(&self, thread_id: ThreadId) -> ThreadStoreResult<()>;

    /// Loads persisted history for resume, fork, rollback, and memory jobs.
    async fn load_history(
        &self,
        params: LoadThreadHistoryParams,
    ) -> ThreadStoreResult<StoredThreadHistory>;

    /// Reads a thread summary and optionally its persisted history.
    async fn read_thread(&self, params: ReadThreadParams) -> ThreadStoreResult<StoredThread>;

    /// Reads persisted dynamic tools associated with a thread, when the store supports them.
    async fn read_thread_dynamic_tools(
        &self,
        _params: ReadThreadDynamicToolsParams,
    ) -> ThreadStoreResult<Option<Vec<DynamicToolSpec>>> {
        Ok(None)
    }

    /// Reads a rollout-backed thread by path when the store supports path-addressed lookups.
    ///
    /// Deprecated: new callers should use [`ThreadStore::read_thread`] instead.
    async fn read_thread_by_rollout_path(
        &self,
        params: ReadThreadByRolloutPathParams,
    ) -> ThreadStoreResult<StoredThread>;

    /// Lists stored threads matching the supplied filters.
    async fn list_threads(&self, params: ListThreadsParams) -> ThreadStoreResult<ThreadPage>;

    /// Searches stored threads and returns search-only preview metadata.
    async fn search_threads(
        &self,
        _params: SearchThreadsParams,
    ) -> ThreadStoreResult<ThreadSearchPage> {
        Err(ThreadStoreError::Unsupported {
            operation: "thread/search",
        })
    }

    /// Lists turns within a stored thread.
    async fn list_turns(&self, _params: ListTurnsParams) -> ThreadStoreResult<TurnPage> {
        Err(ThreadStoreError::Unsupported {
            operation: "list_turns",
        })
    }

    /// Lists persisted items within a stored turn.
    async fn list_items(&self, _params: ListItemsParams) -> ThreadStoreResult<ItemPage> {
        Err(ThreadStoreError::Unsupported {
            operation: "list_items",
        })
    }

    /// Applies a literal metadata patch and returns the updated thread.
    ///
    /// Implementations should apply the supplied fields directly. Policy such as deciding whether
    /// an append-derived preview should be emitted belongs above the store.
    async fn update_thread_metadata(
        &self,
        params: UpdateThreadMetadataParams,
    ) -> ThreadStoreResult<StoredThread>;

    /// Archives a thread.
    async fn archive_thread(&self, params: ArchiveThreadParams) -> ThreadStoreResult<()>;

    /// Unarchives a thread and returns its updated metadata.
    async fn unarchive_thread(
        &self,
        params: ArchiveThreadParams,
    ) -> ThreadStoreResult<StoredThread>;
}

/// Thread-store implementation for contexts where persistence is intentionally unavailable.
#[derive(Default)]
pub struct UnsupportedThreadStore;

impl UnsupportedThreadStore {
    /// Create an unsupported thread store.
    pub fn new() -> Self {
        Self
    }

    fn unsupported<T>(operation: &'static str) -> ThreadStoreResult<T> {
        Err(ThreadStoreError::Unsupported { operation })
    }
}

#[async_trait]
impl ThreadStore for UnsupportedThreadStore {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn create_thread(&self, _params: CreateThreadParams) -> ThreadStoreResult<()> {
        Self::unsupported("create_thread")
    }

    async fn resume_thread(&self, _params: ResumeThreadParams) -> ThreadStoreResult<()> {
        Self::unsupported("resume_thread")
    }

    async fn append_items(&self, _params: AppendThreadItemsParams) -> ThreadStoreResult<()> {
        Self::unsupported("append_items")
    }

    async fn persist_thread(&self, _thread_id: ThreadId) -> ThreadStoreResult<()> {
        Self::unsupported("persist_thread")
    }

    async fn flush_thread(&self, _thread_id: ThreadId) -> ThreadStoreResult<()> {
        Self::unsupported("flush_thread")
    }

    async fn shutdown_thread(&self, _thread_id: ThreadId) -> ThreadStoreResult<()> {
        Self::unsupported("shutdown_thread")
    }

    async fn discard_thread(&self, _thread_id: ThreadId) -> ThreadStoreResult<()> {
        Self::unsupported("discard_thread")
    }

    async fn load_history(
        &self,
        _params: LoadThreadHistoryParams,
    ) -> ThreadStoreResult<StoredThreadHistory> {
        Self::unsupported("load_history")
    }

    async fn read_thread(&self, _params: ReadThreadParams) -> ThreadStoreResult<StoredThread> {
        Self::unsupported("read_thread")
    }

    async fn read_thread_by_rollout_path(
        &self,
        _params: ReadThreadByRolloutPathParams,
    ) -> ThreadStoreResult<StoredThread> {
        Self::unsupported("read_thread_by_rollout_path")
    }

    async fn list_threads(&self, _params: ListThreadsParams) -> ThreadStoreResult<ThreadPage> {
        Self::unsupported("list_threads")
    }

    async fn update_thread_metadata(
        &self,
        _params: UpdateThreadMetadataParams,
    ) -> ThreadStoreResult<StoredThread> {
        Self::unsupported("update_thread_metadata")
    }

    async fn archive_thread(&self, _params: ArchiveThreadParams) -> ThreadStoreResult<()> {
        Self::unsupported("archive_thread")
    }

    async fn unarchive_thread(
        &self,
        _params: ArchiveThreadParams,
    ) -> ThreadStoreResult<StoredThread> {
        Self::unsupported("unarchive_thread")
    }
}
