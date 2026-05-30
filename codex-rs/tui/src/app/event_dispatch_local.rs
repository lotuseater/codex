//! Fork-local `AppEvent` handlers split out of the central dispatcher.
//!
//! These handlers cover fork-specific events (auto-loop control and context-budget/"slow mode"
//! persistence) whose bodies would otherwise live inline in [`super::event_dispatch`]. Keeping them
//! here isolates fork logic from upstream churn in the main match, so the dispatcher arms stay as
//! thin one-line calls.

use super::*;
use crate::app_event::AutoLoopUpdate;

/// Body of `AppEvent::AutoLoop`.
pub(crate) fn on_auto_loop(app: &mut App, tui: &mut tui::Tui, update: AutoLoopUpdate) {
    app.handle_auto_loop_update(update);
    tui.frame_requester().schedule_frame();
}

/// Body of `AppEvent::SubmitAutoLoopAfterSelfReview`.
pub(crate) fn on_submit_auto_loop_after_self_review(app: &mut App, tui: &mut tui::Tui) {
    app.handle_auto_loop_after_self_review();
    tui.frame_requester().schedule_frame();
}

/// Body of `AppEvent::PersistContextBudgetModeSelection`.
pub(crate) async fn on_persist_context_budget_mode_selection(
    app: &mut App,
    mode: codex_protocol::config_types::ContextBudgetMode,
) {
    app.refresh_status_line();
    let profile = app.active_profile.as_deref();
    app.config.context_budget_mode = mode;
    match ConfigEditsBuilder::new(&app.config.codex_home)
        .with_profile(profile)
        .set_context_budget_mode(mode)
        .apply()
        .await
    {
        Ok(()) => {
            let status = if mode == codex_protocol::config_types::ContextBudgetMode::Slow {
                "on"
            } else {
                "off"
            };
            let mut message = format!("Slow mode set to {status}");
            if let Some(profile) = profile {
                message.push_str(" for ");
                message.push_str(profile);
                message.push_str(" profile");
            }
            app.chat_widget.add_info_message(message, /*hint*/ None);
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to persist slow mode selection");
            if let Some(profile) = profile {
                app.chat_widget.add_error_message(format!(
                    "Failed to save Slow mode for profile `{profile}`: {err}"
                ));
            } else {
                app.chat_widget
                    .add_error_message(format!("Failed to save default Slow mode: {err}"));
            }
        }
    }
}
