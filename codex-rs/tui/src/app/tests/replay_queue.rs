use super::*;

#[tokio::test]
async fn replay_thread_snapshot_restores_draft_and_queued_input() {
    let mut app = make_test_app().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    app.thread_event_channels.insert(
        thread_id,
        ThreadEventChannel::new_with_session(
            THREAD_EVENT_CHANNEL_CAPACITY,
            session.clone(),
            Vec::new(),
        ),
    );
    app.activate_thread_channel(thread_id).await;
    app.chat_widget.handle_thread_session(session.clone());

    app.chat_widget
        .apply_external_edit("draft prompt".to_string());
    app.chat_widget.submit_user_message_with_mode(
        "queued follow-up".to_string(),
        CollaborationModeMask {
            name: "Default".to_string(),
            mode: None,
            model: None,
            reasoning_effort: None,
            developer_instructions: None,
        },
    );
    let expected_input_state = app
        .chat_widget
        .capture_thread_input_state()
        .expect("expected thread input state");

    app.store_active_thread_receiver().await;

    let snapshot = {
        let channel = app
            .thread_event_channels
            .get(&thread_id)
            .expect("thread channel should exist");
        let store = channel.store.lock().await;
        assert_eq!(store.input_state, Some(expected_input_state));
        store.snapshot()
    };

    let (chat_widget, _app_event_tx, _rx, mut new_op_rx) =
        make_chatwidget_manual_with_sender().await;
    app.chat_widget = chat_widget;

    app.replay_thread_snapshot(snapshot, /*resume_restored_queue*/ true);

    assert_eq!(app.chat_widget.composer_text_with_pending(), "draft prompt");
    assert!(app.chat_widget.queued_user_message_texts().is_empty());
    while let Ok(op) = new_op_rx.try_recv() {
        assert!(
            !matches!(op, Op::UserTurn { .. }),
            "draft-only replay should not auto-submit queued input"
        );
    }
}

#[tokio::test]
async fn active_turn_id_for_thread_uses_snapshot_turns() {
    let mut app = make_test_app().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    app.thread_event_channels.insert(
        thread_id,
        ThreadEventChannel::new_with_session(
            THREAD_EVENT_CHANNEL_CAPACITY,
            session,
            vec![test_turn("turn-1", TurnStatus::InProgress, Vec::new())],
        ),
    );

    assert_eq!(
        app.active_turn_id_for_thread(thread_id).await,
        Some("turn-1".to_string())
    );
}

#[tokio::test]
async fn replayed_turn_complete_submits_restored_queued_follow_up() {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    app.chat_widget.handle_thread_session(session.clone());
    app.chat_widget.handle_server_notification(
        turn_started_notification(thread_id, "turn-1"),
        /*replay_kind*/ None,
    );
    app.chat_widget.handle_server_notification(
        agent_message_delta_notification(thread_id, "turn-1", "agent-1", "streaming"),
        /*replay_kind*/ None,
    );
    app.chat_widget
        .apply_external_edit("queued follow-up".to_string());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let input_state = app
        .chat_widget
        .capture_thread_input_state()
        .expect("expected queued follow-up state");

    let (chat_widget, _app_event_tx, _rx, mut new_op_rx) =
        make_chatwidget_manual_with_sender().await;
    app.chat_widget = chat_widget;
    app.chat_widget.handle_thread_session(session.clone());
    while new_op_rx.try_recv().is_ok() {}
    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: None,
            turns: Vec::new(),
            events: vec![ThreadBufferedEvent::Notification(
                turn_completed_notification(thread_id, "turn-1", TurnStatus::Completed),
            )],
            input_state: Some(input_state),
        },
        /*resume_restored_queue*/ true,
    );

    match next_user_turn_op(&mut new_op_rx) {
        Op::UserTurn { items, .. } => assert_eq!(
            items,
            vec![UserInput::Text {
                text: "queued follow-up".to_string(),
                text_elements: Vec::new(),
            }]
        ),
        other => panic!("expected queued follow-up submission, got {other:?}"),
    }
}

