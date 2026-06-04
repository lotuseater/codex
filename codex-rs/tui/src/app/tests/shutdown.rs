use super::*;

#[tokio::test]
async fn active_non_primary_shutdown_target_returns_none_for_non_shutdown_event() -> Result<()> {
    let mut app = make_test_app().await;
    app.active_thread_id = Some(ThreadId::new());
    app.primary_thread_id = Some(ThreadId::new());

    assert_eq!(
        app.active_non_primary_shutdown_target(&ServerNotification::SkillsChanged(
            codex_app_server_protocol::SkillsChangedNotification {},
        )),
        None
    );
    Ok(())
}

#[tokio::test]
async fn active_non_primary_shutdown_target_returns_none_for_primary_thread_shutdown() -> Result<()>
{
    let mut app = make_test_app().await;
    let thread_id = ThreadId::new();
    app.active_thread_id = Some(thread_id);
    app.primary_thread_id = Some(thread_id);

    assert_eq!(
        app.active_non_primary_shutdown_target(&thread_closed_notification(thread_id)),
        None
    );
    Ok(())
}

#[tokio::test]
async fn active_non_primary_shutdown_target_returns_ids_for_non_primary_shutdown() -> Result<()> {
    let mut app = make_test_app().await;
    let active_thread_id = ThreadId::new();
    let primary_thread_id = ThreadId::new();
    app.active_thread_id = Some(active_thread_id);
    app.primary_thread_id = Some(primary_thread_id);

    assert_eq!(
        app.active_non_primary_shutdown_target(&thread_closed_notification(active_thread_id)),
        Some((active_thread_id, primary_thread_id))
    );
    Ok(())
}

#[tokio::test]
async fn active_non_primary_shutdown_target_returns_none_when_shutdown_exit_is_pending()
-> Result<()> {
    let mut app = make_test_app().await;
    let active_thread_id = ThreadId::new();
    let primary_thread_id = ThreadId::new();
    app.active_thread_id = Some(active_thread_id);
    app.primary_thread_id = Some(primary_thread_id);
    app.pending_shutdown_exit_thread_id = Some(active_thread_id);

    assert_eq!(
        app.active_non_primary_shutdown_target(&thread_closed_notification(active_thread_id)),
        None
    );
    Ok(())
}

#[tokio::test]
async fn active_non_primary_shutdown_target_still_switches_for_other_pending_exit_thread()
-> Result<()> {
    let mut app = make_test_app().await;
    let active_thread_id = ThreadId::new();
    let primary_thread_id = ThreadId::new();
    app.active_thread_id = Some(active_thread_id);
    app.primary_thread_id = Some(primary_thread_id);
    app.pending_shutdown_exit_thread_id = Some(ThreadId::new());

    assert_eq!(
        app.active_non_primary_shutdown_target(&thread_closed_notification(active_thread_id)),
        Some((active_thread_id, primary_thread_id))
    );
    Ok(())
}

#[tokio::test]
async fn new_session_requests_shutdown_for_previous_conversation() {
    Box::pin(async {
        let (mut app, mut app_event_rx, mut op_rx) = Box::pin(make_test_app_with_channels()).await;

        let thread_id = ThreadId::new();
        let event = crate::session_state::ThreadSessionState {
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
        };

        app.chat_widget.handle_thread_session(event);

        while app_event_rx.try_recv().is_ok() {}
        while op_rx.try_recv().is_ok() {}

        let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
            app.chat_widget.config_ref(),
        ))
        .await
        .expect("embedded app server");
        Box::pin(app.shutdown_current_thread(&mut app_server)).await;

        assert!(
            op_rx.try_recv().is_err(),
            "shutdown should not submit Op::Shutdown"
        );
    })
    .await;
}

#[tokio::test]
async fn shutdown_first_exit_returns_immediate_exit_when_shutdown_submit_fails() {
    let mut app = make_test_app().await;
    let thread_id = ThreadId::new();
    app.active_thread_id = Some(thread_id);

    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
        app.chat_widget.config_ref(),
    ))
    .await
    .expect("embedded app server");
    let control = Box::pin(app.handle_exit_mode(&mut app_server, ExitMode::ShutdownFirst)).await;

    assert_eq!(app.pending_shutdown_exit_thread_id, None);
    assert!(matches!(
        control,
        AppRunControl::Exit(ExitReason::UserRequested)
    ));
}

#[tokio::test]
async fn shutdown_first_exit_uses_app_server_shutdown_without_submitting_op() {
    let (mut app, _app_event_rx, mut op_rx) = Box::pin(make_test_app_with_channels()).await;
    let thread_id = ThreadId::new();
    app.active_thread_id = Some(thread_id);

    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
        app.chat_widget.config_ref(),
    ))
    .await
    .expect("embedded app server");
    let control = Box::pin(app.handle_exit_mode(&mut app_server, ExitMode::ShutdownFirst)).await;

    assert_eq!(app.pending_shutdown_exit_thread_id, None);
    assert!(matches!(
        control,
        AppRunControl::Exit(ExitReason::UserRequested)
    ));
    assert!(
        op_rx.try_recv().is_err(),
        "shutdown should not submit Op::Shutdown"
    );
}

#[tokio::test]
async fn interrupt_without_active_turn_is_treated_as_handled() {
    Box::pin(async {
        let mut app = make_test_app().await;
        let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
            app.chat_widget.config_ref(),
        ))
        .await
        .expect("embedded app server");
        let started = app_server
            .start_thread(app.chat_widget.config_ref())
            .await
            .expect("thread/start should succeed");
        let thread_id = started.session.thread_id;
        app.enqueue_primary_thread_session(started.session, started.turns)
            .await
            .expect("primary thread should be registered");
        let op = AppCommand::interrupt();

        let handled = Box::pin(app.try_submit_active_thread_op_via_app_server(
            &mut app_server,
            thread_id,
            &op,
        ))
        .await
        .expect("interrupt submission should not fail");

        assert_eq!(handled, true);
    })
    .await;
}
