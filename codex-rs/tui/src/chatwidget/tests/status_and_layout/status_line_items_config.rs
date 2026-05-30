use super::super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn status_line_invalid_items_warn_once() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.config.tui_status_line = Some(vec![
        "model_name".to_string(),
        "bogus_item".to_string(),
        "lines_changed".to_string(),
        "bogus_item".to_string(),
    ]);
    chat.thread_id = Some(ThreadId::new());

    chat.refresh_status_line();
    let cells = drain_insert_history(&mut rx);
    assert_eq!(cells.len(), 1, "expected one warning history cell");
    let rendered = lines_to_single_string(&cells[0]);
    assert!(
        rendered.contains("bogus_item"),
        "warning cell missing invalid item content: {rendered}"
    );

    chat.refresh_status_line();
    let cells = drain_insert_history(&mut rx);
    assert!(
        cells.is_empty(),
        "expected invalid status line warning to emit only once"
    );
}

#[tokio::test]
async fn status_line_context_used_renders_labeled_percent() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.config.tui_status_line = Some(vec!["context-used".to_string()]);

    chat.refresh_status_line();

    assert_eq!(status_line_text(&chat), Some("Context 0% used".to_string()));
    assert!(
        drain_insert_history(&mut rx).is_empty(),
        "context-used should remain a valid status line item"
    );
}

#[tokio::test]
async fn status_line_context_remaining_renders_labeled_percent() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.config.tui_status_line = Some(vec!["context-remaining".to_string()]);

    chat.refresh_status_line();

    assert_eq!(
        status_line_text(&chat),
        Some("Context 100% left".to_string())
    );
    assert!(
        drain_insert_history(&mut rx).is_empty(),
        "context-remaining should remain a valid status line item"
    );
}

#[tokio::test]
async fn status_line_legacy_context_usage_renders_context_used_percent() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.config.tui_status_line = Some(vec!["context-usage".to_string()]);

    chat.refresh_status_line();

    assert_eq!(status_line_text(&chat), Some("Context 0% used".to_string()));
    assert!(
        drain_insert_history(&mut rx).is_empty(),
        "legacy context-usage should remain a valid status line item"
    );
}

#[tokio::test]
async fn status_line_branch_state_resets_when_git_branch_disabled() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.status_line_branch = Some("main".to_string());
    chat.status_line_branch_pending = true;
    chat.status_line_branch_lookup_complete = true;
    chat.config.tui_status_line = Some(vec!["model_name".to_string()]);

    chat.refresh_status_line();

    assert_eq!(chat.status_line_branch, None);
    assert!(!chat.status_line_branch_pending);
    assert!(!chat.status_line_branch_lookup_complete);
}

#[tokio::test]
async fn status_line_branch_refreshes_after_turn_complete() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    install_noop_workspace_command_runner(&mut chat);
    chat.config.tui_status_line = Some(vec!["git-branch".to_string()]);
    chat.status_line_branch_lookup_complete = true;
    chat.status_line_branch_pending = false;

    handle_turn_completed(&mut chat, "turn-1", /*duration_ms*/ None);

    assert!(chat.status_line_branch_pending);
}

#[tokio::test]
async fn status_line_branch_refreshes_after_interrupt() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    install_noop_workspace_command_runner(&mut chat);
    chat.config.tui_status_line = Some(vec!["git-branch".to_string()]);
    chat.status_line_branch_lookup_complete = true;
    chat.status_line_branch_pending = false;

    handle_turn_interrupted(&mut chat, "turn-1");

    assert!(chat.status_line_branch_pending);
}

fn install_noop_workspace_command_runner(chat: &mut ChatWidget) {
    chat.workspace_command_runner = Some(std::sync::Arc::new(NoopWorkspaceCommandRunner));
}

struct NoopWorkspaceCommandRunner;

impl crate::workspace_command::WorkspaceCommandExecutor for NoopWorkspaceCommandRunner {
    fn run(
        &self,
        _command: crate::workspace_command::WorkspaceCommand,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        crate::workspace_command::WorkspaceCommandOutput,
                        crate::workspace_command::WorkspaceCommandError,
                    >,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async {
            Ok(crate::workspace_command::WorkspaceCommandOutput {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            })
        })
    }
}
