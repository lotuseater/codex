use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn side_defers_parent_approval_overlay_until_parent_replay() -> Result<()> {
    let mut app = make_test_app().await;
    let parent_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000011").expect("valid thread");
    let side_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000022").expect("valid thread");

    app.primary_thread_id = Some(parent_thread_id);
    app.active_thread_id = Some(side_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));
    app.thread_event_channels.insert(
        parent_thread_id,
        ThreadEventChannel::new_with_session(
            /*capacity*/ 4,
            test_thread_session(parent_thread_id, test_path_buf("/tmp/main")),
            Vec::new(),
        ),
    );

    app.enqueue_thread_request(
        parent_thread_id,
        exec_approval_request(
            parent_thread_id,
            "turn-approval",
            "call-approval",
            /*approval_id*/ None,
        ),
    )
    .await?;

    assert_eq!(app.chat_widget.has_active_view(), false);
    assert!(app.chat_widget.pending_thread_approvals().is_empty());
    assert_eq!(
        app.side_threads
            .get(&side_thread_id)
            .and_then(|state| state.parent_status),
        Some(SideParentStatus::NeedsApproval)
    );

    let snapshot = {
        let channel = app
            .thread_event_channels
            .get(&parent_thread_id)
            .expect("parent thread channel");
        let store = channel.store.lock().await;
        store.snapshot()
    };
    app.side_threads.remove(&side_thread_id);
    app.active_thread_id = Some(parent_thread_id);
    app.replay_thread_snapshot(snapshot, /*resume_restored_queue*/ false);

    assert_eq!(app.chat_widget.has_active_view(), true);

    Ok(())
}

#[tokio::test]
async fn replay_snapshot_with_pending_request_suppresses_replay_notices() {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000011").expect("valid thread");
    let stale_warning = "stale startup warning that should not cover the approval";

    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: Some(test_thread_session(thread_id, test_path_buf("/tmp/main"))),
            turns: Vec::new(),
            events: vec![
                ThreadBufferedEvent::Notification(ServerNotification::Warning(
                    WarningNotification {
                        thread_id: Some(thread_id.to_string()),
                        message: stale_warning.to_string(),
                    },
                )),
                ThreadBufferedEvent::Request(exec_approval_request(
                    thread_id,
                    "turn-approval",
                    "call-approval",
                    /*approval_id*/ None,
                )),
            ],
            input_state: None,
        },
        /*resume_restored_queue*/ false,
    );

    assert_eq!(app.chat_widget.has_active_view(), true);

    let mut replayed_history = String::new();
    while let Ok(event) = app_event_rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            replayed_history.push_str(&lines_to_single_string(
                &cell.transcript_lines(/*width*/ 80),
            ));
        }
    }

    assert!(
        replayed_history.is_empty(),
        "expected pending approval replay to suppress session notices, got {replayed_history:?}"
    );
}

#[tokio::test]
async fn side_defers_subagent_approval_overlay_until_side_exits() -> Result<()> {
    let mut app = make_test_app().await;
    let main_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000011").expect("valid thread");
    let side_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000022").expect("valid thread");
    let agent_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000033").expect("valid thread");

    app.primary_thread_id = Some(main_thread_id);
    app.active_thread_id = Some(side_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(main_thread_id));
    app.thread_event_channels.insert(
        agent_thread_id,
        ThreadEventChannel::new_with_session(
            /*capacity*/ 4,
            ThreadSessionState {
                approval_policy: AskForApproval::OnRequest.to_core(),
                permission_profile: PermissionProfile::workspace_write(),
                rollout_path: Some(test_path_buf("/tmp/agent-rollout.jsonl")),
                ..test_thread_session(agent_thread_id, test_path_buf("/tmp/agent"))
            },
            Vec::new(),
        ),
    );
    app.agent_navigation.upsert(
        agent_thread_id,
        Some("Robie".to_string()),
        Some("explorer".to_string()),
        /*is_closed*/ false,
    );

    app.enqueue_thread_request(
        agent_thread_id,
        exec_approval_request(
            agent_thread_id,
            "turn-approval",
            "call-approval",
            /*approval_id*/ None,
        ),
    )
    .await?;

    assert_eq!(app.chat_widget.has_active_view(), false);
    assert_eq!(
        app.chat_widget.pending_thread_approvals(),
        &["Robie [explorer]".to_string()]
    );

    app.side_threads.remove(&side_thread_id);
    app.active_thread_id = Some(main_thread_id);
    app.surface_pending_inactive_thread_interactive_requests()
        .await;

    assert_eq!(app.chat_widget.has_active_view(), true);

    Ok(())
}