#[tokio::test]
async fn replay_only_thread_keeps_restored_queue_visible() {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    app.chat_widget.handle_thread_session(session.clone());
    app.chat_widget.handle_server_notification(
        turn_started_notification(thread_id, "turn-1"),
        /*replay_kind*/ None,
    );
    app.chat_widget.handle_server_notification(
        agent_message_delta_notification(thread_id, "turn-1", "agent-1", "streaming"),
        /*replay_kind*/ None,
    );
    app.chat_widget
        .apply_external_edit("queued follow-up".to_string());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let input_state = app
        .chat_widget
        .capture_thread_input_state()
        .expect("expected queued follow-up state");

    let (chat_widget, _app_event_tx, _rx, mut new_op_rx) =
        make_chatwidget_manual_with_sender().await;
    app.chat_widget = chat_widget;
    app.chat_widget.handle_thread_session(session.clone());
    while new_op_rx.try_recv().is_ok() {}

    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: None,
            turns: Vec::new(),
            events: vec![ThreadBufferedEvent::Notification(
                turn_completed_notification(thread_id, "turn-1", TurnStatus::Completed),
            )],
            input_state: Some(input_state),
        },
        /*resume_restored_queue*/ false,
    );

    assert_eq!(
        app.chat_widget.queued_user_message_texts(),
        vec!["queued follow-up".to_string()]
    );
    assert!(
        new_op_rx.try_recv().is_err(),
        "replay-only threads should not auto-submit restored queue"
    );
}

#[tokio::test]
async fn replay_thread_snapshot_keeps_queue_when_running_state_only_comes_from_snapshot() {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    app.chat_widget.handle_thread_session(session.clone());
    app.chat_widget.handle_server_notification(
        turn_started_notification(thread_id, "turn-1"),
        /*replay_kind*/ None,
    );
    app.chat_widget.handle_server_notification(
        agent_message_delta_notification(thread_id, "turn-1", "agent-1", "streaming"),
        /*replay_kind*/ None,
    );
    app.chat_widget
        .apply_external_edit("queued follow-up".to_string());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let input_state = app
        .chat_widget
        .capture_thread_input_state()
        .expect("expected queued follow-up state");

    let (chat_widget, _app_event_tx, _rx, mut new_op_rx) =
        make_chatwidget_manual_with_sender().await;
    app.chat_widget = chat_widget;
    app.chat_widget.handle_thread_session(session.clone());
    while new_op_rx.try_recv().is_ok() {}

    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: None,
            turns: Vec::new(),
            events: vec![],
            input_state: Some(input_state),
        },
        /*resume_restored_queue*/ true,
    );

    assert_eq!(
        app.chat_widget.queued_user_message_texts(),
        vec!["queued follow-up".to_string()]
    );
    assert!(
        new_op_rx.try_recv().is_err(),
        "restored queue should stay queued when replay did not prove the turn finished"
    );
}

#[tokio::test]
async fn replay_thread_snapshot_in_progress_turn_restores_running_queue_state() {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    app.chat_widget.handle_thread_session(session.clone());
    app.chat_widget.handle_server_notification(
        turn_started_notification(thread_id, "turn-1"),
        /*replay_kind*/ None,
    );
    app.chat_widget.handle_server_notification(
        agent_message_delta_notification(thread_id, "turn-1", "agent-1", "streaming"),
        /*replay_kind*/ None,
    );
    app.chat_widget
        .apply_external_edit("queued follow-up".to_string());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let input_state = app
        .chat_widget
        .capture_thread_input_state()
        .expect("expected queued follow-up state");

    let (chat_widget, _app_event_tx, _rx, mut new_op_rx) =
        make_chatwidget_manual_with_sender().await;
    app.chat_widget = chat_widget;
    app.chat_widget.handle_thread_session(session.clone());
    while new_op_rx.try_recv().is_ok() {}

    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: None,
            turns: vec![test_turn("turn-1", TurnStatus::InProgress, Vec::new())],
            events: Vec::new(),
            input_state: Some(input_state),
        },
        /*resume_restored_queue*/ true,
    );

    assert_eq!(
        app.chat_widget.queued_user_message_texts(),
        vec!["queued follow-up".to_string()]
    );
    assert!(
        new_op_rx.try_recv().is_err(),
        "restored queue should stay queued while replayed turn is still running"
    );
}

#[tokio::test]
async fn replay_thread_snapshot_in_progress_turn_restores_running_state_without_input_state() {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    let (chat_widget, _app_event_tx, _rx, _new_op_rx) = make_chatwidget_manual_with_sender().await;
    app.chat_widget = chat_widget;
    app.chat_widget.handle_thread_session(session);

    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: None,
            turns: vec![test_turn("turn-1", TurnStatus::InProgress, Vec::new())],
            events: Vec::new(),
            input_state: None,
        },
        /*resume_restored_queue*/ false,
    );

    assert!(app.chat_widget.is_task_running_for_test());
}

