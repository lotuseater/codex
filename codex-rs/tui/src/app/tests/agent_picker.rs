use super::*;

#[tokio::test]
async fn open_agent_picker_keeps_missing_threads_for_replay() -> Result<()> {
    let mut app = Box::pin(make_test_app()).await;
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
        app.chat_widget.config_ref(),
    ))
    .await
    .expect("embedded app server");
    let thread_id = ThreadId::new();
    app.thread_event_channels
        .insert(thread_id, ThreadEventChannel::new(/*capacity*/ 1));

    Box::pin(app.open_agent_picker(&mut app_server)).await;

    assert_eq!(app.thread_event_channels.contains_key(&thread_id), true);
    assert_eq!(
        app.agent_navigation.get(&thread_id),
        Some(&AgentPickerThreadEntry {
            agent_nickname: None,
            agent_role: None,
            is_closed: true,
            model: None,
            reasoning_effort: None,
            token_context_percent_used: None,
        })
    );
    assert_eq!(app.agent_navigation.ordered_thread_ids(), vec![thread_id]);
    Ok(())
}


#[tokio::test]
async fn open_agent_picker_preserves_cached_metadata_for_replay_threads() -> Result<()> {
    let mut app = Box::pin(make_test_app()).await;
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
        app.chat_widget.config_ref(),
    ))
    .await
    .expect("embedded app server");
    let thread_id = ThreadId::new();
    app.thread_event_channels
        .insert(thread_id, ThreadEventChannel::new(/*capacity*/ 1));
    app.agent_navigation.upsert(
        thread_id,
        Some("Robie".to_string()),
        Some("explorer".to_string()),
        /*is_closed*/ true,
    );

    Box::pin(app.open_agent_picker(&mut app_server)).await;

    assert_eq!(app.thread_event_channels.contains_key(&thread_id), true);
    assert_eq!(
        app.agent_navigation.get(&thread_id),
        Some(&AgentPickerThreadEntry {
            agent_nickname: Some("Robie".to_string()),
            agent_role: Some("explorer".to_string()),
            is_closed: true,
            model: None,
            reasoning_effort: None,
            token_context_percent_used: None,
        })
    );
    Ok(())
}


#[tokio::test]
async fn open_agent_picker_prunes_terminal_metadata_only_threads() -> Result<()> {
    let mut app = Box::pin(make_test_app()).await;
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
        app.chat_widget.config_ref(),
    ))
    .await
    .expect("embedded app server");
    let thread_id = ThreadId::new();
    app.agent_navigation.upsert(
        thread_id,
        Some("Ghost".to_string()),
        Some("worker".to_string()),
        /*is_closed*/ false,
    );

    Box::pin(app.open_agent_picker(&mut app_server)).await;

    assert_eq!(app.agent_navigation.get(&thread_id), None);
    assert!(app.agent_navigation.is_empty());
    Ok(())
}


#[tokio::test]
async fn open_agent_picker_marks_terminal_read_errors_closed() -> Result<()> {
    let mut app = Box::pin(make_test_app()).await;
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
        app.chat_widget.config_ref(),
    ))
    .await
    .expect("embedded app server");
    let thread_id = ThreadId::new();
    app.thread_event_channels
        .insert(thread_id, ThreadEventChannel::new(/*capacity*/ 1));
    app.agent_navigation.upsert(
        thread_id,
        Some("Robie".to_string()),
        Some("explorer".to_string()),
        /*is_closed*/ false,
    );

    Box::pin(app.open_agent_picker(&mut app_server)).await;

    assert_eq!(
        app.agent_navigation.get(&thread_id),
        Some(&AgentPickerThreadEntry {
            agent_nickname: Some("Robie".to_string()),
            agent_role: Some("explorer".to_string()),
            is_closed: true,
            model: None,
            reasoning_effort: None,
            token_context_percent_used: None,
        })
    );
    Ok(())
}


