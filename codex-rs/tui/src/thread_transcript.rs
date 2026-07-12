//! Render persisted thread turns into history-cell building blocks.

use std::sync::Arc;

use crate::app_server_session::AppServerSession;
use crate::git_action_directives::parse_assistant_markdown;
use crate::history_cell::AgentMarkdownCell;
use crate::history_cell::HistoryCell;
use crate::history_cell::PlainHistoryCell;
use crate::history_cell::ReasoningSummaryCell;
use crate::history_cell::UserHistoryCell;
use crate::multi_agents::sub_agent_activity_summary;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadItem;
use codex_protocol::ThreadId;
use codex_protocol::items::UserMessageItem;
use ratatui::style::Stylize as _;
use ratatui::text::Line;

pub(crate) type TranscriptCells = Vec<Arc<dyn HistoryCell>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RawReasoningVisibility {
    Hidden,
    Visible,
}

pub(crate) async fn load_session_transcript(
    app_server: &mut AppServerSession,
    thread_id: ThreadId,
    raw_reasoning_visibility: RawReasoningVisibility,
) -> std::io::Result<TranscriptCells> {
    let thread = app_server
        .thread_read(thread_id, /*include_turns*/ true)
        .await
        .map_err(std::io::Error::other)?;
    Ok(thread_to_transcript_cells(
        &thread,
        raw_reasoning_visibility,
    ))
}

pub(crate) fn thread_to_transcript_cells(
    thread: &Thread,
    raw_reasoning_visibility: RawReasoningVisibility,
) -> TranscriptCells {
    let cwd = thread.cwd.as_path();
    let mut cells: TranscriptCells = Vec::new();
    for item in thread.turns.iter().flat_map(|turn| turn.items.iter()) {
        match item {
            ThreadItem::UserMessage {
                id,
                client_id,
                content,
            } => {
                let item = UserMessageItem {
                    id: id.clone(),
                    client_id: client_id.clone(),
                    content: content
                        .iter()
                        .cloned()
                        .map(codex_app_server_protocol::UserInput::into_core)
                        .collect(),
                };
                cells.push(Arc::new(UserHistoryCell {
                    message: item.message(),
                    text_elements: item.text_elements(),
                    local_image_paths: item.local_image_paths(),
                    remote_image_urls: item.image_urls(),
                }));
            }
            ThreadItem::AgentMessage { text, .. } => {
                let parsed = parse_assistant_markdown(text, cwd);
                if !parsed.visible_markdown.trim().is_empty() {
                    cells.push(Arc::new(AgentMarkdownCell::new(
                        parsed.visible_markdown,
                        cwd,
                    )));
                }
            }
            ThreadItem::Plan { text, .. } => {
                if !text.trim().is_empty() {
                    cells.push(Arc::new(crate::history_cell::new_proposed_plan(
                        text.clone(),
                        cwd,
                    )));
                }
            }
            ThreadItem::Reasoning {
                summary, content, ..
            } => {
                let (header, text) =
                    if matches!(raw_reasoning_visibility, RawReasoningVisibility::Visible)
                        && !content.is_empty()
                    {
                        ("Reasoning".to_string(), content.join("\n\n"))
                    } else {
                        split_reasoning_summary_parts(summary)
                    };
                if !text.trim().is_empty() {
                    cells.push(Arc::new(ReasoningSummaryCell::new(
                        header, text, cwd, /*transcript_only*/ false,
                    )));
                }
            }
            other => {
                if let Some(cell) = fallback_transcript_cell(other) {
                    cells.push(Arc::new(cell));
                }
            }
        }
    }
    if cells.is_empty() {
        cells.push(Arc::new(PlainHistoryCell::new(vec![
            "No transcript content available".italic().dim().into(),
        ])));
    }
    cells
}

