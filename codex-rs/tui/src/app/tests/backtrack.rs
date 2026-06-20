use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn backtrack_selection_with_duplicate_history_targets_unique_turn() {
    let (mut app, _app_event_rx, mut op_rx) = make_test_app_with_channels().await;

    let user_cell = |text: &str,
                     text_elements: Vec<TextElement>,
                     local_image_paths: Vec<PathBuf>,
                     remote_image_urls: Vec<String>|
     -> Arc<dyn HistoryCell> {
        Arc::new(UserHistoryCell {
            message: text.to_string(),
            text_elements,
            local_image_paths,
            remote_image_urls,
        }) as Arc<dyn HistoryCell>
    };
    let agent_cell = |text: &str| -> Arc<dyn HistoryCell> {
        Arc::new(AgentMessageCell::new(
            vec![Line::from(text.to_string())],
            /*is_first_line*/ true,
        )) as Arc<dyn HistoryCell>
    };

    let make_header = |is_first| {
        let session = ThreadSessionState {
            thread_id: ThreadId::new(),
            forked_from_id: None,
            fork_parent_title: None,
            thread_name: None,
            model: "gpt-test".to_string(),
            model_provider_id: "test-provider".to_string(),
            service_tier: None,
            approval_policy: AskForApproval::Never.to_core(),
            approvals_reviewer: ApprovalsReviewer::User,
            permission_profile: PermissionProfile::read_only(),
            active_permission_profile: None,
            cwd: test_path_buf("/home/user/project").abs(),
            runtime_workspace_roots: Vec::new(),
            instruction_source_paths: Vec::new(),
            reasoning_effort: None,
            collaboration_mode: None,
            personality: None,
            message_history: None,
            network_proxy: None,
            rollout_path: Some(PathBuf::new()),
        };
        Arc::new(new_session_info(
            &app.chat_widget.config_ref().cwd,
            app.chat_widget.config_ref().show_tooltips,
            app.chat_widget.current_model(),
            &session,
            is_first,
            /*tooltip_override*/ None,
            /*show_fast_status*/ false,
        )) as Arc<dyn HistoryCell>
    };

    let placeholder = "[Image #1]";
    let edited_text = format!("follow-up (edited) {placeholder}");
    let edited_range = edited_text.len().saturating_sub(placeholder.len())..edited_text.len();
    let edited_text_elements = vec![TextElement::new(
        edited_range.into(),
        /*placeholder*/ None,
    )];
    let edited_local_image_paths = vec![PathBuf::from("/tmp/fake-image.png")];

    // Simulate a transcript with duplicated history (e.g., from prior backtracks)
    // and an edited turn appended after a session header boundary.
    app.transcript_cells = vec![
        make_header(true),
        user_cell("first question", Vec::new(), Vec::new(), Vec::new()),
        agent_cell("answer first"),
        user_cell("follow-up", Vec::new(), Vec::new(), Vec::new()),
        agent_cell("answer follow-up"),
        make_header(false),
        user_cell("first question", Vec::new(), Vec::new(), Vec::new()),
        agent_cell("answer first"),
        user_cell(
            &edited_text,
            edited_text_elements.clone(),
            edited_local_image_paths.clone(),
            vec!["https://example.com/backtrack.png".to_string()],
        ),
        agent_cell("answer edited"),
    ];

    assert_eq!(user_count(&app.transcript_cells), 2);

    let base_id = ThreadId::new();
    app.chat_widget
        .handle_thread_session(crate::session_state::ThreadSessionState {
            thread_id: base_id,
            forked_from_id: None,
            fork_parent_title: None,
            thread_name: None,
            model: "gpt-test".to_string(),
            model_provider_id: "test-provider".to_string(),
            service_tier: None,
            approval_policy: AskForApproval::Never.to_core(),
            approvals_reviewer: ApprovalsReviewer::User,
            permission_profile: PermissionProfile::read_only(),
            active_permission_profile: None,
            cwd: test_path_buf("/home/user/project").abs(),
            runtime_workspace_roots: Vec::new(),
            instruction_source_paths: Vec::new(),
            reasoning_effort: None,
            collaboration_mode: None,
            personality: None,
            message_history: None,
            network_proxy: None,
            rollout_path: Some(PathBuf::new()),
        });

    app.backtrack.base_id = Some(base_id);
    app.backtrack.primed = true;
    app.backtrack.nth_user_message = user_count(&app.transcript_cells).saturating_sub(1);

    let selection = app
        .confirm_backtrack_from_main()
        .expect("backtrack selection");
    assert_eq!(selection.nth_user_message, 1);
    assert_eq!(selection.prefill, edited_text);
    assert_eq!(selection.text_elements, edited_text_elements);
    assert_eq!(selection.local_image_paths, edited_local_image_paths);
    assert_eq!(
        selection.remote_image_urls,
        vec!["https://example.com/backtrack.png".to_string()]
    );

    app.apply_backtrack_rollback(selection);
    assert_eq!(
        app.chat_widget.remote_image_urls(),
        vec!["https://example.com/backtrack.png".to_string()]
    );

    let mut rollback_turns = None;
    while let Ok(op) = op_rx.try_recv() {
        if let Op::ThreadRollback { num_turns } = op {
            rollback_turns = Some(num_turns);
        }
    }

    assert_eq!(rollback_turns, Some(1));
}

