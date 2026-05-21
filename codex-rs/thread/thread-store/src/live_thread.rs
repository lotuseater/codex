use std::path::PathBuf;
use std::sync::Arc;

use codex_protocol::ThreadId;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::ThreadMemoryMode;
use codex_rollout::EventPersistenceMode;
use codex_rollout::persisted_rollout_items;
use codex_thread_store_api::AppendThreadItemsParams;
use codex_thread_store_api::CreateThreadParams;
use codex_thread_store_api::LiveThreadFactory;
use codex_thread_store_api::LiveThreadHandle;
use codex_thread_store_api::LoadThreadHistoryParams;
use codex_thread_store_api::ReadThreadParams;
use codex_thread_store_api::ResumeThreadParams;
use codex_thread_store_api::StoredThread;
use codex_thread_store_api::StoredThreadHistory;
use codex_thread_store_api::ThreadEventPersistenceMode;
use codex_thread_store_api::ThreadMetadataPatch;
use codex_thread_store_api::ThreadStore;
use codex_thread_store_api::ThreadStoreFuture;
use codex_thread_store_api::ThreadStoreResult;
use codex_thread_store_api::UpdateThreadMetadataParams;
use tokio::sync::Mutex;

use crate::LocalThreadStore;
use crate::thread_metadata_sync::ThreadMetadataSync;

/// Handle for an active thread's persistence lifecycle.
///
/// `LiveThread` keeps lifecycle decisions with the caller while delegating storage details to
/// [`ThreadStore`]. Local stores may use a rollout file internally and remote stores may use a
/// service, but session code should only need this handle for the active thread.
#[derive(Clone)]
pub struct LiveThread {
    thread_id: ThreadId,
    thread_store: Arc<dyn ThreadStore>,
    event_persistence_mode: EventPersistenceMode,
    metadata_sync: Arc<Mutex<ThreadMetadataSync>>,
}

#[derive(Default)]
pub struct StoreLiveThreadFactory;

impl StoreLiveThreadFactory {
    pub fn new() -> Self {
        Self
    }
}

impl LiveThread {
    pub async fn create(
        thread_store: Arc<dyn ThreadStore>,
        params: CreateThreadParams,
    ) -> ThreadStoreResult<Self> {
        let thread_id = params.thread_id;
        let event_persistence_mode = event_persistence_mode(params.event_persistence_mode);
        let metadata_sync = ThreadMetadataSync::for_create(&params).await;
        thread_store.create_thread(params).await?;
        Ok(Self {
            thread_id,
            thread_store,
            event_persistence_mode,
            metadata_sync: Arc::new(Mutex::new(metadata_sync)),
        })
    }

    pub async fn resume(
        thread_store: Arc<dyn ThreadStore>,
        mut params: ResumeThreadParams,
    ) -> ThreadStoreResult<Self> {
        let thread_id = params.thread_id;
        let event_persistence_mode = event_persistence_mode(params.event_persistence_mode);
        let should_load_history = params.history.is_none();
        let include_archived = params.include_archived;
        thread_store.resume_thread(params.clone()).await?;
        if should_load_history {
            match thread_store
                .load_history(LoadThreadHistoryParams {
                    thread_id,
                    include_archived,
                })
                .await
            {
                Ok(history) => params.history = Some(history.items),
                Err(err) => {
                    let _ = thread_store.discard_thread(thread_id).await;
                    return Err(err);
                }
            }
        }
        let metadata_sync = ThreadMetadataSync::for_resume(&params);
        Ok(Self {
            thread_id,
            thread_store,
            event_persistence_mode,
            metadata_sync: Arc::new(Mutex::new(metadata_sync)),
        })
    }

    pub async fn append_items(&self, items: &[RolloutItem]) -> ThreadStoreResult<()> {
        let canonical_items = persisted_rollout_items(items, self.event_persistence_mode);
        if canonical_items.is_empty() {
            return Ok(());
        }
        self.thread_store
            .append_items(AppendThreadItemsParams {
                thread_id: self.thread_id,
                items: canonical_items.clone(),
            })
            .await?;
        let update = self
            .metadata_sync
            .lock()
            .await
            .observe_appended_items(canonical_items.as_slice());
        if let Some(update) = update {
            self.thread_store
                .update_thread_metadata(UpdateThreadMetadataParams {
                    thread_id: self.thread_id,
                    patch: update.patch.clone(),
                    include_archived: true,
                })
                .await?;
            self.metadata_sync
                .lock()
                .await
                .mark_pending_update_applied(&update);
        }
        Ok(())
    }

    pub async fn persist(&self) -> ThreadStoreResult<()> {
        self.thread_store.persist_thread(self.thread_id).await?;
        self.flush_pending_metadata_update().await
    }

    pub async fn flush(&self) -> ThreadStoreResult<()> {
        self.thread_store.flush_thread(self.thread_id).await?;
        self.flush_pending_metadata_update_for_existing_history()
            .await
    }

    pub async fn shutdown(&self) -> ThreadStoreResult<()> {
        self.flush_pending_metadata_update_for_existing_history()
            .await?;
        self.thread_store.shutdown_thread(self.thread_id).await
    }

    pub async fn discard(&self) -> ThreadStoreResult<()> {
        self.thread_store.discard_thread(self.thread_id).await
    }

    pub async fn load_history(
        &self,
        include_archived: bool,
    ) -> ThreadStoreResult<StoredThreadHistory> {
        self.thread_store
            .load_history(LoadThreadHistoryParams {
                thread_id: self.thread_id,
                include_archived,
            })
            .await
    }

