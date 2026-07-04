use super::AgentControl;
use crate::agent::AgentStatus;
use crate::codex_thread::CodexThread;
use crate::config::Config;
use crate::thread_manager::ThreadManagerState;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::SessionSource;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use tracing::warn;

/// A freshly spawned sub-agent is momentarily `PendingInit` with no active turn and an
/// empty mailbox — byte-for-byte the predicate used below to reclaim an *abandoned*
/// slot. `reserve_slot` commits residency *before* the agent's initial task has been
/// pulled into its mailbox and turned into a running turn, so a concurrent spawn's
/// eviction pass can briefly observe a healthy new agent in that exact state. To avoid
/// evicting such an agent, a `PendingInit` resident is only treated as abandoned once it
/// has held its slot for at least this long. Healthy agents leave `PendingInit` within
/// milliseconds; a genuinely abandoned one never does — so the grace cleanly separates
/// the two with no practical cost to reclaim latency.
const DEFAULT_PENDING_INIT_EVICTION_GRACE: Duration = Duration::from_secs(30);

#[derive(Default)]
pub(super) struct V2Residency {
    state: Mutex<V2ResidencyState>,
}

struct V2ResidencyState {
    residents: VecDeque<ThreadId>,
    pending_slots: usize,
    /// When each resident committed its slot. Used to age-gate `PendingInit` eviction
    /// (see `DEFAULT_PENDING_INIT_EVICTION_GRACE`). Inserted on commit; removed whenever
    /// the thread permanently leaves `residents`.
    commit_times: HashMap<ThreadId, Instant>,
    /// Minimum age before a `PendingInit` resident may be evicted. A field (rather than a
    /// bare const) so tests can pin it to zero (evict immediately) or a large value
    /// (never within the startup window).
    pending_init_grace: Duration,
}

impl Default for V2ResidencyState {
    fn default() -> Self {
        Self {
            residents: VecDeque::new(),
            pending_slots: 0,
            commit_times: HashMap::new(),
            pending_init_grace: DEFAULT_PENDING_INIT_EVICTION_GRACE,
        }
    }
}

pub(super) struct V2ResidencySlot {
    residency: Arc<V2Residency>,
    active: bool,
}

impl V2ResidencySlot {
    pub(super) fn commit(mut self, thread_id: ThreadId) {
        self.residency.commit_slot(thread_id);
        self.active = false;
    }
}

impl Drop for V2ResidencySlot {
    fn drop(&mut self) {
        if self.active {
            self.residency.release_pending_slot();
        }
    }
}

impl AgentControl {
    pub(super) async fn reserve_v2_residency_slot(
        &self,
        state: &Arc<ThreadManagerState>,
        config: &Config,
        protected_thread_id: Option<ThreadId>,
    ) -> CodexResult<V2ResidencySlot> {
        let capacity = config
            .effective_agent_max_threads(MultiAgentVersion::V2)?
            .unwrap_or(usize::MAX);
        Arc::clone(&self.v2_residency)
            .reserve_slot(state, capacity, protected_thread_id)
            .await
    }

    pub(super) async fn touch_loaded_v2_residency(
        &self,
        state: &Arc<ThreadManagerState>,
        thread_id: ThreadId,
    ) {
        if let Ok(thread) = state.get_thread(thread_id).await
            && is_resident_candidate(thread.as_ref())
        {
            self.v2_residency.touch(thread_id);
        }
    }

    pub(super) fn forget_v2_residency(&self, thread_id: ThreadId) {
        self.v2_residency.remove(thread_id);
    }
}

impl V2Residency {
    async fn reserve_slot(
        self: Arc<Self>,
        manager: &Arc<ThreadManagerState>,
        capacity: usize,
        protected_thread_id: Option<ThreadId>,
    ) -> CodexResult<V2ResidencySlot> {
        loop {
            if self.try_reserve_pending_slot(capacity) {
                return Ok(V2ResidencySlot {
                    residency: self,
                    active: true,
                });
            }
            if !self
                .try_unload_one_resident(manager, protected_thread_id)
                .await
            {
                return Err(CodexErr::AgentLimitReached {
                    max_threads: capacity,
                });
            }
        }
    }

