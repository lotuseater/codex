use codex_protocol::plan_tool::UpdatePlanArgs;

const COMMAND_REMINDER_THRESHOLD: usize = 3;

#[derive(Debug, Default, Clone)]
pub(super) struct SelfReviewTracker {
    command_count: usize,
    patch_count: usize,
    plan_update_count: usize,
    saw_explicit_review: bool,
}

impl SelfReviewTracker {
    pub(super) fn reset_turn(&mut self) {
        *self = Self::default();
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
        self.saw_explicit_review = true;
    }

    pub(super) fn should_remind(&self) -> bool {
        !self.saw_explicit_review
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
            "Self-review reminder: {work} completed without an explicit review. Inspect diff, tests, docs, and user intent before finalizing."
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
        tracker.note_patch();

        assert!(tracker.should_remind());
    }

    #[test]
    fn explicit_review_suppresses_reminder() {
        let mut tracker = SelfReviewTracker::default();
        tracker.note_patch();
        tracker.note_explicit_review();

        assert!(!tracker.should_remind());
    }

    #[test]
    fn command_with_plan_update_triggers_reminder() {
        let mut tracker = SelfReviewTracker::default();
        tracker.note_plan_update(&plan_update());
        tracker.note_command();

        assert!(tracker.should_remind());
    }
}
