mod git_evidence;

use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use git_evidence::GitReviewAnchor;
pub use git_evidence::ReviewAnchor;
pub use git_evidence::ReviewWorkSlice;

const SELF_REVIEW_COOLDOWN: Duration = Duration::from_secs(10 * 60);
const MAX_RECORDED_COMMANDS: usize = 6;
const MAX_RECORDED_PATHS: usize = 12;
const MAX_NOTE_CHARS: usize = 180;

pub const PLAN_UPDATED_MESSAGE: &str = "Plan updated";
pub const SELF_REVIEW_CHECKPOINT_MESSAGE: &str = "\
Plan updated

Self-review checkpoint before continuing: actively review the plan as if the user had asked \"review and improve the plan\". First compare the plan to the user's prompt and confirm it actually plans the requested work. Then check task order, missing verification, assumptions that materially affect value/correctness/integration, stale context, user constraints, and user/remote overlap. Revise the plan first if any issue is found.

<prototype_first_policy>
For feature, tool/runtime, memory, agent, DAB, cache, prompt/context-reducer, or expensive-verification work, first consider whether a focused demo, canary, fixture, or lab script would make the highest-value path cheaper, more observable, or easier to verify before changing the main path. Use the prototype to enable the right architectural move when it is worthwhile; skip it for trivial edits, exact narrow fixes, or when a direct targeted test is cheaper.
</prototype_first_policy>

<coherent_repair_policy>
When self-review finds a concrete, repo-controlled caveat that is directly fixable and verifiable, prefer one coherent repair pass before proceeding. Stop instead of repairing when the blocker is external, destructive, or not reproducible from current evidence.
</coherent_repair_policy>";

#[derive(Debug, Default)]
pub struct SelfReviewTracker {
    command_count: usize,
    patch_count: usize,
    plan_update_count: usize,
    recent_commands: Vec<String>,
    changed_paths: Vec<String>,
    saw_review_this_turn: bool,
    suppress_current_turn: bool,
    suppress_next_turn: bool,
    last_auto_review_started_at: Option<Instant>,
    cwd: Option<PathBuf>,
    git_anchor: Option<GitReviewAnchor>,
}

