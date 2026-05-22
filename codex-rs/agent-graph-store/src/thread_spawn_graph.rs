use codex_protocol::ThreadId;
use codex_state::StateRuntime;
use std::sync::Arc;

use crate::AgentGraphStore;
use crate::AgentGraphStoreResult;
use crate::LocalAgentGraphStore;
use crate::ThreadSpawnEdgeStatus;

/// Agent-owned facade for thread-spawn graph operations.
///
/// Callers use this policy object for common graph workflows instead of coupling
/// directly to a concrete graph store implementation or low-level state APIs.
#[derive(Clone, Debug)]
pub struct ThreadSpawnGraph<S> {
    store: S,
}

impl ThreadSpawnGraph<LocalAgentGraphStore> {
    /// Create a state-backed graph facade from an initialized state runtime.
    pub fn local(state_db: Arc<StateRuntime>) -> Self {
        Self::new(LocalAgentGraphStore::new(state_db))
    }
}

impl<S> ThreadSpawnGraph<S>
where
    S: AgentGraphStore,
{
    /// Create a graph facade around an agent graph store implementation.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Persist an open parent/child thread-spawn edge.
    pub async fn persist_open_edge(
        &self,
        parent_thread_id: ThreadId,
        child_thread_id: ThreadId,
    ) -> AgentGraphStoreResult<()> {
        self.store
            .upsert_thread_spawn_edge(
                parent_thread_id,
                child_thread_id,
                ThreadSpawnEdgeStatus::Open,
            )
            .await
    }

    /// Mark the edge that produced `child_thread_id` as closed.
    pub async fn mark_edge_closed(&self, child_thread_id: ThreadId) -> AgentGraphStoreResult<()> {
        self.store
            .set_thread_spawn_edge_status(child_thread_id, ThreadSpawnEdgeStatus::Closed)
            .await
    }

    /// List direct children for a parent thread regardless of edge status.
    pub async fn list_children(
        &self,
        parent_thread_id: ThreadId,
    ) -> AgentGraphStoreResult<Vec<ThreadId>> {
        self.store
            .list_thread_spawn_children(parent_thread_id, None)
            .await
    }

    /// List direct children for a parent thread filtered by edge status.
    pub async fn list_children_with_status(
        &self,
        parent_thread_id: ThreadId,
        status: ThreadSpawnEdgeStatus,
    ) -> AgentGraphStoreResult<Vec<ThreadId>> {
        self.store
            .list_thread_spawn_children(parent_thread_id, Some(status))
            .await
    }

    /// List descendants for a root thread regardless of edge status.
    pub async fn list_descendants(
        &self,
        root_thread_id: ThreadId,
    ) -> AgentGraphStoreResult<Vec<ThreadId>> {
        self.store
            .list_thread_spawn_descendants(root_thread_id, None)
            .await
    }

    /// List descendants for a root thread filtered by edge status.
    pub async fn list_descendants_with_status(
        &self,
        root_thread_id: ThreadId,
        status: ThreadSpawnEdgeStatus,
    ) -> AgentGraphStoreResult<Vec<ThreadId>> {
        self.store
            .list_thread_spawn_descendants(root_thread_id, Some(status))
            .await
    }
}
