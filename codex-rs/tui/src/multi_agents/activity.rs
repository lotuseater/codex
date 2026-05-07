use super::AgentMetadata;
use super::agent_label;
use super::agent_label_spans;
use super::title_spans_line;
use crate::history_cell::PlainHistoryCell;
use crate::render::line_utils::prefix_lines;
use crate::text_formatting::truncate_text;
use codex_app_server_protocol::CollabAgentTool;
use codex_app_server_protocol::CollabAgentToolCallStatus;
use codex_app_server_protocol::CommandExecutionStatus;
use codex_app_server_protocol::DynamicToolCallOutputContentItem;
use codex_app_server_protocol::DynamicToolCallStatus;
use codex_app_server_protocol::FileUpdateChange;
use codex_app_server_protocol::McpToolCallStatus;
use codex_app_server_protocol::PatchApplyStatus;
use codex_app_server_protocol::ThreadItem;
use codex_protocol::ThreadId;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;

const SUBAGENT_ACTIVITY_LEFT_INDENT: &str = "    ";
const SUBAGENT_ACTIVITY_PREVIEW_GRAPHEMES: usize = 180;

pub(crate) fn subagent_activity_history_cell(
    thread_id: ThreadId,
    item: &ThreadItem,
    metadata: &AgentMetadata,
) -> Option<PlainHistoryCell> {
    let activity = subagent_activity_summary(item)?;
    let mut title_spans = agent_label_spans(agent_label(thread_id, metadata));
    title_spans.push(Span::from(": ").dim());
    title_spans.push(Span::from(activity.title).bold());

    let mut lines = vec![title_spans_line(title_spans)];
    if !activity.details.is_empty() {
        lines.extend(prefix_lines(activity.details, "  └ ".dim(), "    ".into()));
    }
    let lines = prefix_lines(
        lines,
        SUBAGENT_ACTIVITY_LEFT_INDENT.into(),
        SUBAGENT_ACTIVITY_LEFT_INDENT.into(),
    );
    Some(PlainHistoryCell::new(lines))
}

struct SubagentActivitySummary {
    title: &'static str,
    details: Vec<Line<'static>>,
}

