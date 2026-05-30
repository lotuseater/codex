//! Context-budget mode controls for the `/slow` command.
//!
//! Keeping this wiring out of the slash dispatcher lets the command parser stay
//! thin while preserving one owner for local state, session override, and config
//! persistence updates.

use super::*;
use codex_protocol::config_types::ContextBudgetMode;

impl ChatWidget {
    pub(super) fn current_context_budget_mode(&self) -> ContextBudgetMode {
        self.config.context_budget_mode
    }

    pub(super) fn toggle_slow_mode_from_ui(&mut self) {
        let next_mode = if self.current_context_budget_mode() == ContextBudgetMode::Slow {
            ContextBudgetMode::Standard
        } else {
            ContextBudgetMode::Slow
        };
        self.set_context_budget_mode_selection(next_mode);
    }

    pub(super) fn set_context_budget_mode_selection(&mut self, mode: ContextBudgetMode) {
        self.config.context_budget_mode = mode;
        self.app_event_tx
            .send(AppEvent::CodexOp(AppCommand::override_turn_context(
                /*cwd*/ None,
                /*approval_policy*/ None,
                /*approvals_reviewer*/ None,
                /*permission_profile*/ None,
                /*active_permission_profile*/ None,
                /*windows_sandbox_level*/ None,
                /*model*/ None,
                /*effort*/ None,
                /*summary*/ None,
                /*service_tier*/ None,
                /*context_budget_mode*/ Some(mode),
                /*collaboration_mode*/ None,
                /*personality*/ None,
            )));
        self.app_event_tx
            .send(AppEvent::PersistContextBudgetModeSelection { mode });
    }
}