#[tokio::test]
async fn backtrack_remote_image_only_selection_clears_existing_composer_draft() {
    let (mut app, _app_event_rx, mut op_rx) = make_test_app_with_channels().await;

    app.transcript_cells = vec![Arc::new(UserHistoryCell {
        message: "original".to_string(),
        text_elements: Vec::new(),
        local_image_paths: Vec::new(),
        remote_image_urls: Vec::new(),
    }) as Arc<dyn HistoryCell>];
    app.chat_widget
        .set_composer_text("stale draft".to_string(), Vec::new(), Vec::new());

    let remote_image_url = "https://example.com/remote-only.png".to_string();
    app.apply_backtrack_rollback(BacktrackSelection {
        nth_user_message: 0,
        prefill: String::new(),
        text_elements: Vec::new(),
        local_image_paths: Vec::new(),
        remote_image_urls: vec![remote_image_url.clone()],
    });

    assert_eq!(app.chat_widget.composer_text_with_pending(), "");
    assert_eq!(app.chat_widget.remote_image_urls(), vec![remote_image_url]);

    let mut rollback_turns = None;
    while let Ok(op) = op_rx.try_recv() {
        if let Op::ThreadRollback { num_turns } = op {
            rollback_turns = Some(num_turns);
        }
    }
    assert_eq!(rollback_turns, Some(1));
}

#[tokio::test]
async fn backtrack_resubmit_preserves_data_image_urls_in_user_turn() {
    let (mut app, _app_event_rx, mut op_rx) = make_test_app_with_channels().await;

    let thread_id = ThreadId::new();
    app.chat_widget
        .handle_thread_session(crate::session_state::ThreadSessionState {
            thread_id,
            forked_from_id: None,
            fork_parent_title: None,
            thread_name: None,
            model: "gpt-test".to_string(),
            model_provider_id: "test-provider".to_string(),
            service_tier: None,
            approval_policy: AskForApproval::Never.to_core(),
            approvals_reviewer: ApprovalsReviewer::User,
            permission_profile: PermissionProfile::read_only(),
            active_permission_profile: None,
            cwd: test_path_buf("/home/user/project").abs(),
            runtime_workspace_roots: Vec::new(),
            instruction_source_paths: Vec::new(),
            reasoning_effort: None,
            collaboration_mode: None,
            personality: None,
            message_history: None,
            network_proxy: None,
            rollout_path: Some(PathBuf::new()),
        });

    let data_image_url = "data:image/png;base64,abc123".to_string();
    app.transcript_cells = vec![Arc::new(UserHistoryCell {
        message: "please inspect this".to_string(),
        text_elements: Vec::new(),
        local_image_paths: Vec::new(),
        remote_image_urls: vec![data_image_url.clone()],
    }) as Arc<dyn HistoryCell>];

    app.apply_backtrack_rollback(BacktrackSelection {
        nth_user_message: 0,
        prefill: "please inspect this".to_string(),
        text_elements: Vec::new(),
        local_image_paths: Vec::new(),
        remote_image_urls: vec![data_image_url.clone()],
    });

    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let mut saw_rollback = false;
    let mut submitted_items: Option<Vec<UserInput>> = None;
    while let Ok(op) = op_rx.try_recv() {
        match op {
            Op::ThreadRollback { .. } => saw_rollback = true,
            Op::UserTurn { items, .. } => submitted_items = Some(items),
            _ => {}
        }
    }

    assert!(saw_rollback);
    let items = submitted_items.expect("expected user turn after backtrack resubmit");
    assert!(items.iter().any(|item| {
        matches!(
            item,
            UserInput::Image { url, .. } if url == &data_image_url
        )
    }));
}

