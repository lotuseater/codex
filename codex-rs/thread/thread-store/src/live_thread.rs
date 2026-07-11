use std::path::PathBuf;
use std::sync::Arc;

use codex_protocol::ThreadId;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::ThreadMemoryMode;
use codex_rollout::RolloutPersistenceTelemetry;
use codex_rollout::measure_and_filter_rollout_items;
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
use codex_thread_store_api::ThreadMetadataPatch;
use codex_thread_store_api::ThreadStore;
use codex_thread_store_api::ThreadStoreFuture;
use codex_thread_store_api::ThreadStoreResult;
use codex_thread_store_api::UpdateThreadMetadataParams;
use tokio::sync::Mutex;
use tracing::warn;

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
    history_mode: ThreadHistoryMode,
    thread_store: Arc<dyn ThreadStore>,
    metadata_sync: Arc<Mutex<ThreadMetadataSync>>,
    persistence_telemetry: RolloutPersistenceTelemetry,
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
        let history_mode = params.history_mode;
        let metadata_sync = ThreadMetadataSync::for_create(&params).await;
        thread_store.create_thread(params).await?;
        Ok(Self {
            thread_id,
            history_mode,
            thread_store,
            metadata_sync: Arc::new(Mutex::new(metadata_sync)),
            persistence_telemetry: RolloutPersistenceTelemetry::new(thread_id),
        })
    }

    pub async fn resume(
        thread_store: Arc<dyn ThreadStore>,
        history_mode: ThreadHistoryMode,
        params: ResumeThreadParams,
    ) -> ThreadStoreResult<Self> {
        let thread_id = params.thread_id;
        let should_load_history = params.history.is_none();
        let include_archived = params.include_archived;
        let mut metadata_sync = ThreadMetadataSync::for_resume(&params);
        thread_store.resume_thread(params).await?;
        if should_load_history {
            match thread_store
                .load_history(LoadThreadHistoryParams {
                    thread_id,
                    include_archived,
                })
                .await
            {
                Ok(history) => metadata_sync.record_resume_history(&history.items),
                Err(err) => {
                    if let Err(discard_err) = thread_store.discard_thread(thread_id).await {
                        warn!(
                            "failed to discard thread persistence after resume history load failed: {discard_err}"
                        );
                    }
                    return Err(err);
                }
            }
        }
        Ok(Self {
            thread_id,
            history_mode,
            thread_store,
            metadata_sync: Arc::new(Mutex::new(metadata_sync)),
            persistence_telemetry: RolloutPersistenceTelemetry::new(thread_id),
        })
    }

    #[tracing::instrument(
        level = "trace",
        skip_all,
        fields(item_count = raw_items.len())
    )]
    pub async fn append_items(&self, raw_items: &[RolloutItem]) -> ThreadStoreResult<()> {
        // Empty appends are intentionally ignored rather than represented as zero-sized batches.
        if raw_items.is_empty() {
            return Ok(());
        }
        let (items, measurement) = if self.persistence_telemetry.is_enabled() {
            let (items, measurement) =
                measure_and_filter_rollout_items(raw_items, self.history_mode);
            (items, Some(measurement))
        } else {
            (persisted_rollout_items(raw_items, self.history_mode), None)
        };
        self.thread_store
            .append_items(AppendThreadItemsParams {
                thread_id: self.thread_id,
                items: raw_items.to_vec(),
            })
            .await?;
        if let Some(measurement) = measurement.as_ref() {
            self.persistence_telemetry
                .record_batch(raw_items, measurement);
        }
        if items.is_empty() {
            return Ok(());
        }
        let update = self
            .metadata_sync
            .lock()
            .await
            .observe_appended_items(items.as_slice());
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
    fn append_items<'a>(&'a self, items: &'a [RolloutItem]) -> ThreadStoreFuture<'a, ()> {
        Box::pin(async move { LiveThread::append_items(self, items).await })
    }

    fn persist(&self) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move { LiveThread::persist(self).await })
    }

    fn flush(&self) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move { LiveThread::flush(self).await })
    }

    fn shutdown(&self) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move { LiveThread::shutdown(self).await })
    }

    fn discard(&self) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move { LiveThread::discard(self).await })
    }

    fn load_history(&self, include_archived: bool) -> ThreadStoreFuture<'_, StoredThreadHistory> {
        Box::pin(async move { LiveThread::load_history(self, include_archived).await })
    }

    fn read_thread(
        &self,
        include_archived: bool,
        include_history: bool,
    ) -> ThreadStoreFuture<'_, StoredThread> {
        Box::pin(
            async move { LiveThread::read_thread(self, include_archived, include_history).await },
        )
    }

    fn update_metadata(
        &self,
        patch: ThreadMetadataPatch,
        include_archived: bool,
    ) -> ThreadStoreFuture<'_, StoredThread> {
        Box::pin(async move { LiveThread::update_metadata(self, patch, include_archived).await })
    }

    fn update_memory_mode(
        &self,
        mode: ThreadMemoryMode,
        include_archived: bool,
    ) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move { LiveThread::update_memory_mode(self, mode, include_archived).await })
    }

    fn local_rollout_path(&self) -> ThreadStoreFuture<'_, Option<PathBuf>> {
        Box::pin(async move { LiveThread::local_rollout_path(self).await })
    }
}

