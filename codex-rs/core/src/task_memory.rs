use std::time::Duration;
use std::time::Instant;

use codex_protocol::items::TurnItem;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::plan_tool::StepStatus;
use codex_protocol::plan_tool::UpdatePlanArgs;
use codex_utils_cache::sha1_digest;
use codex_utils_output_truncation::TruncationPolicy;
use codex_utils_output_truncation::approx_token_count;
use codex_utils_output_truncation::approx_tokens_from_byte_count_i64;
use codex_utils_output_truncation::truncate_text;
use codex_utils_stream_parser::extract_proposed_plan_text;
use codex_utils_stream_parser::strip_citations;

use crate::compact::SUMMARY_PREFIX;
use crate::context::ContextualUserFragment;
use crate::context::TaskMemory;
use crate::context_manager::estimate_response_item_model_visible_bytes;
use crate::event_mapping::parse_turn_item;

const TOTAL_TOKEN_BUDGET: usize = 2_500;
const PLAN_TOKEN_BUDGET: usize = 1_900;
const REQUEST_TOKEN_BUDGET: usize = 600;
const PRESSURE_TOKEN_THRESHOLD: i64 = 64_000;
const MAX_SAME_DIGEST_PRESSURE_INJECTIONS: u8 = 2;
const MIN_USER_MESSAGES_BETWEEN_INJECTIONS: usize = 3;
const MIN_PRESSURE_INJECTION_INTERVAL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuiltTaskMemory {
    digest: String,
    body: String,
}

impl BuiltTaskMemory {
    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }

    pub(crate) fn into_response_item(self) -> ResponseItem {
        ContextualUserFragment::into(TaskMemory::new(self.body))
    }
}

#[derive(Debug, Default)]
pub(crate) struct TaskMemoryThrottleState {
    last_digest: Option<String>,
    same_digest_injections: u8,
    last_injected_at: Option<Instant>,
    last_injected_user_message_count: usize,
}

impl TaskMemoryThrottleState {
    pub(crate) fn should_inject(
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

    pub(crate) fn reset_after_compaction(&mut self, digest: Option<&str>) {
        self.last_digest = digest.map(str::to_string);
        self.same_digest_injections = 0;
        self.last_injected_at = None;
        self.last_injected_user_message_count = 0;
    }
}

pub(crate) fn build_task_memory(items: &[ResponseItem]) -> Option<BuiltTaskMemory> {
    let latest_plan = latest_plan(items);
    let latest_plan_index = latest_plan.as_ref().map(|plan| plan.index);
    let user_messages = substantive_user_messages(items);

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

pub(crate) fn build_task_memory_item(items: &[ResponseItem]) -> Option<ResponseItem> {
    build_task_memory(items).map(BuiltTaskMemory::into_response_item)
}

pub(crate) fn task_memory_item_digest(item: &ResponseItem) -> Option<String> {
    let text = response_item_text(item)?;
    if !is_task_memory_text(&text) {
        return None;
    }
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("Digest: ").map(str::to_string))
}

pub(crate) fn contains_task_memory_item(items: &[ResponseItem]) -> bool {
    items.iter().any(is_task_memory_item)
}

pub(crate) fn find_task_memory_digest(items: &[ResponseItem]) -> Option<String> {
    items.iter().find_map(task_memory_item_digest)
}

pub(crate) fn remove_task_memory_items(items: &mut Vec<ResponseItem>) {
    items.retain(|item| !is_task_memory_item(item));
}

pub(crate) fn real_user_message_count(items: &[ResponseItem]) -> usize {
    items
        .iter()
        .filter(|item| real_user_message(item).is_some())
        .count()
}

pub(crate) fn estimated_prompt_tokens(items: &[ResponseItem]) -> i64 {
    let bytes = items
        .iter()
        .map(estimate_response_item_model_visible_bytes)
        .fold(0i64, i64::saturating_add);
    approx_tokens_from_byte_count_i64(bytes)
}

pub(crate) fn should_inject_under_pressure(estimated_tokens: i64, auto_compact_limit: i64) -> bool {
    estimated_tokens >= pressure_threshold(auto_compact_limit)
}

fn pressure_threshold(auto_compact_limit: i64) -> i64 {
    PRESSURE_TOKEN_THRESHOLD.min(auto_compact_limit.saturating_div(3))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexedText {
    index: usize,
    text: String,
}

fn latest_plan(items: &[ResponseItem]) -> Option<IndexedText> {
    let mut latest = None;
    for (index, item) in items.iter().enumerate() {
        if is_task_memory_item(item) {
            continue;
        }
        if let Some(text) = proposed_plan_text(item).or_else(|| update_plan_text(item)) {
            latest = Some(IndexedText { index, text });
        }
    }
    latest
}

fn proposed_plan_text(item: &ResponseItem) -> Option<String> {
    let ResponseItem::Message { role, content, .. } = item else {
        return None;
    };
    if role != "assistant" {
        return None;
    }
    let text = content_text(content);
    let plan = extract_proposed_plan_text(&text)?;
    let (plan, _citations) = strip_citations(&plan);
    Some(plan.trim().to_string()).filter(|plan| !plan.is_empty())
}

fn update_plan_text(item: &ResponseItem) -> Option<String> {
    let ResponseItem::FunctionCall {
        name, arguments, ..
    } = item
    else {
        return None;
    };
    if name != "update_plan" {
        return None;
    }
    let args = serde_json::from_str::<UpdatePlanArgs>(arguments).ok()?;
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

fn step_status_label(status: &StepStatus) -> &'static str {
    match status {
        StepStatus::Pending => "pending",
        StepStatus::InProgress => "in_progress",
        StepStatus::Completed => "completed",
    }
}

fn substantive_user_messages(items: &[ResponseItem]) -> Vec<IndexedText> {
    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let text = real_user_message(item)?;
            (!is_continuation_only(&text)).then_some(IndexedText { index, text })
        })
        .collect()
}

fn real_user_message(item: &ResponseItem) -> Option<String> {
    let Some(TurnItem::UserMessage(user)) = parse_turn_item(item) else {
        return None;
    };
    let message = user.message();
    let trimmed = message.trim();
    if trimmed.is_empty() || trimmed.starts_with(SUMMARY_PREFIX) {
        None
    } else {
        Some(message)
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

fn response_item_text(item: &ResponseItem) -> Option<String> {
    let ResponseItem::Message { content, .. } = item else {
        return None;
    };
    Some(content_text(content)).filter(|text| !text.is_empty())
}

fn is_task_memory_item(item: &ResponseItem) -> bool {
    response_item_text(item).is_some_and(|text| is_task_memory_text(&text))
}

fn is_task_memory_text(text: &str) -> bool {
    <TaskMemory as ContextualUserFragment>::matches_text(text)
}

fn content_text(content: &[ContentItem]) -> String {
    content
        .iter()
        .filter_map(|item| match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                Some(text.as_str())
            }
            ContentItem::InputImage { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
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

#[cfg(test)]
#[path = "task_memory_tests.rs"]
mod tests;
