use std::time::Duration;
use std::time::Instant;

use codex_utils_cache::sha1_digest;
use codex_utils_output_truncation::TruncationPolicy;
use codex_utils_output_truncation::approx_token_count;
use codex_utils_output_truncation::truncate_text;
use codex_utils_stream_parser::extract_proposed_plan_text;
use codex_utils_stream_parser::strip_citations;
use serde::Deserialize;

pub const TASK_MEMORY_START_MARKER: &str = "<task_memory>";
pub const TASK_MEMORY_END_MARKER: &str = "</task_memory>";

const TOTAL_TOKEN_BUDGET: usize = 2_500;
const PLAN_TOKEN_BUDGET: usize = 1_900;
const REQUEST_TOKEN_BUDGET: usize = 600;
const PRESSURE_TOKEN_THRESHOLD: i64 = 64_000;
const MAX_SAME_DIGEST_PRESSURE_INJECTIONS: u8 = 2;
const MIN_USER_MESSAGES_BETWEEN_INJECTIONS: usize = 3;
const MIN_PRESSURE_INJECTION_INTERVAL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskMemoryInputItem {
    UserMessage(String),
    AssistantMessage(String),
    UpdatePlanCall(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltTaskMemory {
    digest: String,
    body: String,
}

impl BuiltTaskMemory {
    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn render(self) -> String {
        render_task_memory_body(&self.body)
    }
}

#[derive(Debug, Default)]
pub struct TaskMemoryThrottleState {
    last_digest: Option<String>,
    same_digest_injections: u8,
    last_injected_at: Option<Instant>,
    last_injected_user_message_count: usize,
}

impl TaskMemoryThrottleState {
    pub fn should_inject(
        &mut self,
        digest: &str,
        real_user_message_count: usize,
        now: Instant,
    ) -> bool {
        if self.last_digest.as_deref() != Some(digest) {
            self.last_digest = Some(digest.to_string());
            self.same_digest_injections = 1;
            self.last_injected_at = Some(now);
            self.last_injected_user_message_count = real_user_message_count;
            return true;
        }

        if self.same_digest_injections >= MAX_SAME_DIGEST_PRESSURE_INJECTIONS {
            return false;
        }

        let enough_new_user_messages = real_user_message_count
            .saturating_sub(self.last_injected_user_message_count)
            >= MIN_USER_MESSAGES_BETWEEN_INJECTIONS;
        let enough_time = self
            .last_injected_at
            .is_none_or(|last| now.duration_since(last) >= MIN_PRESSURE_INJECTION_INTERVAL);
        if !enough_new_user_messages && !enough_time {
            return false;
        }

        self.same_digest_injections = self.same_digest_injections.saturating_add(1);
        self.last_injected_at = Some(now);
        self.last_injected_user_message_count = real_user_message_count;
        true
    }

    pub fn reset_after_compaction(&mut self, digest: Option<&str>) {
        self.last_digest = digest.map(str::to_string);
        self.same_digest_injections = 0;
        self.last_injected_at = None;
        self.last_injected_user_message_count = 0;
    }
}

pub fn build_task_memory_with_summary_prefix(
    items: &[TaskMemoryInputItem],
    summary_prefix: &str,
) -> Option<BuiltTaskMemory> {
    let latest_plan = latest_plan(items);
    let latest_plan_index = latest_plan.as_ref().map(|plan| plan.index);
    let user_messages = substantive_user_messages(items, summary_prefix);

    let active_prompt = if let Some(plan_index) = latest_plan_index {
        user_messages
            .iter()
            .rev()
            .find(|message| message.index < plan_index)
            .map(|message| message.text.clone())
    } else {
        user_messages.last().map(|message| message.text.clone())
    };

    let directives = latest_plan_index.map_or_else(Vec::new, |plan_index| {
        user_messages
            .iter()
            .filter(|message| message.index > plan_index)
            .map(|message| message.text.clone())
            .collect()
    });

    if active_prompt.is_none() && latest_plan.is_none() && directives.is_empty() {
        return None;
    }

    let mut request_parts = Vec::new();
    if let Some(prompt) = active_prompt {
        request_parts.push(format!("Active user request:\n{}", prompt.trim()));
    }
    if !directives.is_empty() {
        request_parts.push(format!(
            "Later user directives:\n{}",
            directives
                .iter()
                .map(|directive| format!("- {}", directive.trim()))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    let request_text = budget_text(&request_parts.join("\n\n"), REQUEST_TOKEN_BUDGET);
    let plan_text = latest_plan.map(|plan| budget_text(&plan.text, PLAN_TOKEN_BUDGET));

    let mut body = String::from("# Task Memory\n");
    body.push_str("Policy: near-verbatim active task and plan memory.\n");
    if !request_text.trim().is_empty() {
        body.push_str("\n## User Prompt And Directives\n");
        body.push_str(request_text.trim());
        body.push('\n');
    }
    if let Some(plan_text) = plan_text
        && !plan_text.trim().is_empty()
    {
        body.push_str("\n## Current Plan\n");
        body.push_str(plan_text.trim());
        body.push('\n');
    }
    body = budget_text(&body, TOTAL_TOKEN_BUDGET);

    let digest = digest_for_body(&body);
    let body = format!("Digest: {digest}\n\n{body}");
    Some(BuiltTaskMemory { digest, body })
}

pub fn task_memory_text_digest(text: &str) -> Option<String> {
    if !is_task_memory_text(text) {
        return None;
    }
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("Digest: ").map(str::to_string))
}

pub fn real_user_message_count(items: &[TaskMemoryInputItem], summary_prefix: &str) -> usize {
    items
        .iter()
        .filter(|item| real_user_message(item, summary_prefix).is_some())
        .count()
}

pub fn should_inject_under_pressure(estimated_tokens: i64, auto_compact_limit: i64) -> bool {
    estimated_tokens >= pressure_threshold(auto_compact_limit)
}

pub fn is_task_memory_text(text: &str) -> bool {
    let trimmed_start = text.trim_start();
    let starts_with_marker = trimmed_start
        .get(..TASK_MEMORY_START_MARKER.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(TASK_MEMORY_START_MARKER));
    let trimmed_end = text.trim_end();
    let ends_with_marker = trimmed_end
        .get(
            trimmed_end
                .len()
                .saturating_sub(TASK_MEMORY_END_MARKER.len())..,
        )
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(TASK_MEMORY_END_MARKER));
    starts_with_marker && ends_with_marker
}

fn pressure_threshold(auto_compact_limit: i64) -> i64 {
    PRESSURE_TOKEN_THRESHOLD.min(auto_compact_limit.saturating_div(3))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexedText {
    index: usize,
    text: String,
}

fn latest_plan(items: &[TaskMemoryInputItem]) -> Option<IndexedText> {
    let mut latest = None;
    for (index, item) in items.iter().enumerate() {
        if let Some(text) = proposed_plan_text(item).or_else(|| update_plan_text(item)) {
            latest = Some(IndexedText { index, text });
        }
    }
    latest
}

fn proposed_plan_text(item: &TaskMemoryInputItem) -> Option<String> {
    let TaskMemoryInputItem::AssistantMessage(text) = item else {
        return None;
    };
    let plan = extract_proposed_plan_text(text)?;
    let (plan, _citations) = strip_citations(&plan);
    Some(plan.trim().to_string()).filter(|plan| !plan.is_empty())
}

#[derive(Deserialize)]
struct UpdatePlanArgsLite {
    explanation: Option<String>,
    plan: Vec<PlanItemLite>,
}

#[derive(Deserialize)]
struct PlanItemLite {
    step: String,
    status: StepStatusLite,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum StepStatusLite {
    Pending,
    InProgress,
    Completed,
}

fn update_plan_text(item: &TaskMemoryInputItem) -> Option<String> {
    let TaskMemoryInputItem::UpdatePlanCall(arguments) = item else {
        return None;
    };
    let args = serde_json::from_str::<UpdatePlanArgsLite>(arguments).ok()?;
    let mut text = String::new();
    if let Some(explanation) = args.explanation.as_deref()
        && !explanation.trim().is_empty()
    {
        text.push_str("Explanation: ");
        text.push_str(explanation.trim());
        text.push_str("\n\n");
    }
    for item in args.plan {
        text.push_str("- [");
        text.push_str(step_status_label(&item.status));
        text.push_str("] ");
        text.push_str(item.step.trim());
        text.push('\n');
    }
    Some(text.trim().to_string()).filter(|text| !text.is_empty())
}

fn step_status_label(status: &StepStatusLite) -> &'static str {
    match status {
        StepStatusLite::Pending => "pending",
        StepStatusLite::InProgress => "in_progress",
        StepStatusLite::Completed => "completed",
    }
}

fn substantive_user_messages(
    items: &[TaskMemoryInputItem],
    summary_prefix: &str,
) -> Vec<IndexedText> {
    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let text = real_user_message(item, summary_prefix)?;
            (!is_continuation_only(&text)).then_some(IndexedText { index, text })
        })
        .collect()
}

fn real_user_message(item: &TaskMemoryInputItem, summary_prefix: &str) -> Option<String> {
    let TaskMemoryInputItem::UserMessage(message) = item else {
        return None;
    };
    let trimmed = message.trim();
    if trimmed.is_empty() || trimmed.starts_with(summary_prefix) || is_task_memory_text(trimmed) {
        None
    } else {
        Some(message.clone())
    }
}

fn is_continuation_only(text: &str) -> bool {
    let normalized = text
        .trim()
        .trim_matches(|ch: char| ch.is_ascii_punctuation() || ch.is_whitespace())
        .to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "go on"
            | "continue"
            | "please continue"
            | "carry on"
            | "resume"
            | "keep going"
            | "do it"
            | "fix it"
            | "finish it"
            | "go ahead"
    )
}

fn budget_text(text: &str, max_tokens: usize) -> String {
    if text.trim().is_empty() || approx_token_count(text) <= max_tokens {
        return text.to_string();
    }
    truncate_text(text, TruncationPolicy::Tokens(max_tokens))
}

fn digest_for_body(body: &str) -> String {
    sha1_digest(body.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn render_task_memory_body(body: &str) -> String {
    format!(
        "{TASK_MEMORY_START_MARKER}\n{}\n{TASK_MEMORY_END_MARKER}",
        body.trim()
    )
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    const SUMMARY_PREFIX: &str = "Another language model started to solve this problem";

    fn user_message(text: &str) -> TaskMemoryInputItem {
        TaskMemoryInputItem::UserMessage(text.to_string())
    }

    fn assistant_message(text: &str) -> TaskMemoryInputItem {
        TaskMemoryInputItem::AssistantMessage(text.to_string())
    }

    fn update_plan_call(arguments: &str) -> TaskMemoryInputItem {
        TaskMemoryInputItem::UpdatePlanCall(arguments.to_string())
    }

    #[test]
    fn builds_near_verbatim_memory_from_latest_proposed_plan() {
        let items = vec![
            user_message("Build the native task memory feature before the final build."),
            assistant_message(
                "<proposed_plan>\n# Plan\n- inspect history\n- patch memory\n</proposed_plan>",
            ),
            user_message("go on"),
            user_message("Also throttle repeated pre-compact injections."),
        ];

        let memory = build_task_memory_with_summary_prefix(&items, SUMMARY_PREFIX).expect("memory");

        assert!(memory.body().contains("Digest: "));
        assert!(
            memory
                .body()
                .contains("Build the native task memory feature before the final build.")
        );
        assert!(memory.body().contains("- inspect history"));
        assert!(
            memory
                .body()
                .contains("Also throttle repeated pre-compact injections.")
        );
        assert!(!memory.body().contains("go on"));
    }

    #[test]
    fn latest_update_plan_is_used_when_it_is_newer_than_proposed_plan() {
        let items = vec![
            user_message("Fix the feature."),
            assistant_message("<proposed_plan>\n# Old Plan\n- old step\n</proposed_plan>"),
            update_plan_call(
                r#"{"explanation":"Need a safer order.","plan":[{"step":"inspect","status":"completed"},{"step":"patch","status":"in_progress"}]}"#,
            ),
        ];

        let memory = build_task_memory_with_summary_prefix(&items, SUMMARY_PREFIX).expect("memory");

        assert!(memory.body().contains("Need a safer order."));
        assert!(memory.body().contains("- [completed] inspect"));
        assert!(memory.body().contains("- [in_progress] patch"));
        assert!(!memory.body().contains("old step"));
    }

    #[test]
    fn task_memory_items_are_not_real_user_messages() {
        let item = build_task_memory_with_summary_prefix(
            &[
                user_message("Keep this task visible."),
                assistant_message("<proposed_plan>\n# Plan\n- keep memory\n</proposed_plan>"),
            ],
            SUMMARY_PREFIX,
        )
        .expect("memory item")
        .render();

        assert!(task_memory_text_digest(&item).is_some());
        assert_eq!(
            real_user_message_count(&[user_message(&item)], SUMMARY_PREFIX),
            0
        );
    }

    #[test]
    fn detects_task_memory_text_even_without_digest() {
        let malformed_memory = "<task_memory>\n# Task Memory\nmissing digest\n</task_memory>";

        assert!(is_task_memory_text(malformed_memory));
        assert_eq!(task_memory_text_digest(malformed_memory), None);
    }

    #[test]
    fn pressure_throttle_limits_repeated_same_digest_injections() {
        let mut state = TaskMemoryThrottleState::default();
        let start = Instant::now();

        assert!(state.should_inject("digest-a", 1, start));
        assert!(!state.should_inject("digest-a", 1, start + Duration::from_secs(60)));
        assert!(state.should_inject("digest-a", 4, start + Duration::from_secs(60)));
        assert!(!state.should_inject("digest-a", 8, start + Duration::from_secs(20 * 60)));
        assert!(state.should_inject("digest-b", 8, start + Duration::from_secs(20 * 60)));
    }

    #[test]
    fn pressure_threshold_uses_one_third_of_auto_compact_limit_capped_at_64k() {
        assert!(!should_inject_under_pressure(9_999, 30_000));
        assert!(should_inject_under_pressure(10_000, 30_000));
        assert!(!should_inject_under_pressure(63_999, 300_000));
        assert!(should_inject_under_pressure(64_000, 300_000));
    }
}