impl SelfReviewTracker {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        let cwd = cwd.into();
        let mut tracker = Self::default();
        tracker.git_anchor = Some(GitReviewAnchor::capture(cwd.clone()));
        tracker.cwd = Some(cwd);
        tracker
    }

    pub fn set_cwd(&mut self, cwd: impl Into<PathBuf>) {
        let cwd = cwd.into();
        if self.cwd.as_ref() == Some(&cwd) && self.git_anchor.is_some() {
            return;
        }
        self.cwd = Some(cwd);
        self.refresh_review_anchor();
    }

    pub fn refresh_review_anchor_at_cwd(&mut self, cwd: impl Into<PathBuf>) {
        self.cwd = Some(cwd.into());
        self.refresh_review_anchor();
    }

    pub fn refresh_review_anchor(&mut self) {
        if let Some(anchor) = self.git_anchor.take() {
            anchor.cleanup();
        }
        self.git_anchor = self
            .cwd
            .as_ref()
            .map(|cwd| GitReviewAnchor::capture(cwd.clone()));
    }

    pub fn reset_turn(&mut self) {
        let last_auto_review_started_at = self.last_auto_review_started_at;
        let suppress_current_turn = self.suppress_next_turn;
        let cwd = self.cwd.clone();
        let git_anchor = self.git_anchor.take();
        let mut reset = Self::default();
        reset.suppress_current_turn = suppress_current_turn;
        reset.last_auto_review_started_at = last_auto_review_started_at;
        reset.cwd = cwd;
        reset.git_anchor = git_anchor;
        *self = reset;
    }

    pub fn note_plan_update(&mut self) {
        self.plan_update_count += 1;
    }

    pub fn note_command(&mut self, command: impl Into<String>) {
        self.command_count += 1;
        push_recent(
            &mut self.recent_commands,
            command.into(),
            MAX_RECORDED_COMMANDS,
        );
    }

    pub fn note_patch(&mut self, paths: impl IntoIterator<Item = String>) {
        self.patch_count += 1;
        for path in paths {
            push_recent(&mut self.changed_paths, path, MAX_RECORDED_PATHS);
        }
    }

    pub fn note_explicit_review(&mut self) {
        self.saw_review_this_turn = true;
    }

    pub fn note_automatic_review_started(&mut self, now: Instant) {
        self.saw_review_this_turn = true;
        self.suppress_next_turn = true;
        self.last_auto_review_started_at = Some(now);
    }

    pub fn should_remind(&self, now: Instant) -> bool {
        let in_cooldown = self.last_auto_review_started_at.is_some_and(|started_at| {
            now.saturating_duration_since(started_at) < SELF_REVIEW_COOLDOWN
        });

        !self.saw_review_this_turn
            && !self.suppress_current_turn
            && !in_cooldown
            && self.patch_count > 0
    }

    pub fn reminder_message(&self) -> String {
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

    pub fn review_instructions(&self) -> String {
        if let Some(anchor) = &self.git_anchor {
            return anchor.prompt(&self.compact_work_notes());
        }

        format!(
            "\
Automatic self-review of the just-completed work slice.

Do a focused review as if the user asked: review your last actions since the previous explicit or automatic review and improve if needed.

Ground the review in repository state, not full conversation history:
- Start with `git status --short`.
- If there are uncommitted changes, inspect `git diff --stat` and then targeted `git diff -- <path>` for relevant files.
- If the tree is clean or the notes indicate committed work, inspect the relevant commit with `git show --stat --oneline HEAD` and targeted `git show HEAD -- <path>`.
- Use the compact work notes below only as orientation; they are intentionally concise so this still works after compaction.
- Check correctness, regressions, user constraints, missing tests, and whether verification is sufficient.
- For feature, tool/runtime, memory, agent, DAB, cache, prompt/context-reducer, or expensive-verification work, check whether a focused demo, canary, fixture, or lab script would have made the highest-value path cheaper, more observable, or easier to verify; skip this for trivial edits, exact narrow fixes, or when a direct targeted test is cheaper.
- If the review finds a concrete repo-controlled caveat, apply one coherent repair pass and rerun the most relevant targeted verification before finalizing.
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

impl Drop for SelfReviewTracker {
    fn drop(&mut self) {
        if let Some(anchor) = &self.git_anchor {
            anchor.cleanup();
        }
    }
}

pub fn plan_tool_response(include_self_review_checkpoint: bool) -> String {
    if include_self_review_checkpoint {
        SELF_REVIEW_CHECKPOINT_MESSAGE.to_string()
    } else {
        PLAN_UPDATED_MESSAGE.to_string()
    }
}

pub fn is_plan_review_candidate(
    plan_len: usize,
    explanation: Option<&str>,
    has_completed_step: bool,
) -> bool {
    let has_substance =
        plan_len >= 2 || explanation.is_some_and(|explanation| !explanation.trim().is_empty());
    has_substance && !has_completed_step
}

pub fn plan_self_review_prompt(plan_markdown: &str) -> String {
    format!(
        "\
Self-review the plan below before implementation.

Read through the plan against the current conversation and repository context. Start by comparing the plan to the user's prompt: identify the requested outcome, required constraints, and important details, then verify the plan actually covers them without drifting into adjacent work. Use targeted file reads or searches only if the context is insufficient. Improve task order, missing verification, assumptions that materially affect value/correctness/integration, stale context, user constraints, and whether feature/tool/runtime/context work should start with a focused demo, canary, fixture, or lab script. Do not downgrade a valuable architectural fix to a safest-looking patch when ownership evidence supports the larger coherent slice. Keep the result practical and implementation-ready.

Return the revised plan as the next proposed plan. If the plan is already strong, keep it and add only the minimal clarifications needed.

Current plan:
{plan_markdown}"
    )
}

pub fn plan_completion_followup_prompt(completed_plan_markdown: Option<&str>) -> String {
    let completed_plan_section = completed_plan_markdown
        .map(str::trim)
        .filter(|plan| !plan.is_empty())
        .map(|plan| format!("\nCompleted plan:\n{plan}"))
        .unwrap_or_default();

    format!(
        "\
The current plan appears complete.

Review the completed work against the user's request and repository state. Decide whether the work is genuinely done or whether a follow-up planning iteration is needed. Follow-up may happen only after the current plan is complete; this is that completion checkpoint.

If a self-review just ran, first account for its findings and any actions already taken or still needed. Do not open unrelated follow-up scope until review findings are resolved or represented in the next plan.

If another iteration is needed, return the next proposed plan. That next plan should go through the normal cycle: first proposed plan, plan self-review, revised plan if needed, then coherent worker/subagent execution and supervision. If no follow-up is needed, say that directly and summarize the final readiness/verification state.{completed_plan_section}"
    )
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
    use pretty_assertions::assert_eq;

    use super::*;

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
        tracker.note_plan_update();
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
    fn review_prompt_includes_compact_work_notes() {
        let mut tracker = SelfReviewTracker::default();
        tracker.note_plan_update();
        tracker.note_patch(vec!["src/lib.rs".to_string()]);
        tracker.note_command("git diff --stat");

        let instructions = tracker.review_instructions();

        assert!(instructions.contains("git status --short"));
        assert!(instructions.contains("git diff -- <path>"));
        assert!(instructions.contains("git show --stat --oneline HEAD"));
        assert!(instructions.contains("changed paths: src/lib.rs"));
        assert!(instructions.contains("recent commands: git diff --stat"));
    }

    #[test]
    fn review_prompt_includes_prototype_first_policy() {
        let tracker = SelfReviewTracker::default();
        let instructions = tracker.review_instructions();

        assert!(instructions.contains("focused demo, canary, fixture, or lab script"));
        assert!(instructions.contains("prompt/context-reducer"));
        assert!(instructions.contains("direct targeted test is cheaper"));
    }

    #[test]
    fn trivial_plan_keeps_compact_output() {
        assert!(!is_plan_review_candidate(1, None, false));
        assert_eq!(plan_tool_response(false), PLAN_UPDATED_MESSAGE);
    }

    #[test]
    fn nontrivial_plan_includes_self_review_checkpoint() {
        assert!(is_plan_review_candidate(2, None, false));
        assert_eq!(plan_tool_response(true), SELF_REVIEW_CHECKPOINT_MESSAGE);
        assert_eq!(plan_tool_response(false), PLAN_UPDATED_MESSAGE);
        assert!(SELF_REVIEW_CHECKPOINT_MESSAGE.contains("<prototype_first_policy>"));
        assert!(SELF_REVIEW_CHECKPOINT_MESSAGE.contains("<coherent_repair_policy>"));
    }

    #[test]
    fn explanation_makes_plan_nontrivial() {
        assert!(is_plan_review_candidate(
            1,
            Some("Need to sequence work carefully."),
            false
        ));
    }

    #[test]
    fn completed_status_update_does_not_request_plan_review() {
        assert!(!is_plan_review_candidate(2, None, true));
    }

    #[test]
    fn plan_self_review_prompt_contains_current_plan() {
        let prompt = plan_self_review_prompt("# Plan\n- inspect");

        assert!(prompt.contains("Self-review the plan"));
        assert!(prompt.contains("comparing the plan to the user's prompt"));
        assert!(prompt.contains("without drifting into adjacent work"));
        assert!(prompt.contains("focused demo, canary, fixture, or lab script"));
        assert!(prompt.contains("safest-looking patch"));
        assert!(prompt.contains("# Plan\n- inspect"));
    }

    #[test]
    fn plan_completion_followup_prompt_contains_completed_plan_and_cycle() {
        let prompt = plan_completion_followup_prompt(Some("- completed: Inspect"));

        assert!(prompt.contains("The current plan appears complete."));
        assert!(prompt.contains("follow-up planning iteration"));
        assert!(prompt.contains("first account for its findings"));
        assert!(prompt.contains("first proposed plan, plan self-review"));
        assert!(prompt.contains("coherent worker/subagent execution and supervision"));
        assert!(prompt.contains("- completed: Inspect"));
    }
}
