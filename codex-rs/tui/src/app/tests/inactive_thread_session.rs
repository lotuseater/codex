use super::*;

#[tokio::test]
async fn inactive_thread_started_notification_initializes_replay_session() -> Result<()> {
    let mut app = make_test_app().await;
    let temp_dir = tempdir()?;
    let main_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000101").expect("valid thread");
    let agent_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000202").expect("valid thread");
    let primary_cwd = test_path_buf("/tmp/main").abs();
    let shared_root = test_path_buf("/tmp/shared").abs();
    let primary_session = ThreadSessionState {
        approval_policy: AskForApproval::OnRequest.to_core(),
        permission_profile: PermissionProfile::workspace_write(),
        runtime_workspace_roots: vec![primary_cwd.clone(), shared_root.clone()],
        ..test_thread_session(main_thread_id, primary_cwd.to_path_buf())
    };

    app.primary_thread_id = Some(main_thread_id);
    app.active_thread_id = Some(main_thread_id);
    app.primary_session_configured = Some(primary_session.clone());
    app.thread_event_channels.insert(
        main_thread_id,
        ThreadEventChannel::new_with_session(
            /*capacity*/ 4,
            primary_session.clone(),
            Vec::new(),
        ),
    );

    let rollout_path = temp_dir.path().join("agent-rollout.jsonl");
    let rollout = serde_json::json!({
        "timestamp": "t0",
        "type": "turn_context",
        "payload": {
            "cwd": test_path_buf("/tmp/agent"),
            "model": "gpt-agent",
        },
    });
    std::fs::write(
        &rollout_path,
        format!("{}\n", serde_json::to_string(&rollout)?),
    )?;
    app.enqueue_thread_notification(
        agent_thread_id,
        ServerNotification::ThreadStarted(ThreadStartedNotification {
            thread: Thread {
                id: agent_thread_id.to_string(),
                session_id: agent_thread_id.to_string(),
                forked_from_id: None,
                preview: "agent thread".to_string(),
                ephemeral: false,
                model_provider: "agent-provider".to_string(),
                created_at: 1,
                updated_at: 2,
                status: codex_app_server_protocol::ThreadStatus::Idle,
                path: Some(rollout_path.clone()),
                cwd: test_path_buf("/tmp/agent").abs(),
                cli_version: "0.0.0".to_string(),
                source: codex_app_server_protocol::SessionSource::Unknown,
                thread_source: None,
                agent_nickname: Some("Robie".to_string()),
                agent_role: Some("explorer".to_string()),
                git_info: None,
                name: Some("agent thread".to_string()),
                turns: Vec::new(),
            },
        }),
    )
    .await?;

    let store = app
        .thread_event_channels
        .get(&agent_thread_id)
        .expect("agent thread channel")
        .store
        .lock()
        .await;
    let session = store.session.clone().expect("inferred session");
    drop(store);

    assert_eq!(session.thread_id, agent_thread_id);
    assert_eq!(session.thread_name, Some("agent thread".to_string()));
    assert_eq!(session.model, "gpt-agent");
    assert_eq!(session.model_provider_id, "agent-provider");
    assert_eq!(session.approval_policy, primary_session.approval_policy);
    assert_eq!(session.cwd.as_path(), test_path_buf("/tmp/agent").as_path());
    assert_eq!(
        session.runtime_workspace_roots,
        vec![test_path_buf("/tmp/agent").abs(), shared_root]
    );
    assert_eq!(session.rollout_path, Some(rollout_path));
    assert_eq!(
        app.agent_navigation.get(&agent_thread_id),
        Some(&AgentPickerThreadEntry {
            agent_nickname: Some("Robie".to_string()),
            agent_role: Some("explorer".to_string()),
            is_closed: false,
            model: None,
            reasoning_effort: None,
            token_context_percent_used: None,
        })
    );

    Ok(())
}

