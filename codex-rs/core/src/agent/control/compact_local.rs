//! Fork-local context compaction for live subagents.
//!
//! This is a single-purpose sibling module of `agent::control`, holding the
//! fork-only [`AgentControl::compact_agent`] entry point. It is kept out of
//! `control.rs` so that upstream's churn on that file (e.g. its own
//! `control/{spawn,legacy}` module split) does not repeatedly collide with this
//! fork-added code at merge time. As a child module of `control` it can still
//! reach the module-private `upgrade()` / `handle_thread_request_result()`
//! helpers and the `state` field on [`AgentControl`].

use super::AgentControl;
use codex_protocol::ThreadId;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::protocol::Op;

impl AgentControl {
    /// Request context compaction for an existing agent thread.
    pub(crate) async fn compact_agent(&self, agent_id: ThreadId) -> CodexResult<String> {
        let state = self.upgrade()?;
        self.handle_thread_request_result(
            agent_id,
            &state,
            state.send_op(agent_id, Op::Compact).await,
        )
        .await
    }
}
