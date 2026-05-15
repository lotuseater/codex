//! TUI metadata and inactive-activity rendering for collaboration agents.
//!
//! The app owns navigation and active-thread routing. The chat widget owns the
//! render cache needed to label historical agent activity consistently.

use super::*;

impl ChatWidget {
    pub(crate) fn set_collab_agent_runtime_details(
        &mut self,
        thread_id: ThreadId,
        model: Option<String>,
        reasoning_effort: Option<ReasoningEffortConfig>,
    ) {
        let metadata = self.collab_agent_metadata.entry(thread_id).or_default();
        metadata.model = model;
        metadata.reasoning_effort = reasoning_effort;
    }

    pub(crate) fn set_collab_agent_token_context_percent_used(
        &mut self,
        thread_id: ThreadId,
        token_context_percent_used: Option<i64>,
    ) {
        self.collab_agent_metadata
            .entry(thread_id)
            .or_default()
            .token_context_percent_used = token_context_percent_used;
    }

    pub(crate) fn on_inactive_collab_agent_activity(
        &mut self,
        thread_id: ThreadId,
        notification: &ServerNotification,
    ) {
        let metadata = self.collab_agent_metadata(thread_id);
        if let Some(cell) = crate::multi_agents::subagent_activity_history_cell_for_notification(
            thread_id,
            notification,
            &metadata,
        ) {
            self.app_event_tx
                .send(AppEvent::InsertHistoryCell(Box::new(cell)));
        }
    }
}