#[test]
fn open_agent_picker_marks_loaded_threads_open() -> Result<()> {
    const WORKER_THREADS: usize = 1;
    const TEST_STACK_SIZE_BYTES: usize = 8 * 1024 * 1024;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(WORKER_THREADS)
        .thread_stack_size(TEST_STACK_SIZE_BYTES)
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let mut app = Box::pin(make_test_app()).await;
        let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
            app.chat_widget.config_ref(),
        ))
        .await
        .expect("embedded app server");
        let started = app_server
            .start_thread(app.chat_widget.config_ref())
            .await?;
        let thread_id = started.session.thread_id;
        app.thread_event_channels
            .insert(thread_id, ThreadEventChannel::new(/*capacity*/ 1));

        Box::pin(app.open_agent_picker(&mut app_server)).await;

        assert_eq!(
            app.agent_navigation.get(&thread_id),
            Some(&AgentPickerThreadEntry {
                agent_nickname: None,
                agent_role: None,
                is_closed: false,
                model: None,
                reasoning_effort: None,
                token_context_percent_used: None,
            })
        );
        Ok(())
    })
}


#[test]
fn attach_live_thread_for_selection_rejects_empty_non_ephemeral_fallback_threads() -> Result<()> {
    const WORKER_THREADS: usize = 1;
    const TEST_STACK_SIZE_BYTES: usize = 8 * 1024 * 1024;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(WORKER_THREADS)
        .thread_stack_size(TEST_STACK_SIZE_BYTES)
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let config = {
            let app = make_test_app().await;
            app.chat_widget.config_ref().clone()
        };
        let mut app_server = crate::start_embedded_app_server_for_picker(&config)
            .await
            .expect("embedded app server");
        let started = app_server.start_thread(&config).await?;
        let thread_id = started.session.thread_id;
        let mut app = make_test_app().await;
        app.agent_navigation.upsert(
            thread_id,
            Some("Scout".to_string()),
            Some("worker".to_string()),
            /*is_closed*/ false,
        );

        let err = app
            .attach_live_thread_for_selection(&mut app_server, thread_id)
            .await
            .expect_err("empty fallback should not attach as a blank replay-only thread");

        assert_eq!(
            err.to_string(),
            format!("Agent thread {thread_id} is not yet available for replay or live attach.")
        );
        assert!(!app.thread_event_channels.contains_key(&thread_id));
        Ok(())
    })
}


#[test]
fn attach_live_thread_for_selection_rejects_unmaterialized_fallback_threads() -> Result<()> {
    const WORKER_THREADS: usize = 1;
    const TEST_STACK_SIZE_BYTES: usize = 8 * 1024 * 1024;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(WORKER_THREADS)
        .thread_stack_size(TEST_STACK_SIZE_BYTES)
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let mut app = make_test_app().await;
        let mut app_server =
            crate::start_embedded_app_server_for_picker(app.chat_widget.config_ref()).await?;
        let mut ephemeral_config = app.chat_widget.config_ref().clone();
        ephemeral_config.ephemeral = true;
        let started = app_server.start_thread(&ephemeral_config).await?;
        let thread_id = started.session.thread_id;
        app.agent_navigation.upsert(
            thread_id,
            Some("Scout".to_string()),
            Some("worker".to_string()),
            /*is_closed*/ false,
        );

        let err = app
            .attach_live_thread_for_selection(&mut app_server, thread_id)
            .await
            .expect_err("ephemeral fallback should not attach as a blank live thread");

        assert_eq!(
            err.to_string(),
            format!("Agent thread {thread_id} is not yet available for replay or live attach.")
        );
        assert!(!app.thread_event_channels.contains_key(&thread_id));
        Ok(())
    })
}


#[tokio::test]
async fn should_attach_live_thread_for_selection_skips_closed_metadata_only_threads() {
    let mut app = make_test_app().await;
    let thread_id = ThreadId::new();
    app.agent_navigation.upsert(
        thread_id,
        Some("Ghost".to_string()),
        Some("worker".to_string()),
        /*is_closed*/ true,
    );

    assert!(!app.should_attach_live_thread_for_selection(thread_id));

    app.agent_navigation.upsert(
        thread_id,
        Some("Ghost".to_string()),
        Some("worker".to_string()),
        /*is_closed*/ false,
    );
    assert!(app.should_attach_live_thread_for_selection(thread_id));

    app.thread_event_channels
        .insert(thread_id, ThreadEventChannel::new(/*capacity*/ 1));
    assert!(!app.should_attach_live_thread_for_selection(thread_id));
}


