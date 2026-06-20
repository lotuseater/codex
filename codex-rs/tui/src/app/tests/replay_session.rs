use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn enqueue_primary_thread_session_replays_buffered_approval_after_attach() -> Result<()> {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let approval_request =
        exec_approval_request(thread_id, "turn-1", "call-1", /*approval_id*/ None);

    assert_eq!(
        app.pending_app_server_requests
            .note_server_request(&approval_request),
        None
    );
    app.enqueue_primary_thread_request(approval_request).await?;
    app.enqueue_primary_thread_session(
        test_thread_session(thread_id, test_path_buf("/tmp/project")),
        Vec::new(),
    )
    .await?;

    let rx = app
        .active_thread_rx
        .as_mut()
        .expect("primary thread receiver should be active");
    let event = time::timeout(Duration::from_millis(50), rx.recv())
        .await
        .expect("timed out waiting for buffered approval event")
        .expect("channel closed unexpectedly");

    assert!(matches!(
        &event,
        ThreadBufferedEvent::Request(ServerRequest::CommandExecutionRequestApproval {
            params,
            ..
        }) if params.turn_id == "turn-1"
    ));

    app.handle_thread_event_now(event);
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    while let Ok(app_event) = app_event_rx.try_recv() {
        if let AppEvent::SubmitThreadOp {
            thread_id: op_thread_id,
            ..
        } = app_event
        {
            assert_eq!(op_thread_id, thread_id);
            return Ok(());
        }
    }

    panic!("expected approval action to submit a thread-scoped op");
}

#[tokio::test]
async fn resolved_buffered_approval_does_not_become_actionable_after_drain() -> Result<()> {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let approval_request =
        exec_approval_request(thread_id, "turn-1", "call-1", /*approval_id*/ None);

    app.enqueue_primary_thread_session(
        test_thread_session(thread_id, test_path_buf("/tmp/project")),
        Vec::new(),
    )
    .await?;
    while app_event_rx.try_recv().is_ok() {}

    assert_eq!(
        app.pending_app_server_requests
            .note_server_request(&approval_request),
        None
    );
    app.enqueue_thread_request(thread_id, approval_request)
        .await?;

    let resolved = app
        .pending_app_server_requests
        .resolve_notification(&AppServerRequestId::Integer(1))
        .expect("matching app-server request should resolve");
    app.chat_widget.dismiss_app_server_request(&resolved);
    while app_event_rx.try_recv().is_ok() {}

    let rx = app
        .active_thread_rx
        .as_mut()
        .expect("primary thread receiver should be active");
    let event = time::timeout(Duration::from_millis(50), rx.recv())
        .await
        .expect("timed out waiting for buffered approval event")
        .expect("channel closed unexpectedly");

    assert!(matches!(
        &event,
        ThreadBufferedEvent::Request(ServerRequest::CommandExecutionRequestApproval {
            params,
            ..
        }) if params.turn_id == "turn-1"
    ));

    app.handle_thread_event_now(event);
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    while let Ok(app_event) = app_event_rx.try_recv() {
        assert!(
            !matches!(app_event, AppEvent::SubmitThreadOp { .. }),
            "resolved buffered approval should not become actionable"
        );
    }

    Ok(())
}