#[tokio::test]
async fn queued_rollback_syncs_overlay_and_clears_deferred_history() {
    let mut app = make_test_app().await;
    app.transcript_cells = vec![
        Arc::new(UserHistoryCell {
            message: "first".to_string(),
            text_elements: Vec::new(),
            local_image_paths: Vec::new(),
            remote_image_urls: Vec::new(),
        }) as Arc<dyn HistoryCell>,
        Arc::new(AgentMessageCell::new(
            vec![Line::from("after first")],
            /*is_first_line*/ false,
        )) as Arc<dyn HistoryCell>,
        Arc::new(UserHistoryCell {
            message: "second".to_string(),
            text_elements: Vec::new(),
            local_image_paths: Vec::new(),
            remote_image_urls: Vec::new(),
        }) as Arc<dyn HistoryCell>,
        Arc::new(AgentMessageCell::new(
            vec![Line::from("after second")],
            /*is_first_line*/ false,
        )) as Arc<dyn HistoryCell>,
    ];
    app.overlay = Some(Overlay::new_transcript(
        app.transcript_cells.clone(),
        app.keymap.pager.clone(),
    ));
    app.deferred_history_lines = vec![Line::from("stale buffered line")];
    app.backtrack.overlay_preview_active = true;
    app.backtrack.nth_user_message = 1;

    let changed = app.apply_non_pending_thread_rollback(/*num_turns*/ 1);

    assert!(changed);
    assert!(app.backtrack_render_pending);
    assert!(app.deferred_history_lines.is_empty());
    assert_eq!(app.backtrack.nth_user_message, 0);
    let user_messages: Vec<String> = app
        .transcript_cells
        .iter()
        .filter_map(|cell| {
            cell.as_any()
                .downcast_ref::<UserHistoryCell>()
                .map(|cell| cell.message.clone())
        })
        .collect();
    assert_eq!(user_messages, vec!["first".to_string()]);
    let overlay_cell_count = match app.overlay.as_ref() {
        Some(Overlay::Transcript(t)) => t.committed_cell_count(),
        _ => panic!("expected transcript overlay"),
    };
    assert_eq!(overlay_cell_count, app.transcript_cells.len());
}

#[tokio::test]
async fn thread_rollback_response_discards_queued_active_thread_events() {
    let mut app = make_test_app().await;
    let thread_id = ThreadId::new();
    let (tx, rx) = mpsc::channel(8);
    app.active_thread_id = Some(thread_id);
    app.active_thread_rx = Some(rx);
    tx.send(ThreadBufferedEvent::Notification(
        ServerNotification::ConfigWarning(ConfigWarningNotification {
            summary: "stale warning".to_string(),
            details: None,
            path: None,
            range: None,
        }),
    ))
    .await
    .expect("event should queue");

    app.handle_thread_rollback_response(
        thread_id,
        /*num_turns*/ 1,
        &ThreadRollbackResponse {
            thread: Thread {
                id: thread_id.to_string(),
                session_id: thread_id.to_string(),
                forked_from_id: None,
                preview: String::new(),
                ephemeral: false,
                model_provider: "openai".to_string(),
                created_at: 0,
                updated_at: 0,
                status: codex_app_server_protocol::ThreadStatus::Idle,
                path: None,
                cwd: test_path_buf("/tmp/project").abs(),
                cli_version: "0.0.0".to_string(),
                source: SessionSource::Cli,
                thread_source: None,
                agent_nickname: None,
                agent_role: None,
                git_info: None,
                name: None,
                turns: Vec::new(),
            },
        },
    )
    .await;

    let rx = app
        .active_thread_rx
        .as_mut()
        .expect("active receiver should remain attached");
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn backtrack_esc_does_not_steal_empty_vim_insert_escape() {
    let mut app = make_test_app().await;
    let esc = crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Esc, KeyModifiers::NONE);

    assert!(app.chat_widget.composer_is_empty());
    assert!(app.should_handle_backtrack_esc(esc));

    app.chat_widget.toggle_vim_mode_and_notify();
    assert!(app.should_handle_backtrack_esc(esc));

    app.chat_widget
        .handle_key_event(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('i'),
            KeyModifiers::NONE,
        ));
    assert!(app.chat_widget.should_handle_vim_insert_escape(esc));
    assert!(!app.should_handle_backtrack_esc(esc));

    app.chat_widget.handle_key_event(esc);

    assert!(!app.backtrack.primed);
    assert!(!app.chat_widget.should_handle_vim_insert_escape(esc));
    assert!(app.should_handle_backtrack_esc(esc));
}