#[tokio::test]
async fn refresh_agent_picker_thread_liveness_prunes_closed_metadata_only_threads() -> Result<()> {
    let mut app = Box::pin(make_test_app()).await;
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
        app.chat_widget.config_ref(),
    ))
    .await
    .expect("embedded app server");
    let thread_id = ThreadId::new();
    app.agent_navigation.upsert(
        thread_id,
        Some("Ghost".to_string()),
        Some("worker".to_string()),
        /*is_closed*/ false,
    );

    let is_available =
        Box::pin(app.refresh_agent_picker_thread_liveness(&mut app_server, thread_id)).await;

    assert!(!is_available);
    assert_eq!(app.agent_navigation.get(&thread_id), None);
    assert!(!app.thread_event_channels.contains_key(&thread_id));
    Ok(())
}


#[tokio::test]
async fn open_agent_picker_prompts_to_enable_multi_agent_when_disabled() -> Result<()> {
    let (mut app, mut app_event_rx, _op_rx) = Box::pin(make_test_app_with_channels()).await;
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
        app.chat_widget.config_ref(),
    ))
    .await
    .expect("embedded app server");
    let _ = app.config.features.disable(Feature::Collab);

    Box::pin(app.open_agent_picker(&mut app_server)).await;
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_matches!(
        app_event_rx.try_recv(),
        Ok(AppEvent::UpdateFeatureFlags { updates }) if updates == vec![(Feature::Collab, true)]
    );
    let cell = match app_event_rx.try_recv() {
        Ok(AppEvent::InsertHistoryCell(cell)) => cell,
        other => panic!("expected InsertHistoryCell event, got {other:?}"),
    };
    let rendered = cell
        .display_lines(/*width*/ 120)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("Subagents will be enabled in the next session."));
    Ok(())
}


#[tokio::test]
async fn open_agent_picker_allows_existing_agent_threads_when_feature_is_disabled() -> Result<()> {
    let (mut app, mut app_event_rx, _op_rx) = Box::pin(make_test_app_with_channels()).await;
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
        app.chat_widget.config_ref(),
    ))
    .await
    .expect("embedded app server");
    let thread_id = ThreadId::new();
    app.thread_event_channels
        .insert(thread_id, ThreadEventChannel::new(/*capacity*/ 1));

    Box::pin(app.open_agent_picker(&mut app_server)).await;
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_matches!(
        app_event_rx.try_recv(),
        Ok(AppEvent::SelectAgentThread(selected_thread_id)) if selected_thread_id == thread_id
    );
    Ok(())
}


#[test]
fn agent_picker_item_name_snapshot() {
    let thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000123").expect("valid thread id");
    let snapshot = [
        format!(
            "{} | {}",
            format_agent_picker_item_name(
                Some("Robie"),
                Some("explorer"),
                /*is_primary*/ true
            ),
            thread_id
        ),
        format!(
            "{} | {}",
            format_agent_picker_item_name(
                Some("Robie"),
                Some("explorer"),
                /*is_primary*/ false
            ),
            thread_id
        ),
        format!(
            "{} | {}",
            format_agent_picker_item_name(
                Some("Robie"),
                /*agent_role*/ None,
                /*is_primary*/ false
            ),
            thread_id
        ),
        format!(
            "{} | {}",
            format_agent_picker_item_name(
                /*agent_nickname*/ None,
                Some("explorer"),
                /*is_primary*/ false
            ),
            thread_id
        ),
        format!(
            "{} | {}",
            format_agent_picker_item_name(
                /*agent_nickname*/ None, /*agent_role*/ None, /*is_primary*/ false
            ),
            thread_id
        ),
    ]
    .join("\n");
    assert_app_snapshot!("agent_picker_item_name", snapshot);
}
