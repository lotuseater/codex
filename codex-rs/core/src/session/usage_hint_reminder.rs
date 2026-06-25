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

    // TODO(usage-hint-cadence): wire the actual hint text in here. The text is
    // built by `super::multi_agents::usage_hint_text(turn_context,
    // &turn_context.session_source)` (which itself falls back to
    // `codex_agent_policy::default_multi_agent_v2_root_usage_hint_text_with_k` /
    // `..._subagent_..._with_k`). Once wired, record it as a contextual user
    // fragment via `sess.record_conversation_items(...)`, mirroring how
    // `maybe_record_current_time_reminder` records `CurrentTimeReminder`.
    //
    // The cadence STATE + gate above is the deliverable for this change; the
    // text-injection seam is intentionally left as a marked TODO so the default
    // (InitialContext) remains a verified behavioral no-op and the build stays
    // green without taking a position on the exact re-injection payload.
    let _ = (sess, turn_context);

    Ok(())
}
