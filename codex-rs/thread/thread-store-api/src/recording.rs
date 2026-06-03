use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use codex_protocol::ThreadId;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::SessionSource;

use crate::AppendThreadItemsParams;
use crate::ArchiveThreadParams;
use crate::CreateThreadParams;
use crate::ItemPage;
use crate::ListItemsParams;
use crate::ListThreadsParams;
use crate::ListTurnsParams;
use crate::LiveThreadFactory;
use crate::LiveThreadHandle;
use crate::ReadThreadByRolloutPathParams;
use crate::ReadThreadDynamicToolsParams;
use crate::ReadThreadParams;
use crate::ResumeThreadParams;
use crate::StoredThread;
use crate::StoredThreadHistory;
use crate::ThreadMetadataPatch;
use crate::ThreadPage;
use crate::ThreadPersistenceMetadata;
use crate::ThreadPersistenceServices;
use crate::ThreadStore;
use crate::ThreadStoreError;
use crate::ThreadStoreFuture;
use crate::ThreadStoreResult;
use crate::TurnPage;
use crate::UpdateThreadMetadataParams;

/// Operation counters captured by [`RecordingThreadStore`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordingThreadStoreCalls {
    pub create_thread: usize,
    pub resume_thread: usize,
    pub append_items: usize,
    pub persist_thread: usize,
    pub flush_thread: usize,
    pub shutdown_thread: usize,
    pub discard_thread: usize,
    pub load_history: usize,
    pub read_thread: usize,
    pub read_thread_dynamic_tools: usize,
    pub read_thread_by_rollout_path: usize,
    pub list_threads: usize,
    pub list_turns: usize,
    pub list_items: usize,
    pub update_thread_metadata: usize,
    pub archive_thread: usize,
    pub unarchive_thread: usize,
}

/// Storage-neutral recording store for tests that need persistence behavior.
///
/// This type intentionally lives in the API crate so crates under test can exercise
/// `ThreadStore` behavior without depending on concrete production stores.
#[derive(Debug, Default)]
pub struct RecordingThreadStore {
    state: Mutex<RecordingThreadStoreState>,
}

#[derive(Debug, Default)]
struct RecordingThreadStoreState {
    calls: RecordingThreadStoreCalls,
    threads: HashMap<ThreadId, RecordingThread>,
    rollout_paths: HashMap<PathBuf, ThreadId>,
}

#[derive(Clone, Debug)]
struct RecordingThread {
    thread_id: ThreadId,
    rollout_path: Option<PathBuf>,
    forked_from_id: Option<ThreadId>,
    source: SessionSource,
    metadata: ThreadPersistenceMetadata,
    dynamic_tools: Vec<DynamicToolSpec>,
    patch: ThreadMetadataPatch,
    history: Vec<RolloutItem>,
    archived_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl RecordingThreadStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calls(&self) -> RecordingThreadStoreCalls {
        self.state
            .lock()
            .expect("recording store lock")
            .calls
            .clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, RecordingThreadStoreState> {
        self.state.lock().expect("recording store lock")
    }
}

impl RecordingThreadStoreState {
    fn store_rollout_path(&mut self, thread_id: ThreadId, rollout_path: &Option<PathBuf>) {
        if let Some(rollout_path) = rollout_path {
            self.rollout_paths.insert(rollout_path.clone(), thread_id);
        }
    }

    fn thread(&self, thread_id: ThreadId) -> ThreadStoreResult<&RecordingThread> {
        self.threads
            .get(&thread_id)
            .ok_or(ThreadStoreError::ThreadNotFound { thread_id })
    }

