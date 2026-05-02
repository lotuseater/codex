use codex_protocol::plan_tool::UpdatePlanArgs;
use std::time::Duration;
use std::time::Instant;

const COMMAND_REMINDER_THRESHOLD: usize = 3;
const SELF_REVIEW_COOLDOWN: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Default, Clone)]
pub(super) struct SelfReviewTracker {
    command_count: usize,
    patch_count: usize,
    plan_update_count: usize,
    saw_review_this_turn: bool,
    suppress_current_turn: bool,
    suppress_next_turn: bool,
    last_auto_review_started_at: Option<Instant>,
}

impl SelfReviewTracker {
    pub(super) fn reset_turn(&mut self) {
        let last_auto_review_started_at = self.last_auto_review_started_at;
        let suppress_current_turn = self.suppress_next_turn;
        *self = Self {
            suppress_current_turn,
            last_auto_review_started_at,
            ..Self::default()
        };
    }

    pub(super) fn note_plan_update(&mut self, update: &UpdatePlanArgs) {
        if !update.plan.is_empty() {
            self.plan_update_count += 1;
        }
    }

    pub(super) fn note_command(&mut self) {
        self.command_count += 1;
    }

    pub(super) fn note_patch(&mut self) {
        self.patch_count += 1;
    }

    pub(super) fn note_explicit_review(&mut self) {
        self.saw_review_this_turn = true;
    }

    pub(super) fn note_automatic_review_started(&mut self, now: Instant) {
        self.saw_review_this_turn = true;
        self.suppress_next_turn = true;
        self.last_auto_review_started_at = Some(now);
    }

    pub(super) fn should_remind(&self, now: Instant) -> bool {
        let in_cooldown = self.last_auto_review_started_at.is_some_and(|started_at| {
            now.saturating_duration_since(started_at) < SELF_REVIEW_COOLDOWN
        });

        !self.saw_review_this_turn
            && !self.suppress_current_turn
            && !in_cooldown
            && (self.patch_count > 0
                || self.command_count >= COMMAND_REMINDER_THRESHOLD
                || (self.command_count > 0 && self.plan_update_count > 0))
    }

    pub(super) fn reminder_message(&self) -> String {
        let work = match (self.patch_count, self.command_count) {
            (patches, commands) if patches > 0 && commands > 0 => {
                format!("{patches} file-change step(s) and {commands} command(s)")
            }
            (patches, _) if patches > 0 => format!("{patches} file-change step(s)"),
            (_, commands) => format!("{commands} command(s)"),
        };

        format!(
            "Self-review required: {work} completed without an explicit review. Starting an automatic review of current changes before finalizing."
        )
    }
}

#[cfg(test)]
mod tests {
    use codex_protocol::plan_tool::PlanItemArg;
    use codex_protocol::plan_tool::StepStatus;

    use super::*;

    fn plan_update() -> UpdatePlanArgs {
        UpdatePlanArgs {
            explanation: None,
            plan: vec![PlanItemArg {
                step: "inspect".to_string(),
                status: StepStatus::InProgress,
            }],
        }
    }

    #[test]
    fn patch_activity_triggers_reminder() {
        let mut tracker = SelfReviewTracker::default();
        let now = Instant::now();
        tracker.note_patch();

        assert!(tracker.should_remind(now));
    }

    #[test]
    fn explicit_review_suppresses_reminder() {
        let mut tracker = SelfReviewTracker::default();
        let now = Instant::now();
        tracker.note_patch();
        tracker.note_explicit_review();

        assert!(!tracker.should_remind(now));
    }

    #[test]
    fn command_with_plan_update_triggers_reminder() {
        let mut tracker = SelfReviewTracker::default();
        let now = Instant::now();
        tracker.note_plan_update(&plan_update());
        tracker.note_command();

        assert!(tracker.should_remind(now));
    }

    #[test]
    fn automatic_review_cooldown_suppresses_follow_up_reminders() {
        let mut tracker = SelfReviewTracker::default();
        let started_at = Instant::now();
        tracker.note_automatic_review_started(started_at);
        tracker.reset_turn();
        tracker.reset_turn();
        tracker.note_patch();

        assert!(!tracker.should_remind(started_at + SELF_REVIEW_COOLDOWN - Duration::from_secs(1)));
        assert!(tracker.should_remind(started_at + SELF_REVIEW_COOLDOWN));
    }

    #[test]
    fn automatic_review_turn_does_not_trigger_recursive_review() {
        let mut tracker = SelfReviewTracker::default();
        let now = Instant::now();
        tracker.note_patch();

        assert!(tracker.should_remind(now));

        tracker.note_automatic_review_started(now);
        tracker.reset_turn();
        tracker.note_command();
        tracker.note_command();
        tracker.note_command();

        assert!(!tracker.should_remind(now + SELF_REVIEW_COOLDOWN));
    }
}