#[tokio::test]
async fn inactive_thread_started_notification_preserves_primary_model_when_path_missing()
-> Result<()> {
    let mut app = make_test_app().await;
    let main_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000301").expect("valid thread");
    let agent_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000302").expect("valid thread");
    let primary_cwd = test_path_buf("/tmp/main").abs();
    let primary_session = ThreadSessionState {
        approval_policy: AskForApproval::OnRequest.to_core(),
        permission_profile: PermissionProfile::workspace_write(),
        runtime_workspace_roots: vec![primary_cwd.clone()],
        ..test_thread_session(main_thread_id, primary_cwd.to_path_buf())
    };

    app.primary_thread_id = Some(main_thread_id);
    app.active_thread_id = Some(main_thread_id);
    app.primary_session_configured = Some(primary_session.clone());
    app.thread_event_channels.insert(
        main_thread_id,
        ThreadEventChannel::new_with_session(
            /*capacity*/ 4,
            primary_session.clone(),
            Vec::new(),
        ),
    );

    app.enqueue_thread_notification(
        agent_thread_id,
        ServerNotification::ThreadStarted(ThreadStartedNotification {
            thread: Thread {
                id: agent_thread_id.to_string(),
                session_id: agent_thread_id.to_string(),
                forked_from_id: None,
                preview: "agent thread".to_string(),
                ephemeral: false,
                model_provider: "agent-provider".to_string(),
                created_at: 1,
                updated_at: 2,
                status: codex_app_server_protocol::ThreadStatus::Idle,
                path: None,
                cwd: test_path_buf("/tmp/agent").abs(),
                cli_version: "0.0.0".to_string(),
                source: codex_app_server_protocol::SessionSource::Unknown,
                thread_source: None,
                agent_nickname: Some("Robie".to_string()),
                agent_role: Some("explorer".to_string()),
                git_info: None,
                name: Some("agent thread".to_string()),
                turns: Vec::new(),
            },
        }),
    )
    .await?;

    let store = app
        .thread_event_channels
        .get(&agent_thread_id)
        .expect("agent thread channel")
        .store
        .lock()
        .await;
    let session = store.session.clone().expect("inferred session");

    assert_eq!(session.model, primary_session.model);

    Ok(())
}

/// `thread/read` is metadata/replay hydration and does not return a fresh
/// server-authored `PermissionProfile`, so it must not reuse the cached primary
/// session profile after swapping in the read thread's cwd.
#[tokio::test]
async fn thread_read_session_state_does_not_reuse_primary_permission_profile() {
    let mut app = make_test_app().await;
    let main_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000401").expect("valid thread");
    let read_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000402").expect("valid thread");
    let primary_cwd = test_path_buf("/tmp/main").abs();
    let primary_session = ThreadSessionState {
        approval_policy: AskForApproval::OnRequest.to_core(),
        permission_profile: PermissionProfile::workspace_write(),
        runtime_workspace_roots: vec![primary_cwd.clone()],
        ..test_thread_session(main_thread_id, primary_cwd.to_path_buf())
    };
    app.primary_session_configured = Some(primary_session);

    let thread = Thread {
        id: read_thread_id.to_string(),
        session_id: read_thread_id.to_string(),
        forked_from_id: None,
        preview: "read thread".to_string(),
        ephemeral: false,
        model_provider: "read-provider".to_string(),
        created_at: 1,
        updated_at: 2,
        status: codex_app_server_protocol::ThreadStatus::Idle,
        path: None,
        cwd: test_path_buf("/tmp/read").abs(),
        cli_version: "0.0.0".to_string(),
        source: codex_app_server_protocol::SessionSource::Unknown,
        thread_source: None,
        agent_nickname: None,
        agent_role: None,
        git_info: None,
        name: Some("read thread".to_string()),
        turns: Vec::new(),
    };

    let session = app
        .session_state_for_thread_read(read_thread_id, &thread)
        .await;

    assert_eq!(session.thread_id, read_thread_id);
    assert_eq!(session.cwd.as_path(), test_path_buf("/tmp/read").as_path());
    assert_eq!(
        session.runtime_workspace_roots,
        vec![test_path_buf("/tmp/read").abs()]
    );
    let expected_permission_profile = app
        .chat_widget
        .config_ref()
        .permissions
        .permission_profile()
        .clone();
    assert_eq!(
        session.permission_profile, expected_permission_profile,
        "thread/read does not return fresh server permissions; the fallback profile must use the \
         active widget permissions rather than reusing the cached primary session profile"
    );
}