#[tokio::test]
async fn side_fork_config_is_ephemeral_and_appends_developer_guardrails() {
    let app = make_test_app().await;
    let original_approval_policy = app.config.permissions.approval_policy.value();
    let original_sandbox_policy = app.config.legacy_sandbox_policy();

    let fork_config = app.side_fork_config();

    assert!(fork_config.ephemeral);
    assert_eq!(
        fork_config.permissions.approval_policy.value(),
        original_approval_policy
    );
    assert_eq!(fork_config.legacy_sandbox_policy(), original_sandbox_policy);
    let developer_instructions = fork_config
        .developer_instructions
        .as_deref()
        .expect("side developer instructions");
    assert!(
        developer_instructions.contains("You are in a side conversation, not the main thread.")
    );
    assert!(
        developer_instructions
            .contains("inherited fork history is provided only as reference context")
    );
    assert!(
        developer_instructions.contains(
            "Only instructions submitted after the side-conversation boundary are active"
        )
    );
    assert!(developer_instructions.contains("Do not continue, execute, or complete any task"));
    assert!(
        developer_instructions
            .contains("External tools may be available according to this thread's current")
    );
    assert!(
        developer_instructions
            .contains("Any MCP or external tool calls or outputs visible in the inherited")
    );
    assert!(developer_instructions.contains("non-mutating inspection"));
    assert!(developer_instructions.contains("Do not modify files"));
    assert!(developer_instructions.contains("Do not request escalated permissions"));
    assert!(app.transcript_cells.is_empty());
}

#[tokio::test]
async fn side_fork_config_inherits_parent_thread_runtime_settings() {
    let mut app = make_test_app().await;
    app.config.model = Some("persisted-default-model".to_string());
    app.config.model_reasoning_effort = Some(ReasoningEffortConfig::Low);

    let parent_service_tier = ServiceTier::Fast.request_value();
    let parent_permission_profile = PermissionProfile::workspace_write();
    app.chat_widget.set_model("parent-thread-model");
    app.chat_widget
        .set_reasoning_effort(Some(ReasoningEffortConfig::High));
    app.chat_widget
        .set_service_tier(Some(parent_service_tier.to_string()));
    app.chat_widget
        .set_approval_policy(AskForApproval::OnRequest);
    app.chat_widget
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::legacy(
            parent_permission_profile.clone(),
        ))
        .expect("test permission profile should be accepted");
    app.chat_widget
        .set_approvals_reviewer(ApprovalsReviewer::AutoReview);

    let fork_config = app.side_fork_config();

    assert_eq!(
        (
            fork_config.model.as_deref(),
            fork_config.model_reasoning_effort,
            fork_config.service_tier.as_deref(),
            fork_config.permissions.approval_policy.value(),
            fork_config.permissions.permission_profile(),
            fork_config.approvals_reviewer,
        ),
        (
            Some("parent-thread-model"),
            Some(ReasoningEffortConfig::High),
            Some(parent_service_tier),
            AskForApproval::OnRequest.to_core(),
            &parent_permission_profile,
            ApprovalsReviewer::AutoReview,
        )
    );
}