#[tokio::test]
async fn replay_thread_snapshot_does_not_submit_queue_before_replay_catches_up() {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    app.chat_widget.handle_thread_session(session.clone());
    app.chat_widget.handle_server_notification(
        turn_started_notification(thread_id, "turn-1"),
        /*replay_kind*/ None,
    );
    app.chat_widget.handle_server_notification(
        agent_message_delta_notification(thread_id, "turn-1", "agent-1", "streaming"),
        /*replay_kind*/ None,
    );
    app.chat_widget
        .apply_external_edit("queued follow-up".to_string());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let input_state = app
        .chat_widget
        .capture_thread_input_state()
        .expect("expected queued follow-up state");

    let (chat_widget, _app_event_tx, _rx, mut new_op_rx) =
        make_chatwidget_manual_with_sender().await;
    app.chat_widget = chat_widget;
    app.chat_widget.handle_thread_session(session.clone());
    while new_op_rx.try_recv().is_ok() {}

    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: None,
            turns: Vec::new(),
            events: vec![
                ThreadBufferedEvent::Notification(turn_completed_notification(
                    thread_id,
                    "turn-0",
                    TurnStatus::Completed,
                )),
                ThreadBufferedEvent::Notification(turn_started_notification(thread_id, "turn-1")),
            ],
            input_state: Some(input_state),
        },
        /*resume_restored_queue*/ true,
    );

    assert!(
        new_op_rx.try_recv().is_err(),
        "queued follow-up should stay queued until the latest turn completes"
    );
    assert_eq!(
        app.chat_widget.queued_user_message_texts(),
        vec!["queued follow-up".to_string()]
    );

    app.chat_widget.handle_server_notification(
        turn_completed_notification(thread_id, "turn-1", TurnStatus::Completed),
        /*replay_kind*/ None,
    );

    match next_user_turn_op(&mut new_op_rx) {
        Op::UserTurn { items, .. } => assert_eq!(
            items,
            vec![UserInput::Text {
                text: "queued follow-up".to_string(),
                text_elements: Vec::new(),
            }]
        ),
        other => panic!("expected queued follow-up submission, got {other:?}"),
    }
}

#[tokio::test]
async fn replay_thread_snapshot_restores_pending_pastes_for_submit() {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    app.thread_event_channels.insert(
        thread_id,
        ThreadEventChannel::new_with_session(
            THREAD_EVENT_CHANNEL_CAPACITY,
            session.clone(),
            Vec::new(),
        ),
    );
    app.activate_thread_channel(thread_id).await;
    app.chat_widget.handle_thread_session(session);

    let large = "x".repeat(1005);
    app.chat_widget.handle_paste(large.clone());
    let expected_input_state = app
        .chat_widget
        .capture_thread_input_state()
        .expect("expected thread input state");

    app.store_active_thread_receiver().await;

    let snapshot = {
        let channel = app
            .thread_event_channels
            .get(&thread_id)
            .expect("thread channel should exist");
        let store = channel.store.lock().await;
        assert_eq!(store.input_state, Some(expected_input_state));
        store.snapshot()
    };

    let (chat_widget, _app_event_tx, _rx, mut new_op_rx) =
        make_chatwidget_manual_with_sender().await;
    app.chat_widget = chat_widget;
    app.replay_thread_snapshot(snapshot, /*resume_restored_queue*/ true);

    assert_eq!(app.chat_widget.composer_text_with_pending(), large);

    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    match next_user_turn_op(&mut new_op_rx) {
        Op::UserTurn { items, .. } => assert_eq!(
            items,
            vec![UserInput::Text {
                text: large,
                text_elements: Vec::new(),
            }]
        ),
        other => panic!("expected restored paste submission, got {other:?}"),
    }
}

#[tokio::test]
async fn replay_thread_snapshot_restores_collaboration_mode_for_draft_submit() {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    app.chat_widget.handle_thread_session(session.clone());
    app.chat_widget
        .set_reasoning_effort(Some(ReasoningEffortConfig::High));
    app.chat_widget
        .set_collaboration_mask(CollaborationModeMask {
            name: "Plan".to_string(),
            mode: Some(ModeKind::Plan),
            model: Some("gpt-restored".to_string()),
            reasoning_effort: Some(Some(ReasoningEffortConfig::High)),
            developer_instructions: None,
        });
    app.chat_widget
        .apply_external_edit("draft prompt".to_string());
    let input_state = app
        .chat_widget
        .capture_thread_input_state()
        .expect("expected draft input state");

    let (chat_widget, _app_event_tx, _rx, mut new_op_rx) =
        make_chatwidget_manual_with_sender().await;
    app.chat_widget = chat_widget;
    app.chat_widget.handle_thread_session(session.clone());
    app.chat_widget
        .set_reasoning_effort(Some(ReasoningEffortConfig::Low));
    app.chat_widget
        .set_collaboration_mask(CollaborationModeMask {
            name: "Default".to_string(),
            mode: Some(ModeKind::Default),
            model: Some("gpt-replacement".to_string()),
            reasoning_effort: Some(Some(ReasoningEffortConfig::Low)),
            developer_instructions: None,
        });
    while new_op_rx.try_recv().is_ok() {}

    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: None,
            turns: Vec::new(),
            events: vec![],
            input_state: Some(input_state),
        },
        /*resume_restored_queue*/ true,
    );
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    match next_user_turn_op(&mut new_op_rx) {
        Op::UserTurn {
            items,
            model,
            effort,
            collaboration_mode,
            ..
        } => {
            assert_eq!(
                items,
                vec![UserInput::Text {
                    text: "draft prompt".to_string(),
                    text_elements: Vec::new(),
                }]
            );
            assert_eq!(model, "gpt-restored".to_string());
            assert_eq!(effort, Some(ReasoningEffortConfig::High));
            assert_eq!(
                collaboration_mode,
                Some(CollaborationMode {
                    mode: ModeKind::Plan,
                    settings: Settings {
                        model: "gpt-restored".to_string(),
                        reasoning_effort: Some(ReasoningEffortConfig::High),
                        developer_instructions: None,
                    },
                })
            );
        }
        other => panic!("expected restored draft submission, got {other:?}"),
    }
}

