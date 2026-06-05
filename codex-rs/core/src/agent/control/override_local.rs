//! Fork-local turn-context override for live subagents.
//!
//! This is a single-purpose sibling module of `agent::control`, holding the
//! fork-only [`AgentControl::override_agent_turn_context`] entry point. It is
//! kept out of `control.rs` so that upstream's churn on that file (e.g. its own
//! `control/{spawn,legacy}` module split) does not repeatedly collide with this
//! fork-added code at merge time. As a child module of `control` it can still
//! reach the module-private `upgrade()` / `handle_thread_request_result()`
//! helpers and the `state` field on [`AgentControl`].

use super::AgentControl;
use crate::codex_thread::CodexThreadSettingsOverrides;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::Op;

impl AgentControl {
    /// Override a live agent's persistent turn context before assigning follow-up work.
    pub(crate) async fn override_agent_turn_context(
        &self,
        agent_id: ThreadId,
        model: Option<String>,
        reasoning_effort: Option<ReasoningEffort>,
    ) -> CodexResult<String> {
        let state = self.upgrade()?;
        let thread = state.get_thread(agent_id).await?;
        let effort = reasoning_effort.map(Some);
        thread
            .validate_turn_context_overrides(CodexThreadSettingsOverrides {
                model: model.clone(),
                effort,
                ..Default::default()
            })
            .await
            .map_err(|err| CodexErr::UnsupportedOperation(err.to_string()))?;
        self.handle_thread_request_result(
            agent_id,
            &state,
            state
                .send_op(
                    agent_id,
                    Op::OverrideTurnContext {
                        cwd: None,
                        approval_policy: None,
                        approvals_reviewer: None,
                        sandbox_policy: None,
                        permission_profile: None,
                        windows_sandbox_level: None,
                        model,
                        effort,
                        summary: None,
                        service_tier: None,
                        context_budget_mode: None,
                        collaboration_mode: None,
                        personality: None,
                    },
                )
                .await,
        )
        .await
    }
}