#[tokio::test]
async fn enqueue_primary_thread_session_replays_turns_before_initial_prompt_submit() -> Result<()> {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let initial_prompt = "follow-up after replay".to_string();
    let config = app.config.clone();
    let model = crate::legacy_core::test_support::get_model_offline(config.model.as_deref());
    app.chat_widget = ChatWidget::new_with_app_event(ChatWidgetInit {
        config,
        frame_requester: crate::tui::FrameRequester::test_dummy(),
        app_event_tx: app.app_event_tx.clone(),
        workspace_command_runner: None,
        initial_user_message: create_initial_user_message(
            Some(initial_prompt.clone()),
            Vec::new(),
            Vec::new(),
        ),
        enhanced_keys_supported: false,
        has_chatgpt_account: false,
        model_catalog: app.model_catalog.clone(),
        feedback: codex_feedback::CodexFeedback::new(),
        is_first_run: false,
        status_account_display: None,
        runtime_model_provider_base_url: None,
        initial_plan_type: None,
        model: Some(model),
        startup_tooltip_override: None,
        status_line_invalid_items_warned: app.status_line_invalid_items_warned.clone(),
        terminal_title_invalid_items_warned: app.terminal_title_invalid_items_warned.clone(),
        session_telemetry: app.session_telemetry.clone(),
    });

    app.enqueue_primary_thread_session(
        test_thread_session(thread_id, test_path_buf("/tmp/project")),
        vec![test_turn(
            "turn-1",
            TurnStatus::Completed,
            vec![ThreadItem::UserMessage {
                id: "user-1".to_string(),
                content: vec![AppServerUserInput::Text {
                    text: "earlier prompt".to_string(),
                    text_elements: Vec::new(),
                }],
            }],
        )],
    )
    .await?;

    let mut saw_replayed_answer = false;
    let mut submitted_items = None;
    while let Ok(event) = app_event_rx.try_recv() {
        match event {
            AppEvent::InsertHistoryCell(cell) => {
                let transcript = lines_to_single_string(&cell.transcript_lines(/*width*/ 80));
                saw_replayed_answer |= transcript.contains("earlier prompt");
            }
            AppEvent::SubmitThreadOp {
                thread_id: op_thread_id,
                op: Op::UserTurn { items, .. },
            } => {
                assert_eq!(op_thread_id, thread_id);
                submitted_items = Some(items);
            }
            AppEvent::CodexOp(Op::UserTurn { items, .. }) => {
                submitted_items = Some(items);
            }
            _ => {}
        }
    }
    assert!(
        saw_replayed_answer,
        "expected replayed history before initial prompt submit"
    );
    assert_eq!(
        submitted_items,
        Some(vec![UserInput::Text {
            text: initial_prompt,
            text_elements: Vec::new(),
        }])
    );

    Ok(())
}

#[tokio::test]
async fn reset_thread_event_state_aborts_listener_tasks() {
    struct NotifyOnDrop(Option<tokio::sync::oneshot::Sender<()>>);

    impl Drop for NotifyOnDrop {
        fn drop(&mut self) {
            if let Some(tx) = self.0.take() {
                let _ = tx.send(());
            }
        }
    }

    let mut app = make_test_app().await;
    let thread_id = ThreadId::new();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (dropped_tx, dropped_rx) = tokio::sync::oneshot::channel();
    let handle = tokio::spawn(async move {
        let _notify_on_drop = NotifyOnDrop(Some(dropped_tx));
        let _ = started_tx.send(());
        std::future::pending::<()>().await;
    });
    app.thread_event_listener_tasks.insert(thread_id, handle);
    started_rx
        .await
        .expect("listener task should report it started");

    app.reset_thread_event_state();

    assert_eq!(app.thread_event_listener_tasks.is_empty(), true);
    time::timeout(Duration::from_millis(50), dropped_rx)
        .await
        .expect("timed out waiting for listener task abort")
        .expect("listener task drop notification should succeed");
}

#[tokio::test]
async fn history_lookup_response_is_routed_to_requesting_thread() -> Result<()> {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();

    app.lookup_message_history_entry(thread_id, /*offset*/ 0, /*log_id*/ 1)
        .await?;

    let app_event = tokio::time::timeout(Duration::from_secs(1), app_event_rx.recv())
        .await
        .expect("history lookup should emit an app event")
        .expect("app event channel should stay open");

    let AppEvent::ThreadHistoryEntryResponse {
        thread_id: routed_thread_id,
        event,
    } = app_event
    else {
        panic!("expected thread-routed history response");
    };
    assert_eq!(routed_thread_id, thread_id);
    assert_eq!(event.offset, 0);
    assert_eq!(event.log_id, 1);
    assert!(event.entry.is_none());

    Ok(())
}

