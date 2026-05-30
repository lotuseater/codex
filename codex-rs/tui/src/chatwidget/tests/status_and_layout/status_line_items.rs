use super::super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn status_line_git_summary_items_render_values() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.status_line_git_summary = Some(StatusLineGitSummary {
        pull_request: Some(crate::branch_summary::StatusLinePullRequest {
            number: 20_252,
            url: "https://github.com/openai/codex/pull/20252".to_string(),
        }),
        branch_change_stats: Some(crate::branch_summary::GitBranchDiffStats {
            additions: 143,
            deletions: 22,
        }),
    });

    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::PullRequestNumber),
        Some("PR #20252".to_string())
    );
    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::BranchChanges),
        Some("+143 -22".to_string())
    );
}

#[tokio::test]
async fn raw_output_status_line_value_only_shows_when_enabled() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;

    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::RawOutput),
        None
    );

    chat.set_raw_output_mode(/*enabled*/ true);

    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::RawOutput),
        Some("raw output".to_string())
    );
}

#[tokio::test]
async fn status_line_branch_changes_render_no_changes() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.status_line_git_summary = Some(StatusLineGitSummary {
        pull_request: None,
        branch_change_stats: Some(crate::branch_summary::GitBranchDiffStats {
            additions: 0,
            deletions: 0,
        }),
    });

    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::BranchChanges),
        Some("No changes".to_string())
    );
}

#[tokio::test]
async fn stale_status_line_git_summary_update_is_ignored() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.status_line_git_summary_cwd = Some(PathBuf::from("/expected"));
    chat.status_line_git_summary_pending = true;

    chat.set_status_line_git_summary(
        PathBuf::from("/other"),
        StatusLineGitSummary {
            pull_request: Some(crate::branch_summary::StatusLinePullRequest {
                number: 20_252,
                url: "https://github.com/openai/codex/pull/20252".to_string(),
            }),
            branch_change_stats: Some(crate::branch_summary::GitBranchDiffStats {
                additions: 143,
                deletions: 22,
            }),
        },
    );

    assert!(chat.status_line_git_summary.is_none());
    assert!(!chat.status_line_git_summary_pending);
}

#[tokio::test]
async fn raw_output_mode_can_change_without_inserting_notice() {
    let (mut chat, mut rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.set_raw_output_mode(/*enabled*/ true);

    assert!(chat.raw_output_mode());
    assert!(drain_insert_history(&mut rx).is_empty());

    chat.set_raw_output_mode_and_notify(/*enabled*/ false);

    assert!(!chat.raw_output_mode());
    let history = drain_insert_history(&mut rx)
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        history.contains("Raw output mode off: rich transcript rendering restored."),
        "expected raw output notice, got {history:?}"
    );
}
