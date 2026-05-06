use codex_app_server_protocol::ReviewTarget;
use codex_protocol::plan_tool::UpdatePlanArgs;
use std::time::Duration;
use std::time::Instant;

const SELF_REVIEW_COOLDOWN: Duration = Duration::from_secs(10 * 60);
const MAX_RECORDED_COMMANDS: usize = 6;
const MAX_RECORDED_PATHS: usize = 12;
const MAX_NOTE_CHARS: usize = 180;

#[derive(Debug, Default, Clone)]
pub(super) struct SelfReviewTracker {
    command_count: usize,
    patch_count: usize,
    plan_update_count: usize,
    recent_commands: Vec<String>,
    changed_paths: Vec<String>,
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

    pub(super) fn note_plan_update(&mut self, _update: &UpdatePlanArgs) {
        self.plan_update_count += 1;
    }

    pub(super) fn note_command(&mut self, command: impl Into<String>) {
        self.command_count += 1;
        push_recent(
            &mut self.recent_commands,
            command.into(),
            MAX_RECORDED_COMMANDS,
        );
    }

    pub(super) fn note_patch(&mut self, paths: impl IntoIterator<Item = String>) {
        self.patch_count += 1;
        for path in paths {
            push_recent(&mut self.changed_paths, path, MAX_RECORDED_PATHS);
        }
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
            && self.patch_count > 0
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

    pub(super) fn review_target(&self) -> ReviewTarget {
        ReviewTarget::Custom {
            instructions: self.review_instructions(),
        }
    }

    fn review_instructions(&self) -> String {
        format!(
            "\
Automatic self-review of the just-completed work slice.

Do a bounded review as if the user asked: review your last actions since the previous explicit or automatic review and improve if needed.

Ground the review in repository state, not full conversation history:
- Start with `git status --short`.
- If there are uncommitted changes, inspect `git diff --stat` and then targeted `git diff -- <path>` for relevant files.
- If the tree is clean or the notes indicate committed work, inspect the relevant commit with `git show --stat --oneline HEAD` and targeted `git show HEAD -- <path>`.
- Use the compact work notes below only as orientation; they are intentionally small so this still works after compaction.
- Check correctness, regressions, user constraints, missing tests, and whether verification is sufficient.
- Return prioritized review findings. If there are no findings, say that in the review output.

Compact work notes:
{}
",
            self.compact_work_notes()
        )
    }

    fn compact_work_notes(&self) -> String {
        let mut notes = vec![
            format!("file-change steps: {}", self.patch_count),
            format!("commands completed: {}", self.command_count),
            format!("plan updates: {}", self.plan_update_count),
        ];
        if !self.changed_paths.is_empty() {
            notes.push(format!("changed paths: {}", self.changed_paths.join(", ")));
        }
        if !self.recent_commands.is_empty() {
            notes.push(format!(
                "recent commands: {}",
                self.recent_commands.join(" | ")
            ));
        }
        notes
            .into_iter()
            .map(|note| format!("- {note}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn push_recent(items: &mut Vec<String>, value: String, max_items: usize) {
    let value = truncate_note(value.trim());
    if value.is_empty() || items.iter().any(|item| item == &value) {
        return;
    }
    if items.len() >= max_items {
        items.remove(0);
    }
    items.push(value);
}

fn truncate_note(value: &str) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(MAX_NOTE_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
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
        tracker.note_patch(Vec::new());

        assert!(tracker.should_remind(now));
    }

    #[test]
    fn explicit_review_suppresses_reminder() {
        let mut tracker = SelfReviewTracker::default();
        let now = Instant::now();
        tracker.note_patch(Vec::new());
        tracker.note_explicit_review();

        assert!(!tracker.should_remind(now));
    }

    #[test]
    fn command_with_plan_update_does_not_trigger_review_task() {
        let mut tracker = SelfReviewTracker::default();
        let now = Instant::now();
        tracker.note_plan_update(&plan_update());
        tracker.note_command("git status --short");

        assert!(!tracker.should_remind(now));
    }

    #[test]
    fn automatic_review_cooldown_suppresses_follow_up_reminders() {
        let mut tracker = SelfReviewTracker::default();
        let started_at = Instant::now();
        tracker.note_automatic_review_started(started_at);
        tracker.reset_turn();
        tracker.reset_turn();
        tracker.note_patch(Vec::new());

        assert!(!tracker.should_remind(started_at + SELF_REVIEW_COOLDOWN - Duration::from_secs(1)));
        assert!(tracker.should_remind(started_at + SELF_REVIEW_COOLDOWN));
    }

    #[test]
    fn automatic_review_turn_does_not_trigger_recursive_review() {
        let mut tracker = SelfReviewTracker::default();
        let now = Instant::now();
        tracker.note_patch(Vec::new());

        assert!(tracker.should_remind(now));

        tracker.note_automatic_review_started(now);
        tracker.reset_turn();
        tracker.note_command("git status --short");
        tracker.note_command("git diff --stat");
        tracker.note_command("cargo test --release");

        assert!(!tracker.should_remind(now + SELF_REVIEW_COOLDOWN));
    }

    #[test]
    fn review_prompt_includes_bounded_work_notes() {
        let mut tracker = SelfReviewTracker::default();
        tracker.note_plan_update(&plan_update());
        tracker.note_patch(vec!["src/lib.rs".to_string()]);
        tracker.note_command("git diff --stat");

        let ReviewTarget::Custom { instructions } = tracker.review_target() else {
            panic!("automatic self-review should use custom instructions");
        };

        assert!(instructions.contains("git status --short"));
        assert!(instructions.contains("git diff -- <path>"));
        assert!(instructions.contains("git show --stat --oneline HEAD"));
        assert!(instructions.contains("changed paths: src/lib.rs"));
        assert!(instructions.contains("recent commands: git diff --stat"));
    }
}
