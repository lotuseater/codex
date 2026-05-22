use codex_agent_graph_store::AgentGraphStoreResult;
use codex_agent_graph_store::ThreadSpawnEdgeStatus;
use codex_agent_graph_store::ThreadSpawnGraph;
use codex_protocol::ThreadId;

use crate::StateDbHandle;

pub(crate) async fn persist_open_thread_spawn_edge(
    state_db: &StateDbHandle,
    parent_thread_id: ThreadId,
    child_thread_id: ThreadId,
) -> AgentGraphStoreResult<()> {
    local_graph(state_db)
        .persist_open_edge(parent_thread_id, child_thread_id)
        .await
}

pub(crate) async fn list_thread_spawn_children(
    state_db: &StateDbHandle,
    parent_thread_id: ThreadId,
    status_filter: Option<ThreadSpawnEdgeStatus>,
) -> AgentGraphStoreResult<Vec<ThreadId>> {
    let graph = local_graph(state_db);
    match status_filter {
        Some(status) => {
            graph
                .list_children_with_status(parent_thread_id, status)
                .await
        }
        None => graph.list_children(parent_thread_id).await,
    }
}

pub(crate) async fn list_open_thread_spawn_children(
    state_db: &StateDbHandle,
    parent_thread_id: ThreadId,
) -> AgentGraphStoreResult<Vec<ThreadId>> {
    list_thread_spawn_children(
        state_db,
        parent_thread_id,
        Some(ThreadSpawnEdgeStatus::Open),
    )
    .await
}

pub(crate) async fn list_thread_spawn_descendants(
    state_db: &StateDbHandle,
    root_thread_id: ThreadId,
    status_filter: Option<ThreadSpawnEdgeStatus>,
) -> AgentGraphStoreResult<Vec<ThreadId>> {
    let graph = local_graph(state_db);
    match status_filter {
        Some(status) => {
            graph
                .list_descendants_with_status(root_thread_id, status)
                .await
        }
        None => graph.list_descendants(root_thread_id).await,
    }
}

pub(crate) async fn mark_thread_spawn_edge_closed(
    state_db: &StateDbHandle,
    child_thread_id: ThreadId,
) -> AgentGraphStoreResult<()> {
    local_graph(state_db)
        .mark_edge_closed(child_thread_id)
        .await
}

fn local_graph(
    state_db: &StateDbHandle,
) -> ThreadSpawnGraph<impl codex_agent_graph_store::AgentGraphStore> {
    ThreadSpawnGraph::local(state_db.clone())
}