fn fallback_transcript_cell(item: &ThreadItem) -> Option<PlainHistoryCell> {
    let lines = match item {
        ThreadItem::HookPrompt { fragments, .. } => fragments
            .iter()
            .map(|fragment| {
                vec![
                    "hook prompt: ".dim(),
                    fragment.text.trim().to_string().into(),
                ]
                .into()
            })
            .collect::<Vec<_>>(),
        ThreadItem::CommandExecution {
            command,
            status,
            aggregated_output,
            exit_code,
            ..
        } => {
            let mut lines: Vec<Line<'static>> =
                vec![vec!["$ ".dim(), command.clone().into()].into()];
            lines.push(
                format!(
                    "status: {status:?}{}",
                    exit_code
                        .map(|code| format!(" · exit {code}"))
                        .unwrap_or_default()
                )
                .dim()
                .into(),
            );
            if let Some(output) = aggregated_output.as_deref()
                && !output.trim().is_empty()
            {
                lines.extend(
                    output
                        .lines()
                        .map(|line| vec!["  ".dim(), line.trim_end().to_string().dim()].into()),
                );
            }
            lines
        }
        ThreadItem::FileChange {
            changes, status, ..
        } => vec![
            format!("file changes: {status:?} · {} changes", changes.len())
                .dim()
                .into(),
        ],
        ThreadItem::McpToolCall {
            server,
            tool,
            status,
            ..
        } => vec![
            format!("mcp tool: {server}/{tool} · {status:?}")
                .dim()
                .into(),
        ],
        ThreadItem::DynamicToolCall {
            namespace,
            tool,
            status,
            ..
        } => {
            let name = namespace
                .as_ref()
                .map(|namespace| format!("{namespace}/{tool}"))
                .unwrap_or_else(|| tool.clone());
            vec![format!("tool: {name} · {status:?}").dim().into()]
        }
        ThreadItem::CollabAgentToolCall { tool, status, .. } => {
            vec![format!("agent tool: {tool:?} · {status:?}").dim().into()]
        }
        ThreadItem::SubAgentActivity {
            kind, agent_path, ..
        } => {
            vec![sub_agent_activity_summary(*kind, agent_path).dim().into()]
        }
        ThreadItem::WebSearch(item) => {
            vec![vec!["web search: ".dim(), item.query.clone().into()].into()]
        }
        ThreadItem::ImageView { path, .. } => {
            let path = path.render_for_ui();
            vec![format!("image: {path}").dim().into()]
        }
        ThreadItem::ImageGeneration(item) => {
            let saved = item
                .saved_path
                .as_ref()
                .map(|path| format!(" · {}", path.as_path().display()))
                .unwrap_or_default();
            vec![
                format!("image generation: {}{saved}", item.status)
                    .dim()
                    .into(),
            ]
        }
        ThreadItem::EnteredReviewMode { review, .. } => {
            vec![vec!["review started: ".dim(), review.clone().into()].into()]
        }
        ThreadItem::ExitedReviewMode { review, .. } => {
            vec![vec!["review finished: ".dim(), review.clone().into()].into()]
        }
        ThreadItem::ContextCompaction { .. } => {
            vec!["context compacted".dim().into()]
        }
        ThreadItem::UserMessage { .. }
        | ThreadItem::AgentMessage { .. }
        | ThreadItem::Plan { .. }
        | ThreadItem::Reasoning { .. }
        | ThreadItem::Sleep { .. } => return None,
    };
    (!lines.is_empty()).then(|| PlainHistoryCell::new(lines))
}

/// Split structured reasoning-summary parts into the status header and renderable content.
///
/// Mirrors `codex_tui_render::history_cell::split_reasoning_summary_parts`, which upstream made
/// crate-private to `codex-tui-render`. Inlined here (byte-for-byte with the upstream body) so the
/// fork's thread-transcript rendering keeps its exact split semantics — header extraction and
/// empty-placeholder handling — without depending on that now-inaccessible crate-private helper.
fn split_reasoning_summary_parts(reasoning_parts: &[String]) -> (String, String) {
    let mut leading_empty_part_header = None;
    let mut content_parts = Vec::with_capacity(reasoning_parts.len());

    for part in reasoning_parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let header_end = part.strip_prefix("**").and_then(|after_open| {
            after_open
                .find("**")
                .and_then(|close| (close > 0).then_some(close + 4))
        });
        let body = header_end.map_or(part, |header_end| &part[header_end..]);
        if body.trim() == "<!-- -->" {
            if content_parts.is_empty()
                && leading_empty_part_header.is_none()
                && let Some(header_end) = header_end
            {
                leading_empty_part_header = Some(part[..header_end].to_string());
            }
            continue;
        }

        content_parts.push(part);
    }

    let content = content_parts.join("\n\n");
    if content.is_empty() {
        return (leading_empty_part_header.unwrap_or_default(), content);
    }

    if let Some(after_open) = content.strip_prefix("**")
        && let Some(close) = after_open.find("**")
    {
        let after_close_idx = 2 + close + 2;
        let after_close = &content[after_close_idx..];
        if after_close.starts_with('\n') || after_close.starts_with('\r') {
            return (
                content[..after_close_idx].to_string(),
                after_close.to_string(),
            );
        }
    }

    (leading_empty_part_header.unwrap_or_default(), content)
}