#[tokio::test]
async fn enqueue_thread_event_does_not_block_when_channel_full() -> Result<()> {
    let mut app = make_test_app().await;
    let thread_id = ThreadId::new();
    app.thread_event_channels
        .insert(thread_id, ThreadEventChannel::new(/*capacity*/ 1));
    app.set_thread_active(thread_id, /*active*/ true).await;

    let event = thread_closed_notification(thread_id);

    app.enqueue_thread_notification(thread_id, event.clone())
        .await?;
    time::timeout(
        Duration::from_millis(50),
        app.enqueue_thread_notification(thread_id, event),
    )
    .await
    .expect("enqueue_thread_notification blocked on a full channel")?;

    let mut rx = app
        .thread_event_channels
        .get_mut(&thread_id)
        .expect("missing thread channel")
        .receiver
        .take()
        .expect("missing receiver");

    time::timeout(Duration::from_millis(50), rx.recv())
        .await
        .expect("timed out waiting for first event")
        .expect("channel closed unexpectedly");
    time::timeout(Duration::from_millis(50), rx.recv())
        .await
        .expect("timed out waiting for second event")
        .expect("channel closed unexpectedly");

    Ok(())
}

#[tokio::test]
async fn replay_thread_snapshot_replays_turn_history_in_order() {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: Some(test_thread_session(
                thread_id,
                test_path_buf("/home/user/project"),
            )),
            turns: vec![
                Turn {
                    id: "turn-1".to_string(),
                    items_view: codex_app_server_protocol::TurnItemsView::Full,
                    items: vec![ThreadItem::UserMessage {
                        id: "user-1".to_string(),
                        content: vec![AppServerUserInput::Text {
                            text: "first prompt".to_string(),
                            text_elements: Vec::new(),
                        }],
                    }],
                    status: TurnStatus::Completed,
                    error: None,
                    started_at: None,
                    completed_at: None,
                    duration_ms: None,
                },
                Turn {
                    id: "turn-2".to_string(),
                    items_view: codex_app_server_protocol::TurnItemsView::Full,
                    items: vec![
                        ThreadItem::UserMessage {
                            id: "user-2".to_string(),
                            content: vec![AppServerUserInput::Text {
                                text: "third prompt".to_string(),
                                text_elements: Vec::new(),
                            }],
                        },
                        ThreadItem::AgentMessage {
                            id: "assistant-2".to_string(),
                            text: "done".to_string(),
                            phase: None,
                            memory_citation: None,
                        },
                    ],
                    status: TurnStatus::Completed,
                    error: None,
                    started_at: None,
                    completed_at: None,
                    duration_ms: None,
                },
            ],
            events: Vec::new(),
            input_state: None,
        },
        /*resume_restored_queue*/ false,
    );

    while let Ok(event) = app_event_rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let cell: Arc<dyn HistoryCell> = cell.into();
            app.transcript_cells.push(cell);
        }
    }

    let user_messages: Vec<String> = app
        .transcript_cells
        .iter()
        .filter_map(|cell| {
            cell.as_any()
                .downcast_ref::<UserHistoryCell>()
                .map(|cell| cell.message.clone())
        })
        .collect();
    assert_eq!(
        user_messages,
        vec!["first prompt".to_string(), "third prompt".to_string()]
    );
}