#[tokio::test]
async fn side_start_block_message_tracks_open_side_conversation() {
    let mut app = make_test_app().await;
    assert_eq!(
        app.side_start_block_message(),
        Some("'/side' is unavailable until the main thread is ready.")
    );

    app.primary_thread_id = Some(ThreadId::new());
    assert_eq!(app.side_start_block_message(), None);

    let parent_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));

    assert_eq!(
        app.side_start_block_message(),
        Some(
            "A side conversation is already open. Press Ctrl+C to return before starting another."
        )
    );

    app.side_threads.remove(&side_thread_id);
    assert_eq!(app.side_start_block_message(), None);
}

#[tokio::test]
async fn side_parent_status_tracks_parent_turn_lifecycle() -> Result<()> {
    let mut app = make_test_app().await;
    let parent_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    app.primary_thread_id = Some(parent_thread_id);
    app.active_thread_id = Some(side_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));

    app.enqueue_thread_notification(
        parent_thread_id,
        turn_completed_notification(parent_thread_id, "turn-1", TurnStatus::Completed),
    )
    .await?;
    assert_eq!(
        app.side_threads
            .get(&side_thread_id)
            .and_then(|state| state.parent_status),
        Some(SideParentStatus::Finished)
    );

    app.enqueue_thread_notification(
        parent_thread_id,
        turn_started_notification(parent_thread_id, "turn-2"),
    )
    .await?;
    assert_eq!(
        app.side_threads
            .get(&side_thread_id)
            .and_then(|state| state.parent_status),
        None
    );

    app.enqueue_thread_notification(
        parent_thread_id,
        turn_completed_notification(parent_thread_id, "turn-2", TurnStatus::Failed),
    )
    .await?;
    assert_eq!(
        app.side_threads
            .get(&side_thread_id)
            .and_then(|state| state.parent_status),
        Some(SideParentStatus::Failed)
    );

    Ok(())
}

#[tokio::test]
async fn side_parent_status_prioritizes_input_over_approval() -> Result<()> {
    let mut app = make_test_app().await;
    let parent_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    app.primary_thread_id = Some(parent_thread_id);
    app.active_thread_id = Some(side_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));

    app.enqueue_thread_request(
        parent_thread_id,
        exec_approval_request(
            parent_thread_id,
            "turn-approval",
            "call-approval",
            /*approval_id*/ None,
        ),
    )
    .await?;
    assert_eq!(
        app.side_threads
            .get(&side_thread_id)
            .and_then(|state| state.parent_status),
        Some(SideParentStatus::NeedsApproval)
    );

    app.enqueue_thread_request(
        parent_thread_id,
        request_user_input_request(parent_thread_id, "turn-input", "call-input"),
    )
    .await?;
    assert_eq!(
        app.side_threads
            .get(&side_thread_id)
            .and_then(|state| state.parent_status),
        Some(SideParentStatus::NeedsInput)
    );

    app.enqueue_thread_notification(
        parent_thread_id,
        ServerNotification::ServerRequestResolved(
            codex_app_server_protocol::ServerRequestResolvedNotification {
                thread_id: parent_thread_id.to_string(),
                request_id: AppServerRequestId::Integer(2),
            },
        ),
    )
    .await?;
    assert_eq!(
        app.side_threads
            .get(&side_thread_id)
            .and_then(|state| state.parent_status),
        Some(SideParentStatus::NeedsApproval)
    );

    app.enqueue_thread_notification(
        parent_thread_id,
        ServerNotification::ServerRequestResolved(
            codex_app_server_protocol::ServerRequestResolvedNotification {
                thread_id: parent_thread_id.to_string(),
                request_id: AppServerRequestId::Integer(1),
            },
        ),
    )
    .await?;
    assert_eq!(
        app.side_threads
            .get(&side_thread_id)
            .and_then(|state| state.parent_status),
        None
    );

    Ok(())
}