    fn thread_mut(&mut self, thread_id: ThreadId) -> ThreadStoreResult<&mut RecordingThread> {
        self.threads
            .get_mut(&thread_id)
            .ok_or(ThreadStoreError::ThreadNotFound { thread_id })
    }
}

impl RecordingThread {
    fn from_create(params: CreateThreadParams) -> Self {
        let now = Utc::now();
        Self {
            thread_id: params.thread_id,
            rollout_path: None,
            forked_from_id: params.forked_from_id,
            source: params.source,
            metadata: params.metadata,
            dynamic_tools: params.dynamic_tools,
            patch: ThreadMetadataPatch::default(),
            history: Vec::new(),
            archived_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn from_resume(params: ResumeThreadParams) -> Self {
        let now = Utc::now();
        Self {
            thread_id: params.thread_id,
            rollout_path: params.rollout_path,
            forked_from_id: None,
            source: SessionSource::Exec,
            metadata: params.metadata,
            dynamic_tools: Vec::new(),
            patch: ThreadMetadataPatch::default(),
            history: params.history.unwrap_or_default(),
            archived_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn stored_thread(&self, include_history: bool) -> StoredThread {
        StoredThread {
            thread_id: self.thread_id,
            rollout_path: self.rollout_path.clone(),
            forked_from_id: self.forked_from_id,
            preview: self.patch.preview.clone().unwrap_or_default(),
            name: self.patch.name.clone().flatten(),
            model_provider: self
                .patch
                .model_provider
                .clone()
                .unwrap_or_else(|| self.metadata.model_provider.clone()),
            model: self.patch.model.clone(),
            reasoning_effort: self.patch.reasoning_effort.clone(),
            created_at: self.patch.created_at.unwrap_or(self.created_at),
            updated_at: self.patch.updated_at.unwrap_or(self.updated_at),
            archived_at: self.archived_at,
            cwd: self.metadata.cwd.clone().unwrap_or_default(),
            cli_version: self.patch.cli_version.clone().unwrap_or_default(),
            source: self
                .patch
                .source
                .clone()
                .unwrap_or_else(|| self.source.clone()),
            thread_source: self.patch.thread_source.clone().flatten(),
            agent_nickname: self.patch.agent_nickname.clone().flatten(),
            agent_role: self.patch.agent_role.clone().flatten(),
            agent_path: self.patch.agent_path.clone().flatten(),
            git_info: None,
            approval_mode: self
                .patch
                .approval_mode
                .clone()
                .unwrap_or(AskForApproval::Never),
            permission_profile: self
                .patch
                .permission_profile
                .clone()
                .unwrap_or_else(PermissionProfile::read_only),
            token_usage: self.patch.token_usage.clone(),
            first_user_message: self.patch.first_user_message.clone(),
            history: include_history.then(|| StoredThreadHistory {
                thread_id: self.thread_id,
                items: self.history.clone(),
            }),
        }
    }
}

#[async_trait]
impl ThreadStore for RecordingThreadStore {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn create_thread(&self, params: CreateThreadParams) -> ThreadStoreResult<()> {
        let mut state = self.lock();
        state.calls.create_thread += 1;
        state
            .threads
            .insert(params.thread_id, RecordingThread::from_create(params));
        Ok(())
    }

    async fn resume_thread(&self, params: ResumeThreadParams) -> ThreadStoreResult<()> {
        let mut state = self.lock();
        state.calls.resume_thread += 1;
        let thread_id = params.thread_id;
        if let Some(thread) = state.threads.get_mut(&thread_id) {
            if params.rollout_path.is_some() {
                thread.rollout_path = params.rollout_path;
            }
            if let Some(history) = params.history {
                thread.history = history;
            }
            thread.metadata = params.metadata;
            thread.updated_at = Utc::now();
        } else {
            state
                .threads
                .insert(thread_id, RecordingThread::from_resume(params));
        }
        let rollout_path = state
            .threads
            .get(&thread_id)
            .and_then(|thread| thread.rollout_path.as_ref().map(std::clone::Clone::clone));
        state.store_rollout_path(thread_id, &rollout_path);
        Ok(())
    }

    async fn append_items(&self, params: AppendThreadItemsParams) -> ThreadStoreResult<()> {
        let mut state = self.lock();
        state.calls.append_items += 1;
        let thread = state.thread_mut(params.thread_id)?;
        thread.history.extend(params.items);
        thread.updated_at = Utc::now();
        Ok(())
    }

    async fn persist_thread(&self, thread_id: ThreadId) -> ThreadStoreResult<()> {
        let mut state = self.lock();
        state.calls.persist_thread += 1;
        state.thread(thread_id)?;
        Ok(())
    }

    async fn flush_thread(&self, thread_id: ThreadId) -> ThreadStoreResult<()> {
        let mut state = self.lock();
        state.calls.flush_thread += 1;
        state.thread(thread_id)?;
        Ok(())
    }

    async fn shutdown_thread(&self, thread_id: ThreadId) -> ThreadStoreResult<()> {
        let mut state = self.lock();
        state.calls.shutdown_thread += 1;
        let thread = state.thread_mut(thread_id)?;
        thread.updated_at = Utc::now();
        Ok(())
    }

    async fn discard_thread(&self, thread_id: ThreadId) -> ThreadStoreResult<()> {
        let mut state = self.lock();
        state.calls.discard_thread += 1;
        state.threads.remove(&thread_id);
        state
            .rollout_paths
            .retain(|_, mapped_thread_id| *mapped_thread_id != thread_id);
        Ok(())
    }

    async fn load_history(
        &self,
        params: crate::LoadThreadHistoryParams,
    ) -> ThreadStoreResult<StoredThreadHistory> {
        let mut state = self.lock();
        state.calls.load_history += 1;
        let thread = state.thread(params.thread_id)?;
        if thread.archived_at.is_some() && !params.include_archived {
            return Err(ThreadStoreError::ThreadNotFound {
                thread_id: params.thread_id,
            });
        }
        Ok(StoredThreadHistory {
            thread_id: params.thread_id,
            items: thread.history.clone(),
        })
    }

    async fn read_thread(&self, params: ReadThreadParams) -> ThreadStoreResult<StoredThread> {
        let mut state = self.lock();
        state.calls.read_thread += 1;
        let thread = state.thread(params.thread_id)?;
        if thread.archived_at.is_some() && !params.include_archived {
            return Err(ThreadStoreError::ThreadNotFound {
                thread_id: params.thread_id,
            });
        }
        Ok(thread.stored_thread(params.include_history))
    }

    async fn read_thread_dynamic_tools(
        &self,
        params: ReadThreadDynamicToolsParams,
    ) -> ThreadStoreResult<Option<Vec<DynamicToolSpec>>> {
        let mut state = self.lock();
        state.calls.read_thread_dynamic_tools += 1;
        let thread = state.thread(params.thread_id)?;
        Ok(Some(thread.dynamic_tools.clone()))
    }

    async fn read_thread_by_rollout_path(
        &self,
        params: ReadThreadByRolloutPathParams,
    ) -> ThreadStoreResult<StoredThread> {
        let mut state = self.lock();
        state.calls.read_thread_by_rollout_path += 1;
        let Some(thread_id) = state.rollout_paths.get(&params.rollout_path).copied() else {
            return Err(ThreadStoreError::InvalidRequest {
                message: format!(
                    "recording thread store does not know rollout path {}",
                    params.rollout_path.display()
                ),
            });
        };
        let thread = state.thread(thread_id)?;
        if thread.archived_at.is_some() && !params.include_archived {
            return Err(ThreadStoreError::ThreadNotFound { thread_id });
        }
        Ok(thread.stored_thread(params.include_history))
    }

    async fn list_threads(&self, params: ListThreadsParams) -> ThreadStoreResult<ThreadPage> {
        let mut state = self.lock();
        state.calls.list_threads += 1;
        let items = state
            .threads
            .values()
            .filter(|thread| params.archived || thread.archived_at.is_none())
            .map(|thread| thread.stored_thread(/*include_history*/ false))
            .collect();
        Ok(ThreadPage {
            items,
            next_cursor: None,
        })
    }

    async fn list_turns(&self, _params: ListTurnsParams) -> ThreadStoreResult<TurnPage> {
        let mut state = self.lock();
        state.calls.list_turns += 1;
        Ok(TurnPage {
            turns: Vec::new(),
            next_cursor: None,
            backwards_cursor: None,
        })
    }

    async fn list_items(&self, _params: ListItemsParams) -> ThreadStoreResult<ItemPage> {
        let mut state = self.lock();
        state.calls.list_items += 1;
        Ok(ItemPage {
            items: Vec::new(),
            next_cursor: None,
            backwards_cursor: None,
        })
    }

    async fn update_thread_metadata(
        &self,
        params: UpdateThreadMetadataParams,
    ) -> ThreadStoreResult<StoredThread> {
        let mut state = self.lock();
        state.calls.update_thread_metadata += 1;
        let thread = state.thread_mut(params.thread_id)?;
        thread.patch.merge(params.patch);
        thread.updated_at = Utc::now();
        Ok(thread.stored_thread(/*include_history*/ false))
    }

    async fn archive_thread(&self, params: ArchiveThreadParams) -> ThreadStoreResult<()> {
        let mut state = self.lock();
        state.calls.archive_thread += 1;
        let thread = state.thread_mut(params.thread_id)?;
        thread.archived_at = Some(Utc::now());
        thread.updated_at = Utc::now();
        Ok(())
    }

    async fn unarchive_thread(
        &self,
        params: ArchiveThreadParams,
    ) -> ThreadStoreResult<StoredThread> {
        let mut state = self.lock();
        state.calls.unarchive_thread += 1;
        let thread = state.thread_mut(params.thread_id)?;
        thread.archived_at = None;
        thread.updated_at = Utc::now();
        Ok(thread.stored_thread(/*include_history*/ false))
    }
}

/// Live-thread factory backed by [`RecordingThreadStore`].
#[derive(Clone, Debug, Default)]
pub struct RecordingLiveThreadFactory;

impl RecordingLiveThreadFactory {
    pub fn new() -> Self {
        Self
    }
}

impl ThreadPersistenceServices {
    /// Build in-memory recording persistence services for manager tests.
    pub fn recording() -> Self {
        Self::new(
            Arc::new(RecordingThreadStore::default()),
            Arc::new(RecordingLiveThreadFactory::new()),
        )
    }
}

impl LiveThreadFactory for RecordingLiveThreadFactory {
    fn create<'a>(
        &'a self,
        thread_store: Arc<dyn ThreadStore>,
        params: CreateThreadParams,
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<Arc<dyn LiveThreadHandle>>> {
        Box::pin(async move {
            let thread_id = params.thread_id;
            thread_store.create_thread(params).await?;
            let live_thread: Arc<dyn LiveThreadHandle> = Arc::new(RecordingLiveThread {
                thread_store,
                thread_id,
                rollout_path: None,
            });
            Ok(live_thread)
        })
    }

    fn resume<'a>(
        &'a self,
        thread_store: Arc<dyn ThreadStore>,
        params: ResumeThreadParams,
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<Arc<dyn LiveThreadHandle>>> {
        Box::pin(async move {
            let thread_id = params.thread_id;
            let rollout_path = params.rollout_path.clone();
            thread_store.resume_thread(params).await?;
            let live_thread: Arc<dyn LiveThreadHandle> = Arc::new(RecordingLiveThread {
                thread_store,
                thread_id,
                rollout_path,
            });
            Ok(live_thread)
        })
    }
}

struct RecordingLiveThread {
    thread_store: Arc<dyn ThreadStore>,
    thread_id: ThreadId,
    rollout_path: Option<PathBuf>,
}

impl LiveThreadHandle for RecordingLiveThread {
    fn append_items<'a>(
        &'a self,
        items: &'a [RolloutItem],
    ) -> ThreadStoreFuture<'a, ThreadStoreResult<()>> {
        let thread_store = Arc::clone(&self.thread_store);
        let params = AppendThreadItemsParams {
            thread_id: self.thread_id,
            items: items.to_vec(),
        };
        Box::pin(async move { thread_store.append_items(params).await })
    }

    fn persist(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>> {
        let thread_store = Arc::clone(&self.thread_store);
        let thread_id = self.thread_id;
        Box::pin(async move { thread_store.persist_thread(thread_id).await })
    }

    fn flush(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>> {
        let thread_store = Arc::clone(&self.thread_store);
        let thread_id = self.thread_id;
        Box::pin(async move { thread_store.flush_thread(thread_id).await })
    }

    fn shutdown(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>> {
        Box::pin(async move { self.thread_store.shutdown_thread(self.thread_id).await })
    }

    fn discard(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<()>> {
        Box::pin(async move { self.thread_store.discard_thread(self.thread_id).await })
    }

    fn update_metadata(
        &self,
        patch: ThreadMetadataPatch,
        include_archived: bool,
    ) -> ThreadStoreFuture<'_, ThreadStoreResult<StoredThread>> {
        let params = UpdateThreadMetadataParams {
            thread_id: self.thread_id,
            patch,
            include_archived,
        };
        Box::pin(async move { self.thread_store.update_thread_metadata(params).await })
    }

    fn load_history(
        &self,
        include_archived: bool,
    ) -> ThreadStoreFuture<'_, ThreadStoreResult<StoredThreadHistory>> {
        let params = crate::LoadThreadHistoryParams {
            thread_id: self.thread_id,
            include_archived,
        };
        Box::pin(async move { self.thread_store.load_history(params).await })
    }

    fn read_thread(
        &self,
        include_archived: bool,
        include_history: bool,
    ) -> ThreadStoreFuture<'_, ThreadStoreResult<StoredThread>> {
        let params = ReadThreadParams {
            thread_id: self.thread_id,
            include_archived,
            include_history,
        };
        Box::pin(async move { self.thread_store.read_thread(params).await })
    }

    fn local_rollout_path(&self) -> ThreadStoreFuture<'_, ThreadStoreResult<Option<PathBuf>>> {
        Box::pin(async { Ok(self.rollout_path.clone()) })
    }
}
