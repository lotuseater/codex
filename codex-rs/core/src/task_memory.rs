use codex_protocol::items::TurnItem;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_task_memory::TaskMemoryInputItem;
use codex_utils_output_truncation::approx_tokens_from_byte_count_i64;

use crate::compact::SUMMARY_PREFIX;
use crate::context_manager::estimate_response_item_model_visible_bytes;

pub(crate) use codex_task_memory::TaskMemoryThrottleState;

pub(crate) struct BuiltTaskMemory(codex_task_memory::BuiltTaskMemory);

impl BuiltTaskMemory {
    pub(crate) fn digest(&self) -> &str {
        self.0.digest()
    }

    pub(crate) fn into_response_item(self) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: self.0.render(),
            }],
            phase: None,
        }
    }
}

pub(crate) fn build_task_memory(items: &[ResponseItem]) -> Option<BuiltTaskMemory> {
    let items = task_memory_input_items(items);
    codex_task_memory::build_task_memory_with_summary_prefix(&items, SUMMARY_PREFIX)
        .map(BuiltTaskMemory)
}

pub(crate) fn build_task_memory_item(items: &[ResponseItem]) -> Option<ResponseItem> {
    build_task_memory(items).map(BuiltTaskMemory::into_response_item)
}

pub(crate) fn task_memory_item_digest(item: &ResponseItem) -> Option<String> {
    codex_task_memory::task_memory_text_digest(&response_item_text(item)?)
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
    let items = task_memory_input_items(items);
    codex_task_memory::real_user_message_count(&items, SUMMARY_PREFIX)
}

pub(crate) fn estimated_prompt_tokens(items: &[ResponseItem]) -> i64 {
    let bytes = items
        .iter()
        .map(estimate_response_item_model_visible_bytes)
        .fold(0i64, i64::saturating_add);
    approx_tokens_from_byte_count_i64(bytes)
}

pub(crate) fn should_inject_under_pressure(estimated_tokens: i64, auto_compact_limit: i64) -> bool {
    codex_task_memory::should_inject_under_pressure(estimated_tokens, auto_compact_limit)
}

fn task_memory_input_items(items: &[ResponseItem]) -> Vec<TaskMemoryInputItem> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { role, content, .. } if role == "assistant" => {
                Some(TaskMemoryInputItem::AssistantMessage(content_text(content)))
            }
            ResponseItem::FunctionCall {
                name, arguments, ..
            } if name == "update_plan" => {
                Some(TaskMemoryInputItem::UpdatePlanCall(arguments.clone()))
            }
            _ => match crate::event_mapping::parse_turn_item(item) {
                Some(TurnItem::UserMessage(user)) => {
                    Some(TaskMemoryInputItem::UserMessage(user.message()))
                }
                _ => None,
            },
        })
        .collect()
}

fn is_task_memory_item(item: &ResponseItem) -> bool {
    response_item_text(item).is_some_and(|text| codex_task_memory::is_task_memory_text(&text))
}

fn response_item_text(item: &ResponseItem) -> Option<String> {
    let ResponseItem::Message { content, .. } = item else {
        return None;
    };
    Some(content_text(content)).filter(|text| !text.is_empty())
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::context::ContextualUserFragment;
    use crate::context::UserShellCommand;

    use super::*;

    fn user_message(text: &str) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: text.to_string(),
            }],
            phase: None,
        }
    }

    #[test]
    fn task_memory_ignores_contextual_user_fragments() {
        let contextual: ResponseItem = ContextualUserFragment::into(UserShellCommand::new(
            "pwd",
            0,
            Duration::from_secs(0),
            "C:\\repo",
        ));

        assert!(build_task_memory(std::slice::from_ref(&contextual)).is_none());
        assert_eq!(real_user_message_count(&[contextual]), 0);
    }

    #[test]
    fn task_memory_does_not_capture_contextual_fragments_as_directives() {
        let contextual: ResponseItem = ContextualUserFragment::into(UserShellCommand::new(
            "pwd",
            0,
            Duration::from_secs(0),
            "C:\\repo",
        ));
        let memory = build_task_memory_item(&[contextual, user_message("Implement the feature.")])
            .expect("expected task memory");
        let text = response_item_text(&memory).expect("expected task memory text");

        assert!(text.contains("Implement the feature."));
        assert!(!text.contains("<user_shell_command>"));
        assert!(!text.contains("C:\\repo"));
    }
}
