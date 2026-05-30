use super::common::*;
use super::super::*;
use crate::bottom_pane::goal_status_indicator_line;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn status_line_goal_active_token_budget_footer_snapshot() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.4")).await;
    chat.set_feature_enabled(Feature::Goals, /*enabled*/ true);
    chat.show_welcome_banner = false;
    chat.config.tui_status_line = Some(vec!["model-name".to_string()]);
    chat.refresh_status_line();
    chat.handle_server_notification(
        ServerNotification::ThreadGoalUpdated(
            codex_app_server_protocol::ThreadGoalUpdatedNotification {
                thread_id: "thread-1".to_string(),
                turn_id: None,
                goal: test_thread_goal(
                    codex_app_server_protocol::ThreadGoalStatus::Active,
                    /*token_budget*/ Some(50_000),
                    /*tokens_used*/ 40_000,
                ),
            },
        ),
        /*replay_kind*/ None,
    );

    let width = 80;
    let height = chat.desired_height(width);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("create terminal");
    terminal
        .draw(|f| chat.render(f.area(), f.buffer_mut()))
        .expect("draw goal status footer");
    assert_chatwidget_snapshot!(
        "status_line_goal_active_token_budget_footer",
        normalized_backend_snapshot(terminal.backend())
    );
}

#[tokio::test]
async fn status_line_goal_complete_elapsed_footer_snapshot() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.4")).await;
    chat.set_feature_enabled(Feature::Goals, /*enabled*/ true);
    chat.show_welcome_banner = false;
    chat.config.tui_status_line = Some(vec!["model-name".to_string()]);
    chat.refresh_status_line();
    let mut goal = test_thread_goal(
        codex_app_server_protocol::ThreadGoalStatus::Complete,
        /*token_budget*/ None,
        /*tokens_used*/ 40_000,
    );
    goal.time_used_seconds = 2 * 24 * 60 * 60 + 23 * 60 * 60 + 42 * 60;
    chat.handle_server_notification(
        ServerNotification::ThreadGoalUpdated(
            codex_app_server_protocol::ThreadGoalUpdatedNotification {
                thread_id: "thread-1".to_string(),
                turn_id: None,
                goal,
            },
        ),
        /*replay_kind*/ None,
    );

    let width = 80;
    let height = chat.desired_height(width);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("create terminal");
    terminal
        .draw(|f| chat.render(f.area(), f.buffer_mut()))
        .expect("draw goal status footer");
    assert_chatwidget_snapshot!(
        "status_line_goal_complete_elapsed_footer",
        normalized_backend_snapshot(terminal.backend())
    );
}

#[tokio::test]
async fn session_configured_clears_goal_status_footer() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.4")).await;
    chat.set_feature_enabled(Feature::Goals, /*enabled*/ true);
    chat.handle_server_notification(
        ServerNotification::ThreadGoalUpdated(
            codex_app_server_protocol::ThreadGoalUpdatedNotification {
                thread_id: "thread-1".to_string(),
                turn_id: None,
                goal: test_thread_goal(
                    codex_app_server_protocol::ThreadGoalStatus::Active,
                    /*token_budget*/ Some(50_000),
                    /*tokens_used*/ 40_000,
                ),
            },
        ),
        /*replay_kind*/ None,
    );
    assert_eq!(
        chat.current_goal_status_indicator,
        Some(GoalStatusIndicator::Active {
            usage: Some("40K / 50K".to_string())
        })
    );
    chat.turn_lifecycle
        .budget_limited_turn_ids
        .insert("turn-1".to_string());

    let rollout_file = NamedTempFile::new().unwrap();
    chat.handle_thread_session(crate::session_state::ThreadSessionState {
        thread_id: ThreadId::new(),
        forked_from_id: None,
        fork_parent_title: None,
        thread_name: None,
        model: "gpt-5.4".to_string(),
        model_provider_id: "test-provider".to_string(),
        service_tier: None,
        approval_policy: AskForApproval::Never.to_core(),
        approvals_reviewer: ApprovalsReviewer::User,
        permission_profile: PermissionProfile::read_only(),
        active_permission_profile: None,
        cwd: test_path_buf("/home/user/project").abs(),
        runtime_workspace_roots: Vec::new(),
        instruction_source_paths: Vec::new(),
        reasoning_effort: Some(ReasoningEffortConfig::default()),
        collaboration_mode: None,
        personality: None,
        message_history: None,
        network_proxy: None,
        rollout_path: Some(rollout_file.path().to_path_buf()),
    });

    assert_eq!(chat.current_goal_status_indicator, None);
    assert!(chat.turn_lifecycle.budget_limited_turn_ids.is_empty());
}