#[tokio::test]
async fn replay_thread_snapshot_restores_collaboration_mode_without_input() {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    app.chat_widget.handle_thread_session(session.clone());
    app.chat_widget
        .set_reasoning_effort(Some(ReasoningEffortConfig::High));
    app.chat_widget
        .set_collaboration_mask(CollaborationModeMask {
            name: "Plan".to_string(),
            mode: Some(ModeKind::Plan),
            model: Some("gpt-restored".to_string()),
            reasoning_effort: Some(Some(ReasoningEffortConfig::High)),
            developer_instructions: None,
        });
    let input_state = app
        .chat_widget
        .capture_thread_input_state()
        .expect("expected collaboration-only input state");

    let (chat_widget, _app_event_tx, _rx, _new_op_rx) = make_chatwidget_manual_with_sender().await;
    app.chat_widget = chat_widget;
    app.chat_widget.handle_thread_session(session.clone());
    app.chat_widget
        .set_reasoning_effort(Some(ReasoningEffortConfig::Low));
    app.chat_widget
        .set_collaboration_mask(CollaborationModeMask {
            name: "Default".to_string(),
            mode: Some(ModeKind::Default),
            model: Some("gpt-replacement".to_string()),
            reasoning_effort: Some(Some(ReasoningEffortConfig::Low)),
            developer_instructions: None,
        });

    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: None,
            turns: Vec::new(),
            events: vec![],
            input_state: Some(input_state),
        },
        /*resume_restored_queue*/ true,
    );

    assert_eq!(
        app.chat_widget.active_collaboration_mode_kind(),
        ModeKind::Plan
    );
    assert_eq!(app.chat_widget.current_model(), "gpt-restored");
    assert_eq!(
        app.chat_widget.current_reasoning_effort(),
        Some(ReasoningEffortConfig::High)
    );
}

#[tokio::test]
async fn replayed_interrupted_turn_restores_queued_input_to_composer() {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let session = test_thread_session(thread_id, test_path_buf("/tmp/project"));
    app.chat_widget.handle_thread_session(session.clone());
    app.chat_widget.handle_server_notification(
        turn_started_notification(thread_id, "turn-1"),
        /*replay_kind*/ None,
    );
    app.chat_widget.handle_server_notification(
        agent_message_delta_notification(thread_id, "turn-1", "agent-1", "streaming"),
        /*replay_kind*/ None,
    );
    app.chat_widget
        .apply_external_edit("queued follow-up".to_string());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let input_state = app
        .chat_widget
        .capture_thread_input_state()
        .expect("expected queued follow-up state");

    let (chat_widget, _app_event_tx, _rx, mut new_op_rx) =
        make_chatwidget_manual_with_sender().await;
    app.chat_widget = chat_widget;
    app.chat_widget.handle_thread_session(session.clone());
    while new_op_rx.try_recv().is_ok() {}

    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: None,
            turns: Vec::new(),
            events: vec![ThreadBufferedEvent::Notification(
                turn_completed_notification(thread_id, "turn-1", TurnStatus::Interrupted),
            )],
            input_state: Some(input_state),
        },
        /*resume_restored_queue*/ true,
    );

    assert_eq!(
        app.chat_widget.composer_text_with_pending(),
        "queued follow-up"
    );
    assert!(app.chat_widget.queued_user_message_texts().is_empty());
    assert!(
        new_op_rx.try_recv().is_err(),
        "replayed interrupted turns should restore queued input for editing, not submit it"
    );
}