    fn try_reserve_pending_slot(&self, capacity: usize) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.residents.len().saturating_add(state.pending_slots) >= capacity {
            return false;
        }
        state.pending_slots += 1;
        true
    }

    async fn try_unload_one_resident(
        &self,
        manager: &Arc<ThreadManagerState>,
        protected_thread_id: Option<ThreadId>,
    ) -> bool {
        let candidates_to_scan = self.resident_count();
        for _ in 0..candidates_to_scan {
            let Some(candidate_thread_id) = self.pop_lru_candidate(protected_thread_id) else {
                return false;
            };
            let Some(candidate_thread) = manager
                .get_thread(candidate_thread_id)
                .await
                .ok()
                .filter(|thread| is_resident_candidate(thread))
            else {
                // The thread is gone; it will never be re-inserted into `residents`, so
                // drop its commit time too rather than leaking the entry.
                self.forget_commit_time(candidate_thread_id);
                continue;
            };
            let pending_init_eviction_permitted =
                self.pending_init_eviction_permitted(candidate_thread_id);
            if !is_unloadable(candidate_thread.as_ref(), pending_init_eviction_permitted).await {
                self.touch(candidate_thread_id);
                continue;
            }
            candidate_thread.ensure_rollout_materialized().await;
            if let Err(err) = candidate_thread.shutdown_and_wait().await {
                warn!(
                    "failed to shut down v2 resident thread before unloading {candidate_thread_id}: {err}"
                );
                self.touch(candidate_thread_id);
                continue;
            }
            let _ = manager.remove_thread(&candidate_thread_id).await;
            self.forget_commit_time(candidate_thread_id);
            return true;
        }
        false
    }

    fn resident_count(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .residents
            .len()
    }

    fn pop_lru_candidate(&self, protected_thread_id: Option<ThreadId>) -> Option<ThreadId> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let candidates_to_scan = state.residents.len();
        for _ in 0..candidates_to_scan {
            let candidate_thread_id = state.residents.pop_front()?;
            if Some(candidate_thread_id) == protected_thread_id {
                state.residents.push_back(candidate_thread_id);
                continue;
            }
            return Some(candidate_thread_id);
        }
        None
    }

    fn touch(&self, thread_id: ThreadId) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        touch_resident(&mut state.residents, thread_id);
    }

    fn remove(&self, thread_id: ThreadId) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state
            .residents
            .retain(|resident_thread_id| *resident_thread_id != thread_id);
        state.commit_times.remove(&thread_id);
    }

    /// Drop only the recorded commit time for a thread that has permanently left
    /// `residents` (e.g. evicted, or found already gone). Leaves `residents` untouched.
    fn forget_commit_time(&self, thread_id: ThreadId) {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .commit_times
            .remove(&thread_id);
    }

    /// Whether a `PendingInit` resident has held its slot long enough to be treated as
    /// abandoned (past the grace window). An unknown commit time is treated as *not*
    /// aged, so a resident whose age cannot be established is never evicted while
    /// `PendingInit`.
    fn pending_init_eviction_permitted(&self, thread_id: ThreadId) -> bool {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state
            .commit_times
            .get(&thread_id)
            .is_some_and(|committed_at| committed_at.elapsed() >= state.pending_init_grace)
    }

    #[cfg(test)]
    fn set_pending_init_grace(&self, grace: Duration) {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pending_init_grace = grace;
    }

    fn commit_slot(&self, thread_id: ThreadId) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.pending_slots = state.pending_slots.saturating_sub(1);
        state.commit_times.insert(thread_id, Instant::now());
        touch_resident(&mut state.residents, thread_id);
    }

    fn release_pending_slot(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.pending_slots = state.pending_slots.saturating_sub(1);
    }
}

fn touch_resident(residents: &mut VecDeque<ThreadId>, thread_id: ThreadId) {
    residents.retain(|resident_thread_id| *resident_thread_id != thread_id);
    residents.push_back(thread_id);
}

fn is_resident_candidate(thread: &CodexThread) -> bool {
    thread.multi_agent_version() == Some(MultiAgentVersion::V2)
        && is_v2_resident_session_source(&thread.session_source)
}

pub(super) fn is_v2_resident_session_source(session_source: &SessionSource) -> bool {
    matches!(session_source, SessionSource::SubAgent(_))
}

async fn is_unloadable(thread: &CodexThread, pending_init_eviction_permitted: bool) -> bool {
    // Whether the agent's status permits reclaiming its residency slot. The idle
    // guards below (no active turn, no queued mailbox work) still apply, so an agent
    // that is merely *about* to run — e.g. a freshly spawned `PendingInit` agent whose
    // initial task is still queued in its mailbox — is never unloaded here.
    let status_permits_unload = match thread.agent_status().await {
        // Never started a turn and then abandoned while idle (nothing queued): without
        // this, the slot leaks for the whole session, because a `PendingInit` agent with
        // no queued work never emits a status event and so can never become reclaimable.
        // Age-gated (see `DEFAULT_PENDING_INIT_EVICTION_GRACE`): a healthy agent is only
        // *transiently* `PendingInit` right after commit, so it is protected until it has
        // held the slot past the grace window, distinguishing it from a truly abandoned one.
        AgentStatus::PendingInit => pending_init_eviction_permitted,
        // Finished, failed, or paused-and-idle: safe to drop to reclaim the slot. An
        // `Interrupted` agent "may receive more input", but — like the others here — is
        // only evicted when idle and the session is out of residency slots (LRU).
        AgentStatus::Completed(_) | AgentStatus::Errored(_) | AgentStatus::Interrupted => true,
        // Actively using the slot, already reclaimed via the close/shutdown path, or no
        // live thread to unload.
        AgentStatus::Running | AgentStatus::Shutdown | AgentStatus::NotFound => false,
    };
    status_permits_unload
        && thread.codex.session.active_turn.lock().await.is_none()
        && !thread
            .codex
            .session
            .input_queue
            .has_pending_mailbox_items()
            .await
}

#[cfg(test)]
#[path = "residency_tests.rs"]
mod tests;