#[tokio::test]
async fn thread_goal_update_for_other_thread_is_ignored() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.4")).await;
    chat.set_feature_enabled(Feature::Goals, /*enabled*/ true);
    chat.thread_id = Some(ThreadId::new());
    let other_thread_id = ThreadId::new().to_string();
    let mut goal = test_thread_goal(
        codex_app_server_protocol::ThreadGoalStatus::BudgetLimited,
        /*token_budget*/ Some(50_000),
        /*tokens_used*/ 50_000,
    );
    goal.thread_id = other_thread_id.clone();

    chat.handle_server_notification(
        ServerNotification::ThreadGoalUpdated(
            codex_app_server_protocol::ThreadGoalUpdatedNotification {
                thread_id: other_thread_id,
                turn_id: Some("turn-other".to_string()),
                goal,
            },
        ),
        /*replay_kind*/ None,
    );

    assert_eq!(chat.current_goal_status_indicator, None);
    assert!(chat.current_goal_status.is_none());
    assert!(chat.turn_lifecycle.budget_limited_turn_ids.is_empty());
}

#[test]
fn goal_status_indicator_formats_statuses_and_budgets() {
    assert_eq!(
        goal_status_indicator_from_app_goal(&test_thread_goal(
            codex_app_server_protocol::ThreadGoalStatus::Active,
            /*token_budget*/ Some(50_000),
            /*tokens_used*/ 40_000,
        )),
        Some(GoalStatusIndicator::Active {
            usage: Some("40K / 50K".to_string()),
        })
    );
    assert_eq!(
        goal_status_indicator_from_app_goal(&test_thread_goal(
            codex_app_server_protocol::ThreadGoalStatus::Active,
            /*token_budget*/ None,
            /*tokens_used*/ 0,
        )),
        Some(GoalStatusIndicator::Active {
            usage: Some("30m".to_string()),
        })
    );
    assert_eq!(
        goal_status_indicator_from_app_goal(&test_thread_goal(
            codex_app_server_protocol::ThreadGoalStatus::Blocked,
            /*token_budget*/ None,
            /*tokens_used*/ 0,
        )),
        Some(GoalStatusIndicator::Blocked)
    );
    assert_eq!(
        goal_status_indicator_from_app_goal(&test_thread_goal(
            codex_app_server_protocol::ThreadGoalStatus::UsageLimited,
            /*token_budget*/ None,
            /*tokens_used*/ 0,
        )),
        Some(GoalStatusIndicator::UsageLimited)
    );
    assert_eq!(
        goal_status_indicator_from_app_goal(&test_thread_goal(
            codex_app_server_protocol::ThreadGoalStatus::BudgetLimited,
            /*token_budget*/ Some(50_000),
            /*tokens_used*/ 51_000,
        )),
        Some(GoalStatusIndicator::BudgetLimited {
            usage: Some("51K / 50K tokens".to_string()),
        })
    );
    assert_eq!(
        goal_status_indicator_from_app_goal(&test_thread_goal(
            codex_app_server_protocol::ThreadGoalStatus::BudgetLimited,
            /*token_budget*/ None,
            /*tokens_used*/ 0,
        )),
        Some(GoalStatusIndicator::BudgetLimited { usage: None })
    );
    assert_eq!(
        goal_status_indicator_from_app_goal(&test_thread_goal(
            codex_app_server_protocol::ThreadGoalStatus::Complete,
            /*token_budget*/ Some(50_000),
            /*tokens_used*/ 40_000,
        )),
        Some(GoalStatusIndicator::Complete {
            usage: Some("40K tokens".to_string()),
        })
    );
}

#[test]
fn goal_status_indicator_line_formats_goal_text() {
    let cases = [
        (
            GoalStatusIndicator::Active {
                usage: Some("4K / 5K".to_string()),
            },
            "Pursuing goal (4K / 5K)",
        ),
        (
            GoalStatusIndicator::BudgetLimited {
                usage: Some("4K / 5K tokens".to_string()),
            },
            "Goal unmet (4K / 5K tokens)",
        ),
        (GoalStatusIndicator::Paused, "Goal paused (/goal resume)"),
        (GoalStatusIndicator::Blocked, "Goal blocked (/goal resume)"),
        (
            GoalStatusIndicator::UsageLimited,
            "Goal hit usage limits (/goal resume)",
        ),
        (
            GoalStatusIndicator::BudgetLimited { usage: None },
            "Goal abandoned",
        ),
        (
            GoalStatusIndicator::Complete {
                usage: Some("10h 12m".to_string()),
            },
            "Goal achieved (10h 12m)",
        ),
        (
            GoalStatusIndicator::Complete { usage: None },
            "Goal achieved",
        ),
    ];

    for (indicator, expected) in cases {
        let line =
            goal_status_indicator_line(Some(&indicator)).expect("goal indicator should render");
        let actual = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert_eq!(expected, actual);
    }
}