fn subagent_activity_summary(item: &ThreadItem) -> Option<SubagentActivitySummary> {
    match item {
        ThreadItem::AgentMessage { text, .. } => Some(SubagentActivitySummary {
            title: "Message",
            details: preview_lines([text.as_str()]),
        }),
        ThreadItem::Plan { text, .. } => Some(SubagentActivitySummary {
            title: "Plan",
            details: preview_lines([text.as_str()]),
        }),
        ThreadItem::Reasoning {
            summary, content, ..
        } => {
            let details = if summary.is_empty() {
                preview_lines(content.iter().map(String::as_str))
            } else {
                preview_lines(summary.iter().map(String::as_str))
            };
            (!details.is_empty()).then_some(SubagentActivitySummary {
                title: "Reasoning",
                details,
            })
        }
        ThreadItem::CommandExecution {
            command,
            status,
            aggregated_output,
            exit_code,
            duration_ms,
            ..
        } => {
            let mut details = preview_lines([command.as_str()]);
            if let Some(summary) = command_result_summary(*exit_code, *duration_ms) {
                details.push(summary.into());
            }
            if let Some(output) = aggregated_output.as_deref().and_then(preview_text) {
                details.push(Line::from(vec!["Output: ".dim(), output.into()]));
            }
            Some(SubagentActivitySummary {
                title: match status {
                    CommandExecutionStatus::InProgress => "Running command",
                    CommandExecutionStatus::Completed => "Command finished",
                    CommandExecutionStatus::Failed => "Command failed",
                    CommandExecutionStatus::Declined => "Command declined",
                },
                details,
            })
        }
        ThreadItem::FileChange {
            changes, status, ..
        } => Some(SubagentActivitySummary {
            title: match status {
                PatchApplyStatus::InProgress => "Editing files",
                PatchApplyStatus::Completed => "File changes applied",
                PatchApplyStatus::Failed => "File changes failed",
                PatchApplyStatus::Declined => "File changes declined",
            },
            details: file_change_lines(changes),
        }),
        ThreadItem::McpToolCall {
            server,
            tool,
            status,
            arguments,
            result,
            error,
            duration_ms,
            ..
        } => {
            let mut details = vec![Line::from(format!("{server}.{tool}"))];
            if matches!(status, McpToolCallStatus::InProgress)
                && let Some(args) = json_preview(arguments)
            {
                details.push(Line::from(vec!["Args: ".dim(), args.into()]));
            }
            if let Some(duration) = duration_summary(*duration_ms) {
                details.push(duration.into());
            }
            if let Some(error) = error
                .as_ref()
                .and_then(|error| preview_text(&error.message))
            {
                details.push(Line::from(vec!["Error: ".red(), error.into()]));
            } else if let Some(result) = result.as_ref().and_then(|result| {
                result
                    .structured_content
                    .as_ref()
                    .and_then(json_preview)
                    .or_else(|| json_values_preview(&result.content))
            }) {
                details.push(Line::from(vec!["Result: ".dim(), result.into()]));
            }
            Some(SubagentActivitySummary {
                title: match status {
                    McpToolCallStatus::InProgress => "Using tool",
                    McpToolCallStatus::Completed => "Tool finished",
                    McpToolCallStatus::Failed => "Tool failed",
                },
                details,
            })
        }
        ThreadItem::DynamicToolCall {
            namespace,
            tool,
            arguments,
            status,
            content_items,
            success,
            duration_ms,
            ..
        } => {
            let tool_name = namespace
                .as_ref()
                .map(|namespace| format!("{namespace}.{tool}"))
                .unwrap_or_else(|| tool.clone());
            let mut details = vec![Line::from(tool_name)];
            if matches!(status, DynamicToolCallStatus::InProgress)
                && let Some(args) = json_preview(arguments)
            {
                details.push(Line::from(vec!["Args: ".dim(), args.into()]));
            }
            if let Some(duration) = duration_summary(*duration_ms) {
                details.push(duration.into());
            }
            if let Some(success) = success {
                details.push(format!("Success: {success}").into());
            }
            if let Some(output) = content_items
                .as_deref()
                .and_then(dynamic_tool_content_preview)
            {
                details.push(Line::from(vec!["Output: ".dim(), output.into()]));
            }
            Some(SubagentActivitySummary {
                title: match status {
                    DynamicToolCallStatus::InProgress => "Using tool",
                    DynamicToolCallStatus::Completed => "Tool finished",
                    DynamicToolCallStatus::Failed => "Tool failed",
                },
                details,
            })
        }
        ThreadItem::CollabAgentToolCall {
            tool,
            status,
            receiver_thread_ids,
            prompt,
            ..
        } => {
            let mut details = Vec::new();
            details
                .push(format!("{}: {}", collab_tool_name(tool), collab_status_name(status)).into());
            if !receiver_thread_ids.is_empty() {
                details.push(format!("Receivers: {}", receiver_thread_ids.len()).into());
            }
            if let Some(prompt) = prompt.as_deref().and_then(preview_text) {
                details.push(Line::from(vec!["Prompt: ".dim(), prompt.into()]));
            }
            Some(SubagentActivitySummary {
                title: "Agent coordination",
                details,
            })
        }
        ThreadItem::WebSearch { query, action, .. } => {
            let mut details = preview_lines([query.as_str()]);
            if let Some(action) = action {
                details.push(format!("{action:?}").into());
            }
            Some(SubagentActivitySummary {
                title: "Web search",
                details,
            })
        }
        ThreadItem::ImageView { path, .. } => Some(SubagentActivitySummary {
            title: "Viewed image",
            details: vec![path.display().to_string().into()],
        }),
        ThreadItem::ImageGeneration {
            status,
            revised_prompt,
            saved_path,
            ..
        } => {
            let mut details = vec![format!("Status: {status}").into()];
            if let Some(prompt) = revised_prompt.as_deref().and_then(preview_text) {
                details.push(Line::from(vec!["Prompt: ".dim(), prompt.into()]));
            }
            if let Some(path) = saved_path {
                details.push(path.display().to_string().into());
            }
            Some(SubagentActivitySummary {
                title: "Image generation",
                details,
            })
        }
        ThreadItem::EnteredReviewMode { review, .. } => Some(SubagentActivitySummary {
            title: "Entered review mode",
            details: preview_lines([review.as_str()]),
        }),
        ThreadItem::ExitedReviewMode { review, .. } => Some(SubagentActivitySummary {
            title: "Exited review mode",
            details: preview_lines([review.as_str()]),
        }),
        ThreadItem::ContextCompaction { .. } => Some(SubagentActivitySummary {
            title: "Context compacted",
            details: Vec::new(),
        }),
        ThreadItem::UserMessage { .. } | ThreadItem::HookPrompt { .. } => None,
    }
}

