use std::sync::Arc;

use codex_rollout::RolloutConfigView;
use codex_rollout::state_db::StateDbHandle;
use codex_thread_store_api::ThreadStore;
use codex_thread_store_api::ThreadStoreSelection;

use crate::InMemoryThreadStore;
use crate::LocalThreadStore;
use crate::LocalThreadStoreConfig;

/// Build a concrete thread store from application configuration inputs.
pub fn thread_store_from_config(
    config: &impl RolloutConfigView,
    selection: ThreadStoreSelection,
    state_db: Option<StateDbHandle>,
) -> Arc<dyn ThreadStore> {
    match selection {
        ThreadStoreSelection::Local => Arc::new(LocalThreadStore::new(
            LocalThreadStoreConfig::from_config(config),
            state_db,
        )),
        ThreadStoreSelection::InMemory { id } => InMemoryThreadStore::for_id(&id),
    }
}