#[tokio::test]
async fn replace_chat_widget_reseeds_collab_agent_metadata_for_replay() {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let receiver_thread_id =
        ThreadId::from_string("019cff70-2599-75e2-af72-b958ce5dc1cc").expect("valid thread");
    app.agent_navigation.upsert(
        receiver_thread_id,
        Some("Robie".to_string()),
        Some("explorer".to_string()),
        /*is_closed*/ false,
    );

    let replacement = ChatWidget::new_with_app_event(ChatWidgetInit {
        config: app.config.clone(),
        frame_requester: crate::tui::FrameRequester::test_dummy(),
        app_event_tx: app.app_event_tx.clone(),
        workspace_command_runner: None,
        initial_user_message: None,
        enhanced_keys_supported: app.enhanced_keys_supported,
        has_chatgpt_account: app.chat_widget.has_chatgpt_account(),
        model_catalog: app.model_catalog.clone(),
        feedback: app.feedback.clone(),
        is_first_run: false,
        status_account_display: app.chat_widget.status_account_display().cloned(),
        runtime_model_provider_base_url: app
            .chat_widget
            .runtime_model_provider_base_url()
            .map(str::to_string),
        initial_plan_type: app.chat_widget.current_plan_type(),
        model: Some(app.chat_widget.current_model().to_string()),
        startup_tooltip_override: None,
        status_line_invalid_items_warned: app.status_line_invalid_items_warned.clone(),
        terminal_title_invalid_items_warned: app.terminal_title_invalid_items_warned.clone(),
        session_telemetry: app.session_telemetry.clone(),
    });
    app.replace_chat_widget(replacement);

    app.replay_thread_snapshot(
        ThreadEventSnapshot {
            session: None,
            turns: Vec::new(),
            events: vec![ThreadBufferedEvent::Notification(
                ServerNotification::ItemStarted(
                    codex_app_server_protocol::ItemStartedNotification {
                        thread_id: "thread-1".to_string(),
                        turn_id: "turn-1".to_string(),
                        started_at_ms: 0,
                        item: ThreadItem::CollabAgentToolCall {
                            id: "wait-1".to_string(),
                            tool: codex_app_server_protocol::CollabAgentTool::Wait,
                            status:
                                codex_app_server_protocol::CollabAgentToolCallStatus::InProgress,
                            sender_thread_id: ThreadId::new().to_string(),
                            receiver_thread_ids: vec![receiver_thread_id.to_string()],
                            prompt: None,
                            model: None,
                            reasoning_effort: None,
                            agents_states: HashMap::new(),
                        },
                    },
                ),
            )],
            input_state: None,
        },
        /*resume_restored_queue*/ false,
    );

    let mut saw_named_wait = false;
    while let Ok(event) = app_event_rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            let transcript = lines_to_single_string(&cell.transcript_lines(/*width*/ 80));
            saw_named_wait |= transcript.contains("Robie [explorer]");
        }
    }

    assert!(
        saw_named_wait,
        "expected replayed wait item to keep agent name"
    );
}

#[tokio::test]
async fn refreshed_snapshot_session_persists_resumed_turns() {
    let mut app = make_test_app().await;
    let thread_id = ThreadId::new();
    let initial_session = test_thread_session(thread_id, test_path_buf("/tmp/original"));
    app.thread_event_channels.insert(
        thread_id,
        ThreadEventChannel::new_with_session(
            /*capacity*/ 4,
            initial_session.clone(),
            Vec::new(),
        ),
    );

    let resumed_turns = vec![test_turn(
        "turn-1",
        TurnStatus::Completed,
        vec![ThreadItem::UserMessage {
            id: "user-1".to_string(),
            content: vec![AppServerUserInput::Text {
                text: "restored prompt".to_string(),
                text_elements: Vec::new(),
            }],
        }],
    )];
    let resumed_session = ThreadSessionState {
        cwd: test_path_buf("/tmp/refreshed").abs(),
        runtime_workspace_roots: Vec::new(),
        instruction_source_paths: Vec::new(),
        ..initial_session.clone()
    };
    let mut snapshot = ThreadEventSnapshot {
        session: Some(initial_session),
        turns: Vec::new(),
        events: Vec::new(),
        input_state: None,
    };

    app.apply_refreshed_snapshot_thread(
        thread_id,
        AppServerStartedThread {
            session: resumed_session.clone(),
            turns: resumed_turns.clone(),
        },
        &mut snapshot,
    )
    .await;

    assert_eq!(snapshot.session, Some(resumed_session.clone()));
    assert_eq!(snapshot.turns, resumed_turns);

    let store = app
        .thread_event_channels
        .get(&thread_id)
        .expect("thread channel")
        .store
        .lock()
        .await;
    let store_snapshot = store.snapshot();
    assert_eq!(store_snapshot.session, Some(resumed_session));
    assert_eq!(store_snapshot.turns, snapshot.turns);
}