fn preview_lines<'a>(texts: impl IntoIterator<Item = &'a str>) -> Vec<Line<'static>> {
    texts
        .into_iter()
        .filter_map(preview_text)
        .map(Line::from)
        .collect()
}

fn preview_text(text: &str) -> Option<String> {
    let preview = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!preview.is_empty()).then(|| truncate_text(&preview, SUBAGENT_ACTIVITY_PREVIEW_GRAPHEMES))
}

fn json_preview(value: &serde_json::Value) -> Option<String> {
    preview_text(&value.to_string())
}

fn json_values_preview(values: &[serde_json::Value]) -> Option<String> {
    serde_json::to_string(values)
        .ok()
        .and_then(|value| preview_text(&value))
}

fn duration_summary(duration_ms: Option<i64>) -> Option<String> {
    duration_ms.map(|duration_ms| format!("Duration: {duration_ms}ms"))
}

fn command_result_summary(exit_code: Option<i32>, duration_ms: Option<i64>) -> Option<String> {
    match (exit_code, duration_ms) {
        (Some(exit_code), Some(duration_ms)) => {
            Some(format!("Exit {exit_code} in {duration_ms}ms"))
        }
        (Some(exit_code), None) => Some(format!("Exit {exit_code}")),
        (None, Some(duration_ms)) => Some(format!("Duration: {duration_ms}ms")),
        (None, None) => None,
    }
}

fn file_change_lines(changes: &[FileUpdateChange]) -> Vec<Line<'static>> {
    if changes.is_empty() {
        return Vec::new();
    }
    let mut paths = changes
        .iter()
        .take(3)
        .map(|change| change.path.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if changes.len() > 3 {
        paths.push_str(&format!(", +{} more", changes.len() - 3));
    }
    vec![Line::from(format!("{} file(s): {paths}", changes.len()))]
}

fn dynamic_tool_content_preview(items: &[DynamicToolCallOutputContentItem]) -> Option<String> {
    let text = items
        .iter()
        .filter_map(|item| match item {
            DynamicToolCallOutputContentItem::InputText { text } => Some(text.as_str()),
            DynamicToolCallOutputContentItem::InputImage { image_url } => Some(image_url.as_str()),
        })
        .collect::<Vec<_>>()
        .join(" ");
    preview_text(&text)
}

fn collab_tool_name(tool: &CollabAgentTool) -> &'static str {
    match tool {
        CollabAgentTool::SpawnAgent => "spawn_agent",
        CollabAgentTool::SendInput => "send_message",
        CollabAgentTool::ResumeAgent => "resume_agent",
        CollabAgentTool::Wait => "wait_agent",
        CollabAgentTool::CloseAgent => "close_agent",
    }
}

fn collab_status_name(status: &CollabAgentToolCallStatus) -> &'static str {
    match status {
        CollabAgentToolCallStatus::InProgress => "running",
        CollabAgentToolCallStatus::Completed => "completed",
        CollabAgentToolCallStatus::Failed => "failed",
    }
}
