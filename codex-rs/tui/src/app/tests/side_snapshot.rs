use super::*;

#[tokio::test]
async fn side_thread_snapshot_hides_forked_parent_transcript() {
    let parent_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    let mut store = ThreadEventStore::new(/*capacity*/ 4);
    let session = ThreadSessionState {
        forked_from_id: Some(parent_thread_id),
        fork_parent_title: None,
        ..test_thread_session(side_thread_id, test_path_buf("/tmp/side"))
    };
    let parent_turn = test_turn(
        "parent-turn",
        TurnStatus::Completed,
        vec![ThreadItem::UserMessage {
            id: "parent-user".to_string(),
            content: vec![AppServerUserInput::Text {
                text: "parent prompt should stay hidden".to_string(),
                text_elements: Vec::new(),
            }],
        }],
    );

    App::install_side_thread_snapshot(&mut store, session, vec![parent_turn]);

    let stored_session = store.session.as_ref().expect("side session");
    assert_eq!(stored_session.thread_id, side_thread_id);
    assert_eq!(stored_session.forked_from_id, None);
    assert_eq!(store.turns, Vec::<Turn>::new());
    assert_eq!(store.active_turn_id(), None);
}


#[tokio::test]
async fn side_thread_snapshot_does_not_refresh_from_fork_history() {
    let mut app = make_test_app().await;
    let parent_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));

    let snapshot = ThreadEventSnapshot {
        session: Some(ThreadSessionState {
            rollout_path: None,
            ..test_thread_session(side_thread_id, test_path_buf("/tmp/side"))
        }),
        turns: Vec::new(),
        events: Vec::new(),
        input_state: None,
    };

    assert!(!app.should_refresh_snapshot_session(
        side_thread_id,
        /*is_replay_only*/ false,
        &snapshot
    ));
}


#[tokio::test]
async fn side_thread_snapshot_skips_session_header_preamble() {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    while app_event_rx.try_recv().is_ok() {}

    let parent_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    app.primary_thread_id = Some(parent_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));

    let snapshot = ThreadEventSnapshot {
        session: Some(ThreadSessionState {
            forked_from_id: Some(parent_thread_id),
            fork_parent_title: None,
            ..test_thread_session(side_thread_id, test_path_buf("/tmp/side"))
        }),
        turns: Vec::new(),
        events: Vec::new(),
        input_state: None,
    };

    app.replay_thread_snapshot(snapshot, /*resume_restored_queue*/ false);

    let mut rendered_cells = Vec::new();
    while let Ok(event) = app_event_rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            rendered_cells.push(lines_to_single_string(&cell.display_lines(/*width*/ 120)));
        }
    }
    assert_eq!(app.chat_widget.thread_id(), Some(side_thread_id));
    assert_eq!(rendered_cells, Vec::<String>::new());
    assert_eq!(
        app.chat_widget.active_cell_transcript_lines(/*width*/ 120),
        None
    );
}


#[tokio::test]
async fn side_thread_ignores_global_mcp_startup_notifications() {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    while app_event_rx.try_recv().is_ok() {}
    let app_server = crate::start_embedded_app_server_for_picker(app.chat_widget.config_ref())
        .await
        .expect("embedded app server");
    let parent_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    app.primary_thread_id = Some(parent_thread_id);
    app.active_thread_id = Some(side_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));
    app.sync_side_thread_ui();

    app.handle_app_server_event(
        &app_server,
        codex_app_server_client::AppServerEvent::ServerNotification(
            ServerNotification::McpServerStatusUpdated(McpServerStatusUpdatedNotification {
                name: "sentry".to_string(),
                status: McpServerStartupState::Failed,
                error: Some("sentry is not logged in".to_string()),
            }),
        ),
    )
    .await;

    assert!(app_event_rx.try_recv().is_err());
}


#[tokio::test]
async fn side_restore_user_message_puts_inline_question_back_in_composer() {
    let mut app = make_test_app().await;
    let user_message = crate::chatwidget::UserMessage::from("side question");

    app.restore_side_user_message(Some(user_message));

    assert_eq!(
        app.chat_widget.composer_text_with_pending(),
        "side question"
    );
}


#[tokio::test]
async fn side_discard_selection_keeps_current_side_thread() {
    let mut app = make_test_app().await;
    let parent_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    app.active_thread_id = Some(side_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));

    assert_eq!(
        app.side_thread_to_discard_after_switch(side_thread_id),
        None
    );
    assert_eq!(
        app.side_thread_to_discard_after_switch(parent_thread_id),
        Some(side_thread_id)
    );
}