impl LiveThreadFactory for StoreLiveThreadFactory {
    fn create<'a>(
        &'a self,
        thread_store: Arc<dyn ThreadStore>,
        params: CreateThreadParams,
    ) -> ThreadStoreFuture<'a, Arc<dyn LiveThreadHandle>> {
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
    ) -> ThreadStoreFuture<'a, Arc<dyn LiveThreadHandle>> {
        Box::pin(async move {
            LiveThread::resume(thread_store, params)
                .await
                .map(|live_thread| Arc::new(live_thread) as Arc<dyn LiveThreadHandle>)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use codex_protocol::models::BaseInstructions;
    use codex_protocol::protocol::EventMsg;
    use codex_protocol::protocol::SessionSource;
    use codex_protocol::protocol::UserMessageEvent;
    use codex_thread_store_api::CreateThreadParams;
    use codex_thread_store_api::ReadThreadParams;
    use codex_thread_store_api::RecordingThreadStore;
    use codex_thread_store_api::StoredThread;
    use codex_thread_store_api::ThreadEventPersistenceMode;
    use codex_thread_store_api::ThreadPersistenceMetadata;
    use codex_thread_store_api::ThreadStore;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn flush_applies_rollout_derived_preview_through_store_port() {
        let store = Arc::new(RecordingThreadStore::new());
        let thread_id = ThreadId::default();
        let live_thread = LiveThread::create(store.clone(), create_thread_params(thread_id))
            .await
            .expect("create live thread");
        let user_message = RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            message: "first user message".to_string(),
            images: None,
            local_images: Vec::new(),
            text_elements: Vec::new(),
            ..Default::default()
        }));

        live_thread
            .append_items(&[user_message.clone()])
            .await
            .expect("append rollout item");
        live_thread.flush().await.expect("flush live thread");

        let stored = read_stored_thread(store.as_ref(), thread_id).await;
        assert_eq!(stored.preview, "first user message");
        assert_eq!(
            stored.history.expect("stored history").items,
            vec![user_message]
        );
        assert_eq!(store.calls().append_items, 1);
        assert_eq!(store.calls().update_thread_metadata, 1);
    }

    #[tokio::test]
    async fn update_memory_mode_routes_thread_state_through_store_port() {
        let store = Arc::new(RecordingThreadStore::new());
        let thread_id = ThreadId::default();
        let live_thread = LiveThread::create(store.clone(), create_thread_params(thread_id))
            .await
            .expect("create live thread");

        live_thread
            .update_memory_mode(ThreadMemoryMode::Disabled, /*include_archived*/ false)
            .await
            .expect("update memory mode");

        assert_eq!(store.calls().update_thread_metadata, 2);
    }

    fn create_thread_params(thread_id: ThreadId) -> CreateThreadParams {
        CreateThreadParams {
            thread_id,
            forked_from_id: None,
            source: SessionSource::Exec,
            thread_source: None,
            base_instructions: BaseInstructions::default(),
            dynamic_tools: Vec::new(),
            metadata: ThreadPersistenceMetadata {
                cwd: Some(PathBuf::from("workspace")),
                model_provider: "test-provider".to_string(),
                memory_mode: ThreadMemoryMode::Enabled,
            },
            event_persistence_mode: ThreadEventPersistenceMode::Extended,
        }
    }

    async fn read_stored_thread(store: &RecordingThreadStore, thread_id: ThreadId) -> StoredThread {
        store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: true,
            })
            .await
            .expect("read stored thread")
    }
}
