use super::*;

#[tokio::test]
async fn handle_mcp_inventory_result_clears_committed_loading_cell() {
    let mut app = make_test_app().await;
    app.transcript_cells
        .push(Arc::new(history_cell::new_mcp_inventory_loading(
            /*animations_enabled*/ false,
        )));

    app.handle_mcp_inventory_result(
        Ok(vec![McpServerStatus {
            name: "docs".to_string(),
            tools: HashMap::new(),
            resources: Vec::new(),
            resource_templates: Vec::new(),
            auth_status: codex_app_server_protocol::McpAuthStatus::Unsupported,
        }]),
        McpServerStatusDetail::ToolsAndAuthOnly,
    );

    assert_eq!(app.transcript_cells.len(), 0);
}

#[test]
fn bypass_hook_trust_startup_warning_snapshot() {
    let rendered = lines_to_single_string(
        &history_cell::new_warning_event(
            "`--dangerously-bypass-hook-trust` is enabled. Enabled hooks may run without review for this invocation."
                .to_string(),
        )
        .display_lines(/*width*/ 80),
    );

    assert_app_snapshot!("bypass_hook_trust_startup_warning", rendered);
}

#[tokio::test]
async fn feedback_submission_without_thread_emits_error_history_cell() {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;

    app.handle_feedback_submitted(
        /*origin_thread_id*/ None,
        FeedbackCategory::Bug,
        /*include_logs*/ true,
        Err("boom".to_string()),
    )
    .await;

    let cell = match app_event_rx.try_recv() {
        Ok(AppEvent::InsertHistoryCell(cell)) => cell,
        other => panic!("expected feedback error history cell, saw {other:?}"),
    };
    assert_eq!(
        lines_to_single_string(&cell.display_lines(/*width*/ 120)),
        "■ Failed to upload feedback: boom"
    );
}

#[tokio::test]
async fn feedback_submission_for_inactive_thread_replays_into_origin_thread() {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let origin_thread_id = ThreadId::new();
    let active_thread_id = ThreadId::new();
    let origin_session = test_thread_session(origin_thread_id, test_path_buf("/tmp/origin"));
    let active_session = test_thread_session(active_thread_id, test_path_buf("/tmp/active"));
    app.thread_event_channels.insert(
        origin_thread_id,
        ThreadEventChannel::new_with_session(
            THREAD_EVENT_CHANNEL_CAPACITY,
            origin_session.clone(),
            Vec::new(),
        ),
    );
    app.thread_event_channels.insert(
        active_thread_id,
        ThreadEventChannel::new_with_session(
            THREAD_EVENT_CHANNEL_CAPACITY,
            active_session.clone(),
            Vec::new(),
        ),
    );
    app.activate_thread_channel(active_thread_id).await;
    app.chat_widget.handle_thread_session(active_session);
    while app_event_rx.try_recv().is_ok() {}

    app.handle_feedback_submitted(
        Some(origin_thread_id),
        FeedbackCategory::Bug,
        /*include_logs*/ true,
        Ok("uploaded-thread".to_string()),
    )
    .await;

    assert_matches!(
        app_event_rx.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    );

    let snapshot = {
        let channel = app
            .thread_event_channels
            .get(&origin_thread_id)
            .expect("origin thread channel should exist");
        let store = channel.store.lock().await;
        assert!(matches!(
            store.buffer.back(),
            Some(ThreadBufferedEvent::FeedbackSubmission(_))
        ));
        store.snapshot()
    };

    app.replay_thread_snapshot(snapshot, /*resume_restored_queue*/ false);

    let mut rendered_cells = Vec::new();
    while let Ok(event) = app_event_rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            rendered_cells.push(lines_to_single_string(&cell.display_lines(/*width*/ 120)));
        }
    }
    assert!(rendered_cells.iter().any(|cell| {
        cell.contains("• Feedback uploaded. Please open an issue using the following URL:")
            && cell.contains("uploaded-thread")
    }));
}

#[tokio::test]
async fn auto_loop_after_self_review_submits_plan_mode_continuation() {
    let (mut app, _app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    app.auto_loop.settings.enabled = true;
    app.auto_loop.settings.message = "resume after review".to_string();
    app.chat_widget
        .set_thread_id_for_test(Some(ThreadId::new()));

    assert!(app.handle_auto_loop_after_self_review());

    match next_user_turn_op(&mut op_rx) {
        Op::UserTurn {
            items,
            collaboration_mode:
                Some(CollaborationMode {
                    mode: ModeKind::Plan,
                    ..
                }),
            ..
        } => {
            let [UserInput::Text { text, .. }] = items.as_slice() else {
                panic!("expected one text item, got {items:?}");
            };
            assert!(text.contains("Automatic post-self-review loop continuation"));
            assert!(text.contains("resume after review"));
            assert!(text.contains("loop_followup_gain"));
            assert!(text.contains("After plan self-review produces the revised or final plan"));
        }
        other => panic!("expected UserTurn op, got {other:?}"),
    }
}