#[tokio::test]
async fn discard_side_thread_removes_agent_navigation_entry() -> Result<()> {
    Box::pin(async {
        let mut app = make_test_app().await;
        let mut app_server =
            crate::start_embedded_app_server_for_picker(app.chat_widget.config_ref()).await?;
        let mut side_config = app.chat_widget.config_ref().clone();
        side_config.ephemeral = true;
        let started = app_server.start_thread(&side_config).await?;
        let side_thread_id = started.session.thread_id;
        app.side_threads
            .insert(side_thread_id, SideThreadState::new(ThreadId::new()));
        app.agent_navigation.upsert(
            side_thread_id,
            Some("Side".to_string()),
            Some("side".to_string()),
            /*is_closed*/ false,
        );

        assert!(
            app.discard_side_thread(&mut app_server, side_thread_id)
                .await
        );

        assert_eq!(app.agent_navigation.get(&side_thread_id), None);
        assert!(!app.side_threads.contains_key(&side_thread_id));
        Ok(())
    })
    .await
}


#[tokio::test]
async fn discard_side_thread_keeps_local_state_when_server_close_fails() -> Result<()> {
    Box::pin(async {
        let mut app = make_test_app().await;
        let mut app_server =
            crate::start_embedded_app_server_for_picker(app.chat_widget.config_ref()).await?;
        let parent_thread_id = ThreadId::new();
        let side_thread_id = ThreadId::new();
        app.active_thread_id = Some(side_thread_id);
        app.side_threads
            .insert(side_thread_id, SideThreadState::new(parent_thread_id));
        app.agent_navigation.upsert(
            side_thread_id,
            Some("Side".to_string()),
            Some("side".to_string()),
            /*is_closed*/ false,
        );

        assert!(
            !app.discard_side_thread(&mut app_server, side_thread_id)
                .await
        );

        assert_eq!(app.active_thread_id, Some(side_thread_id));
        assert_eq!(
            app.side_threads
                .get(&side_thread_id)
                .map(|state| state.parent_thread_id),
            Some(parent_thread_id)
        );
        assert!(app.agent_navigation.get(&side_thread_id).is_some());
        Ok(())
    })
    .await
}


#[tokio::test]
async fn discard_closed_side_thread_removes_local_state_without_server_rpc() {
    let mut app = make_test_app().await;
    let parent_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    app.active_thread_id = Some(side_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));
    app.thread_event_channels
        .insert(side_thread_id, ThreadEventChannel::new(/*capacity*/ 4));
    app.agent_navigation.upsert(
        side_thread_id,
        Some("Side".to_string()),
        Some("side".to_string()),
        /*is_closed*/ false,
    );

    app.discard_closed_side_thread(side_thread_id).await;

    assert_eq!(app.active_thread_id, None);
    assert!(!app.side_threads.contains_key(&side_thread_id));
    assert!(!app.thread_event_channels.contains_key(&side_thread_id));
    assert_eq!(app.agent_navigation.get(&side_thread_id), None);
}


#[tokio::test]
async fn side_conversations_reject_backtrack_esc_without_stealing_vim_insert_escape() {
    let mut app = make_test_app().await;
    let esc = crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Esc, KeyModifiers::NONE);

    app.chat_widget
        .set_side_conversation_active(/*active*/ true);
    assert!(app.chat_widget.composer_is_empty());
    assert!(!app.should_handle_backtrack_esc(esc));
    assert!(app.should_reject_side_backtrack_esc(esc));

    app.chat_widget.toggle_vim_mode_and_notify();
    app.chat_widget
        .handle_key_event(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('i'),
            KeyModifiers::NONE,
        ));

    assert!(app.chat_widget.should_handle_vim_insert_escape(esc));
    assert!(!app.should_handle_backtrack_esc(esc));
    assert!(!app.should_reject_side_backtrack_esc(esc));
}


#[tokio::test]
async fn side_backtrack_rejection_reports_unavailable_message_snapshot() {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    app.backtrack.primed = true;

    app.reject_side_backtrack_esc();

    assert!(!app.backtrack.primed);
    let cell = match app_event_rx.try_recv() {
        Ok(AppEvent::InsertHistoryCell(cell)) => cell,
        other => panic!("expected InsertHistoryCell event, got {other:?}"),
    };
    let rendered = cell
        .display_lines(/*width*/ 80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert_app_snapshot!(
        "side_backtrack_rejection_reports_unavailable_message",
        rendered
    );
}