    pub async fn read_thread(
        &self,
        include_archived: bool,
        include_history: bool,
    ) -> ThreadStoreResult<StoredThread> {
        self.thread_store
            .read_thread(ReadThreadParams {
                thread_id: self.thread_id,
                include_archived,
                include_history,
            })
            .await
    }

    pub async fn update_memory_mode(
        &self,
        mode: ThreadMemoryMode,
        include_archived: bool,
    ) -> ThreadStoreResult<()> {
        self.flush_pending_metadata_update().await?;
        self.thread_store
            .update_thread_metadata(UpdateThreadMetadataParams {
                thread_id: self.thread_id,
                patch: ThreadMetadataPatch {
                    memory_mode: Some(mode),
                    ..Default::default()
                },
                include_archived,
            })
            .await?;
        Ok(())
    }

    pub async fn update_metadata(
        &self,
        patch: ThreadMetadataPatch,
        include_archived: bool,
    ) -> ThreadStoreResult<StoredThread> {
        self.flush_pending_metadata_update().await?;
        self.thread_store
            .update_thread_metadata(UpdateThreadMetadataParams {
                thread_id: self.thread_id,
                patch,
                include_archived,
            })
            .await
    }

    /// Returns the live local rollout path for legacy local-only callers.
    ///
    /// Remote stores do not expose rollout files, so they return `Ok(None)`.
    pub async fn local_rollout_path(&self) -> ThreadStoreResult<Option<PathBuf>> {
        let Some(local_store) = self
            .thread_store
            .as_any()
            .downcast_ref::<LocalThreadStore>()
        else {
            return Ok(None);
        };
        local_store
            .live_rollout_path(self.thread_id)
            .await
            .map(Some)
    }

    async fn flush_pending_metadata_update(&self) -> ThreadStoreResult<()> {
        let update = self.metadata_sync.lock().await.take_pending_update();
        self.apply_pending_metadata_update(update).await
    }

    async fn flush_pending_metadata_update_for_existing_history(&self) -> ThreadStoreResult<()> {
        let update = self
            .metadata_sync
            .lock()
            .await
            .take_pending_update_for_existing_history();
        self.apply_pending_metadata_update(update).await
    }

    async fn apply_pending_metadata_update(
        &self,
        update: Option<crate::thread_metadata_sync::PendingThreadMetadataPatch>,
    ) -> ThreadStoreResult<()> {
        let Some(update) = update else {
            return Ok(());
        };
        self.thread_store
            .update_thread_metadata(UpdateThreadMetadataParams {
                thread_id: self.thread_id,
                patch: update.patch.clone(),
                include_archived: true,
            })
            .await?;
        self.metadata_sync
            .lock()
            .await
            .mark_pending_update_applied(&update);
        Ok(())
    }
}

impl LiveThreadHandle for LiveThread {
    fn append_items<'a>(
        &'a self,
        items: &'a [RolloutItem],
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<()>> {
        Box::pin(async move { LiveThread::append_items(self, items).await })
    }

    fn persist(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>> {
        Box::pin(async move { LiveThread::persist(self).await })
    }

    fn flush(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>> {
        Box::pin(async move { LiveThread::flush(self).await })
    }

    fn shutdown(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>> {
        Box::pin(async move { LiveThread::shutdown(self).await })
    }

    fn discard(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>> {
        Box::pin(async move { LiveThread::discard(self).await })
    }

    fn load_history(
        &self,
        include_archived: bool,
    ) -> ThreadStoreFuture<'_, ThreadStoreResult<StoredThreadHistory>> {
        Box::pin(async move { LiveThread::load_history(self, include_archived).await })
    }

    fn read_thread(
        &self,
        include_archived: bool,
        include_history: bool,
    ) -> ThreadStoreFuture<'_, ThreadStoreResult<StoredThread>> {
        Box::pin(
            async move { LiveThread::read_thread(self, include_archived, include_history).await },
        )
    }

    fn update_metadata(
        &self,
        patch: ThreadMetadataPatch,
        include_archived: bool,
    ) -> ThreadStoreFuture<'_, ThreadStoreResult<StoredThread>> {
        Box::pin(async move { LiveThread::update_metadata(self, patch, include_archived).await })
    }

    fn local_rollout_path(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<Option<PathBuf>>> {
        Box::pin(async move { LiveThread::local_rollout_path(self).await })
    }
}

impl LiveThreadFactory for StoreLiveThreadFactory {
    fn create<'a>(
        &'a self,
        thread_store: Arc<dyn ThreadStore>,
        params: CreateThreadParams,
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<Arc<dyn LiveThreadHandle>>> {
        Box::pin(async move {
            LiveThread::create(thread_store, params)
                .await
                .map(|live_thread| Arc::new(live_thread) as Arc<dyn LiveThreadHandle>)
        })
    }

    fn resume<'a>(
        &'a self,
        thread_store: Arc<dyn ThreadStore>,
        params: ResumeThreadParams,
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<Arc<dyn LiveThreadHandle>>> {
        Box::pin(async move {
            LiveThread::resume(thread_store, params)
                .await
                .map(|live_thread| Arc::new(live_thread) as Arc<dyn LiveThreadHandle>)
        })
    }
}

fn event_persistence_mode(mode: ThreadEventPersistenceMode) -> EventPersistenceMode {
    match mode {
        ThreadEventPersistenceMode::Limited => EventPersistenceMode::Limited,
        ThreadEventPersistenceMode::Extended => EventPersistenceMode::Extended,
    }
}