#[tokio::test]
async fn inactive_thread_settings_notification_updates_cached_collaboration_mode() {
    let mut app = make_test_app().await;
    let primary_thread_id = ThreadId::new();
    let inactive_thread_id = ThreadId::new();
    let primary_session = test_thread_session(primary_thread_id, test_path_buf("/tmp/main"));
    let inactive_session = test_thread_session(inactive_thread_id, test_path_buf("/tmp/inactive"));
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Plan,
        settings: Settings {
            model: "gpt-plan".to_string(),
            reasoning_effort: Some(ReasoningEffortConfig::High),
            developer_instructions: Some("draft a plan first".to_string()),
        },
    };

    app.primary_thread_id = Some(primary_thread_id);
    app.active_thread_id = Some(primary_thread_id);
    app.primary_session_configured = Some(primary_session.clone());
    app.thread_event_channels.insert(
        primary_thread_id,
        ThreadEventChannel::new_with_session(
            THREAD_EVENT_CHANNEL_CAPACITY,
            primary_session,
            Vec::new(),
        ),
    );
    app.thread_event_channels.insert(
        inactive_thread_id,
        ThreadEventChannel::new_with_session(
            THREAD_EVENT_CHANNEL_CAPACITY,
            inactive_session,
            Vec::new(),
        ),
    );

    let notification = ThreadSettingsUpdatedNotification {
        thread_id: inactive_thread_id.to_string(),
        thread_settings: ThreadSettings {
            cwd: test_absolute_path("/tmp/thread-settings"),
            approval_policy: AskForApproval::OnRequest,
            approvals_reviewer: codex_app_server_protocol::ApprovalsReviewer::AutoReview,
            sandbox_policy: codex_app_server_protocol::SandboxPolicy::ReadOnly {
                network_access: false,
            },
            active_permission_profile: Some(
                codex_app_server_protocol::ActivePermissionProfile::read_only(),
            ),
            model: "gpt-plan".to_string(),
            model_provider: "openai".to_string(),
            service_tier: None,
            effort: collaboration_mode.settings.reasoning_effort,
            summary: None,
            collaboration_mode: collaboration_mode.clone(),
            personality: Some(Personality::Pragmatic),
        },
    };
    app.enqueue_thread_notification(
        inactive_thread_id,
        ServerNotification::ThreadSettingsUpdated(notification),
    )
    .await
    .expect("settings notification should be cached");

    let cached_session = app
        .thread_event_channels
        .get(&inactive_thread_id)
        .expect("inactive thread channel")
        .store
        .lock()
        .await
        .session
        .clone()
        .expect("inactive session should remain cached");
    assert_eq!(cached_session.model, "gpt-test");
    assert_eq!(cached_session.personality, Some(Personality::Pragmatic));
    assert_eq!(
        cached_session.collaboration_mode.as_deref(),
        Some(&collaboration_mode)
    );

    app.chat_widget.handle_thread_session(cached_session);
    assert_eq!(
        app.chat_widget.active_collaboration_mode_kind(),
        ModeKind::Plan
    );
    assert_eq!(app.chat_widget.current_model(), "gpt-plan");
    assert_eq!(
        app.chat_widget.current_collaboration_mode().model(),
        "gpt-test"
    );
    assert_eq!(
        app.chat_widget.current_reasoning_effort(),
        Some(ReasoningEffortConfig::High)
    );
    assert_eq!(
        app.chat_widget.config_ref().personality,
        Some(Personality::Pragmatic)
    );
}
