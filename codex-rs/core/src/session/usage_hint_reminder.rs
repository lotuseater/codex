//! Per-model-request "usage-hint cadence" reminder.
//!
//! This is a decoupled sibling of [`super::time_reminder`]. Where
//! `time_reminder` re-injects the current-time reminder on a wall-clock
//! interval, this module gives the multi-agent delegation usage hint the
//! *capability* to re-inject within a single long turn, on a cadence measured
//! in model requests rather than seconds.
//!
//! IMPORTANT: the default cadence is [`UsageHintCadence::InitialContext`], which
//! is a behavioral no-op here -- under the default the gate never fires from the
//! per-request turn loop, so the usage hint continues to be surfaced only by the
//! existing initial-context / compaction / plan-entry paths exactly as before.
//! Flipping the default to a re-injecting cadence is a separate, later decision.

use codex_protocol::error::Result as CodexResult;

use super::session::Session;
use super::turn_context::TurnContext;
use crate::config::UsageHintCadence;

/// Counter-based cadence state for the delegation usage hint.
///
/// Mirrors the shape of [`super::time_reminder::CurrentTimeReminderState`] but
/// tracks model requests since the last delivery instead of a wall-clock
/// timestamp. `last_window_id` matches the `Option<String>` window-id type used
/// by the time reminder so a new auto-compaction window resets the cadence.
#[derive(Default)]
pub(crate) struct UsageHintReminderState {
    model_requests_since_delivery: u64,
    last_window_id: Option<String>,
}

impl UsageHintReminderState {
    /// Decide whether the usage hint is due on this model request for the given
    /// cadence, advancing the per-request counter. Mirrors
    /// [`super::time_reminder::CurrentTimeReminderState::take_reminder_due`].
    ///
    /// Returns `true` only for the re-injecting cadences (`EveryN`, `Always`).
    /// `InitialContext` and `Plan` are no-ops in the per-request turn loop:
    /// they preserve today's behavior where the hint is delivered only by the
    /// initial-context / plan-entry paths, never re-injected mid-turn.
    fn take_reminder_due(
        &mut self,
        window_id: &str,
        cadence: UsageHintCadence,
        reminder_interval: u64,
    ) -> bool {
        // A new window (e.g. after auto-compaction) restarts the cadence so the
        // first request in the new window is counted from zero.
        if self.last_window_id.as_deref() != Some(window_id) {
            self.model_requests_since_delivery = 0;
            self.last_window_id = Some(window_id.to_string());
        }

        let reminder_is_due = match cadence {
            // No-op cadences: the per-request loop never surfaces the hint; the
            // existing initial-context / plan-entry paths remain the only source.
            UsageHintCadence::InitialContext | UsageHintCadence::Plan => false,
            // Surface on every model request.
            UsageHintCadence::Always => true,
            // Surface once every `reminder_interval` model requests. An interval
            // of 0 is treated as 1 to avoid a divide-by-zero / never-fires trap.
            UsageHintCadence::EveryN => {
                let interval = reminder_interval.max(1);
                self.model_requests_since_delivery += 1;
                if self.model_requests_since_delivery >= interval {
                    self.model_requests_since_delivery = 0;
                    true
                } else {
                    false
                }
            }
        };

        reminder_is_due
    }

    /// Undo the counter advance performed by a `true` result from
    /// [`Self::take_reminder_due`] when the hint turned out to be unavailable
    /// (the fetch returned `None`), so the cadence is treated as *not*
    /// delivered. For the `EveryN` cadence `take_reminder_due` zeroes the
    /// counter on the firing request; rewinding it to `interval - 1` makes the
    /// next model request re-check (and fire as soon as a hint is available)
    /// instead of waiting another full interval. `last_window_id` is left as-is
    /// because the window has not changed. Harmless for `Always` (which ignores
    /// the counter).
    fn rewind_after_undelivered(&mut self, reminder_interval: u64) {
        let interval = reminder_interval.max(1);
        self.model_requests_since_delivery = interval.saturating_sub(1);
    }
}

/// Per-model-request hook mirroring
/// [`super::time_reminder::maybe_record_current_time_reminder`]. Called from the
/// turn loop immediately after the time-reminder hook.
///
/// Under the default cadence ([`UsageHintCadence::InitialContext`]) this is a
/// no-op: the gate returns `false` and nothing is recorded.
pub(super) async fn maybe_record_usage_hint_reminder(
    sess: &Session,
    turn_context: &TurnContext,
    window_id: &str,
) -> CodexResult<()> {
    let multi_agent_v2 = &turn_context.config.multi_agent_v2;

    // Respect the same master switch that gates the usage hint everywhere else;
    // if delegation hints are disabled there is nothing to re-inject.
    if !multi_agent_v2.usage_hint_enabled {
        return Ok(());
    }

    let cadence = multi_agent_v2.usage_hint_cadence;
    let reminder_interval = multi_agent_v2.usage_hint_reminder_interval;

    let reminder_is_due = {
        let mut state = sess.state.lock().await;
        state
            .usage_hint_reminder
            .take_reminder_due(window_id, cadence, reminder_interval)
    };
    if !reminder_is_due {
        return Ok(());
    }

    // Fetch the configured root/subagent usage hint exactly the way the
    // initial-context site does (`multi_agents::usage_hint_text(turn_context,
    // &turn_context.session_source)`). That accessor re-applies the V2 +
    // `usage_hint_enabled` gating and the `plan_token_economy_delegation_k`
    // fallback to the policy defaults, and returns `None` when the hint is
    // disabled / empty for this source (e.g. internal sessions). When it is
    // `None` there is nothing to re-inject, so record nothing and treat this as
    // not-delivered: rewind the cadence counter so the next model request
    // re-checks immediately instead of waiting another full interval.
    let Some(usage_hint_text) =
        super::multi_agents::usage_hint_text(turn_context, &turn_context.session_source)
    else {
        let mut state = sess.state.lock().await;
        state
            .usage_hint_reminder
            .rewind_after_undelivered(reminder_interval);
        return Ok(());
    };

    // Build the same developer-update item the existing initial-context /
    // plan-entry sites build for the usage hint, then record it for the next
    // model request via the same `record_conversation_items` call that
    // `maybe_record_current_time_reminder` uses for `CurrentTimeReminder`. The
    // cadence state was already advanced (counter reset + `last_window_id` set)
    // by `take_reminder_due` above, so on a successful delivery no further state
    // update is needed and the hint fires again after `reminder_interval`
    // model requests.
    let usage_hint_message = if multi_agent_v2.inject_delegation_as_user() {
        crate::context_manager::updates::build_contextual_user_message(vec![usage_hint_text])
    } else {
        crate::context_manager::updates::build_developer_update_item(vec![usage_hint_text])
    };
    if let Some(usage_hint_message) = usage_hint_message {
        sess.record_conversation_items(turn_context, std::slice::from_ref(&usage_hint_message))
            .await;
    }

    Ok(())
}
