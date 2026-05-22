use super::*;

#[tokio::test]
async fn regular_turn_emits_turn_started_without_waiting_for_startup_prewarm() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let (_tx, startup_prewarm_rx) = tokio::sync::oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        let _ = startup_prewarm_rx.await;
        Ok(test_model_client_session())
    });

    sess.set_session_startup_prewarm(
        crate::session_startup_prewarm::SessionStartupPrewarmHandle::new(
            handle,
            std::time::Instant::now(),
            crate::client::WEBSOCKET_CONNECT_TIMEOUT,
        ),
    )
    .await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        crate::tasks::RegularTask::new(),
    )
    .await;

    let first = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("expected turn started event without waiting for startup prewarm")
        .expect("channel open");
    assert!(matches!(
        first.msg,
        EventMsg::TurnStarted(TurnStartedEvent { turn_id, .. }) if turn_id == tc.sub_id
    ));

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
}

#[tokio::test]
async fn interrupting_regular_turn_waiting_on_startup_prewarm_emits_turn_aborted() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let (_tx, startup_prewarm_rx) = tokio::sync::oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        let _ = startup_prewarm_rx.await;
        Ok(test_model_client_session())
    });

    sess.set_session_startup_prewarm(
        crate::session_startup_prewarm::SessionStartupPrewarmHandle::new(
            handle,
            std::time::Instant::now(),
            crate::client::WEBSOCKET_CONNECT_TIMEOUT,
        ),
    )
    .await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        crate::tasks::RegularTask::new(),
    )
    .await;

    let first = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("expected turn started event without waiting for startup prewarm")
        .expect("channel open");
    assert!(matches!(
        first.msg,
        EventMsg::TurnStarted(TurnStartedEvent { turn_id, .. }) if turn_id == tc.sub_id
    ));

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    let second = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected turn aborted event")
        .expect("channel open");
    let EventMsg::TurnAborted(TurnAbortedEvent {
        turn_id,
        reason,
        completed_at,
        duration_ms,
    }) = second.msg
    else {
        panic!("expected turn aborted event");
    };
    assert_eq!(turn_id, Some(tc.sub_id.clone()));
    assert_eq!(reason, TurnAbortReason::Interrupted);
    assert!(completed_at.is_some());
    assert!(duration_ms.is_some());
}

#[tokio::test]
async fn reload_user_config_layer_refreshes_hooks() -> anyhow::Result<()> {
    let session = make_session_with_config(|config| {
        config
            .features
            .enable(Feature::CodexHooks)
            .expect("enable Codex hooks");
    })
    .await?;
    let codex_home = session.codex_home().await;
    std::fs::create_dir_all(&codex_home)?;
    let config_toml_path = codex_home.join(CONFIG_TOML_FILE);
    let user_config: codex_config::TomlValue = serde_json::from_value(serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": "python3 /tmp/user.py",
                }],
            }],
        },
    }))?;

    let request = codex_hooks::SessionStartRequest {
        session_id: session.conversation_id,
        cwd: session.get_config().await.cwd.clone(),
        transcript_path: None,
        model: "gpt-5.2".to_string(),
        permission_mode: "default".to_string(),
        target: codex_hooks::StartHookTarget::SessionStart {
            source: codex_hooks::SessionStartSource::Startup,
        },
    };
    assert!(session.hooks().preview_session_start(&request).is_empty());

    let config = session.get_config().await;
    let hook_list = codex_hooks::list_hooks(codex_hooks::HooksConfig {
        feature_enabled: true,
        config_layer_stack: Some(
            config
                .config_layer_stack
                .with_user_config(&config_toml_path, user_config.clone()),
        ),
        ..codex_hooks::HooksConfig::default()
    });
    assert_eq!(hook_list.hooks.len(), 1);
    assert_eq!(
        hook_list.hooks[0].trust_status,
        codex_protocol::protocol::HookTrustStatus::Untrusted
    );

    let trusted_user_config: codex_config::TomlValue = serde_json::from_value(serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": "python3 /tmp/user.py",
                }],
            }],
            "state": {
                hook_list.hooks[0].key.clone(): {
                    "trusted_hash": hook_list.hooks[0].current_hash.clone(),
                },
            },
        },
    }))?;
    std::fs::write(&config_toml_path, toml::to_string(&trusted_user_config)?)?;

    session.reload_user_config_layer().await;

    assert_eq!(session.hooks().preview_session_start(&request).len(), 1);
    Ok(())
}

#[tokio::test]
async fn refresh_runtime_config_refreshes_hooks() -> anyhow::Result<()> {
    let (session, _turn_context) = make_session_and_context().await;
    {
        let mut state = session.state.lock().await;
        let mut config = (*state.session_configuration.original_config_do_not_use).clone();
        config
            .features
            .enable(Feature::CodexHooks)
            .expect("enable Codex hooks");
        state.session_configuration.original_config_do_not_use = Arc::new(config);
    }
    let codex_home = session.codex_home().await;
    std::fs::create_dir_all(&codex_home)?;
    let config_toml_path = codex_home.join(CONFIG_TOML_FILE);
    #[derive(serde::Serialize)]
    struct NormalizedHookIdentity {
        event_name: &'static str,
        #[serde(flatten)]
        group: codex_config::MatcherGroup,
    }
    let trusted_hash = {
        let identity = NormalizedHookIdentity {
            event_name: "session_start",
            group: codex_config::MatcherGroup {
                matcher: None,
                hooks: vec![codex_config::HookHandlerConfig::Command {
                    command: "python3 /tmp/user.py".to_string(),
                    command_windows: None,
                    timeout_sec: Some(600),
                    r#async: false,
                    status_message: None,
                }],
            },
        };
        let identity = codex_config::TomlValue::try_from(identity)?;
        codex_config::version_for_toml(&identity)
    };
    let hook_key = format!("{}:session_start:0:0", config_toml_path.display());
    let trusted_user_config: codex_config::TomlValue = serde_json::from_value(serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": "python3 /tmp/user.py",
                }],
            }],
            "state": {
                hook_key: {
                    "trusted_hash": trusted_hash,
                },
            },
        },
    }))?;
    std::fs::write(&config_toml_path, toml::to_string(&trusted_user_config)?)?;

    let request = codex_hooks::SessionStartRequest {
        session_id: session.conversation_id,
        cwd: session.get_config().await.cwd.clone(),
        transcript_path: None,
        model: "gpt-5.2".to_string(),
        permission_mode: "default".to_string(),
        target: codex_hooks::StartHookTarget::SessionStart {
            source: codex_hooks::SessionStartSource::Startup,
        },
    };
    assert!(session.hooks().preview_session_start(&request).is_empty());

    let next_config = load_latest_config_for_session(&session).await;
    session.refresh_runtime_config(next_config).await;

    assert_eq!(session.hooks().preview_session_start(&request).len(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fork_startup_context_then_first_turn_diff_snapshot() -> anyhow::Result<()> {
    let server = start_mock_server().await;
    mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let first_forked_request = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-2"), ev_completed("resp-2")]),
    )
    .await;

    let mut builder = test_codex().with_config(|config| {
        config.permissions.approval_policy =
            codex_config::Constrained::allow_any(AskForApproval::OnRequest);
    });
    let initial = builder.build(&server).await?;
    let rollout_path = initial
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");

    initial
        .codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "fork seed".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&initial.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
    // Forking reads the persisted rollout JSONL, so force the completed source turn to disk
    // before snapshotting from it.
    initial.codex.ensure_rollout_materialized().await;
    initial
        .codex
        .flush_rollout()
        .await
        .expect("source rollout should flush before fork");

    let mut fork_config = initial.config.clone();
    fork_config.permissions.approval_policy =
        codex_config::Constrained::allow_any(AskForApproval::UnlessTrusted);
    let forked = initial
        .thread_manager
        .fork_thread(
            usize::MAX,
            fork_config.clone(),
            rollout_path,
            /*thread_source*/ None,
            /*persist_extended_history*/ false,
            /*parent_trace*/ None,
        )
        .await?;

    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Plan,
        settings: Settings {
            model: forked.session_configured.model.clone(),
            reasoning_effort: None,
            developer_instructions: Some("Fork turn collaboration instructions.".to_string()),
        },
    };
    forked
        .thread
        .submit(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: Some(AskForApproval::Never),
            approvals_reviewer: None,
            sandbox_policy: None,
            permission_profile: None,
            windows_sandbox_level: None,
            model: None,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: Some(collaboration_mode),
            personality: None,
        })
        .await?;

    forked
        .thread
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "after fork".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&forked.thread, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let request = first_forked_request.single_request();
    let snapshot = context_snapshot::format_labeled_requests_snapshot(
        "First request after fork when startup preserves the parent baseline, the fork changes approval policy, and the first forked turn enters plan mode.",
        &[("First Forked Turn Request", &request)],
        &ContextSnapshotOptions::default()
            .render_mode(ContextSnapshotRenderMode::KindWithTextPrefix { max_chars: 96 })
            .strip_capability_instructions()
            .strip_agents_md_user_context(),
    );

    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path("snapshots");
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| {
        insta::assert_snapshot!(
            "codex_core__codex_tests__fork_startup_context_then_first_turn_diff",
            snapshot
        );
    });

    Ok(())
}

#[tokio::test]
async fn spawn_task_turn_span_inherits_dispatch_trace_context() {
    struct TraceCaptureTask {
        captured_trace: Arc<std::sync::Mutex<Option<W3cTraceContext>>>,
    }

    impl SessionTask for TraceCaptureTask {
        fn kind(&self) -> TaskKind {
            TaskKind::Regular
        }

        fn span_name(&self) -> &'static str {
            "session_task.trace_capture"
        }

        async fn run(
            self: Arc<Self>,
            _session: Arc<SessionTaskContext>,
            _ctx: Arc<TurnContext>,
            _input: Vec<UserInput>,
            _cancellation_token: CancellationToken,
        ) -> Option<String> {
            let mut trace = self
                .captured_trace
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *trace = current_span_w3c_trace_context();
            None
        }
    }

    let _trace_test_context = install_test_tracing("codex-core-tests");

    let request_parent = W3cTraceContext {
        traceparent: Some("00-00000000000000000000000000000011-0000000000000022-01".into()),
        tracestate: Some("vendor=value".into()),
    };
    let request_span = tracing::info_span!("app_server.request");
    assert!(set_parent_from_w3c_trace_context(
        &request_span,
        &request_parent
    ));

    let submission_trace =
        async { current_span_w3c_trace_context().expect("request span should have trace context") }
            .instrument(request_span)
            .await;

    let dispatch_span = submission_dispatch_span(&Submission {
        id: "sub-1".into(),
        op: Op::Interrupt,
        trace: Some(submission_trace.clone()),
    });
    let dispatch_span_id = dispatch_span.context().span().span_context().span_id();

    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let captured_trace = Arc::new(std::sync::Mutex::new(None));

    async {
        sess.spawn_task(
            Arc::clone(&tc),
            vec![UserInput::Text {
                text: "hello".to_string(),
                text_elements: Vec::new(),
            }],
            TraceCaptureTask {
                captured_trace: Arc::clone(&captured_trace),
            },
        )
        .await;
    }
    .instrument(dispatch_span)
    .await;

    let evt = tokio::time::timeout(StdDuration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for turn completion")
        .expect("event");
    assert!(matches!(evt.msg, EventMsg::TurnComplete(_)));

    let task_trace = captured_trace
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .expect("turn task should capture the current span trace context");
    let submission_context =
        codex_otel::context_from_w3c_trace_context(&submission_trace).expect("submission");
    let task_context = codex_otel::context_from_w3c_trace_context(&task_trace).expect("task trace");

    assert_eq!(
        task_context.span().span_context().trace_id(),
        submission_context.span().span_context().trace_id()
    );
    assert_ne!(
        task_context.span().span_context().span_id(),
        dispatch_span_id
    );
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn shutdown_complete_does_not_append_to_thread_store_after_shutdown() {
    let (mut session, _turn_context) = make_session_and_context().await;
    let store = Arc::new(InMemoryThreadStore::default());
    let thread_store: Arc<dyn codex_thread_store_api::ThreadStore> = store.clone();
    let config = session.get_config().await;
    let live_thread = LiveThread::create(
        Arc::clone(&thread_store),
        CreateThreadParams {
            thread_id: session.conversation_id,
            forked_from_id: None,
            source: SessionSource::Exec,
            thread_source: None,
            base_instructions: BaseInstructions::default(),
            dynamic_tools: Vec::new(),
            metadata: ThreadPersistenceMetadata {
                cwd: Some(config.cwd.to_path_buf()),
                model_provider: config.model_provider_id.clone(),
                memory_mode: if config.memories.generate_memories {
                    ThreadMemoryMode::Enabled
                } else {
                    ThreadMemoryMode::Disabled
                },
            },
            event_persistence_mode: ThreadEventPersistenceMode::Limited,
        },
    )
    .await
    .expect("create thread persistence");
    session.services.thread_store = thread_store;
    session.services.live_thread = Some(live_thread);
    let session = Arc::new(session);

    assert!(handlers::shutdown(&session, "sub-1".to_string()).await);

    assert_eq!(
        InMemoryThreadStoreCalls {
            create_thread: 1,
            shutdown_thread: 1,
            ..Default::default()
        },
        store.calls().await
    );
}

#[tokio::test]
async fn shutdown_and_wait_allows_multiple_waiters() {
    let (session, _turn_context) = make_session_and_context().await;
    let (tx_sub, rx_sub) = async_channel::bounded(4);
    let (_tx_event, rx_event) = async_channel::unbounded();
    let (_agent_status_tx, agent_status) = watch::channel(AgentStatus::PendingInit);
    let session_loop_handle = tokio::spawn(async move {
        let shutdown: Submission = rx_sub.recv().await.expect("shutdown submission");
        assert_eq!(shutdown.op, Op::Shutdown);
        tokio::time::sleep(StdDuration::from_millis(50)).await;
    });
    let codex = Arc::new(Codex {
        tx_sub,
        rx_event,
        agent_status,
        session: Arc::new(session),
        session_loop_termination: session_loop_termination_from_handle(session_loop_handle),
    });

    let waiter_1 = {
        let codex = Arc::clone(&codex);
        tokio::spawn(async move { codex.shutdown_and_wait().await })
    };
    let waiter_2 = {
        let codex = Arc::clone(&codex);
        tokio::spawn(async move { codex.shutdown_and_wait().await })
    };

    waiter_1
        .await
        .expect("first shutdown waiter join")
        .expect("first shutdown waiter");
    waiter_2
        .await
        .expect("second shutdown waiter join")
        .expect("second shutdown waiter");
}

#[tokio::test]
async fn shutdown_and_wait_waits_when_shutdown_is_already_in_progress() {
    let (session, _turn_context) = make_session_and_context().await;
    let (tx_sub, rx_sub) = async_channel::bounded(4);
    drop(rx_sub);
    let (_tx_event, rx_event) = async_channel::unbounded();
    let (_agent_status_tx, agent_status) = watch::channel(AgentStatus::PendingInit);
    let (shutdown_complete_tx, shutdown_complete_rx) = tokio::sync::oneshot::channel();
    let session_loop_handle = tokio::spawn(async move {
        let _ = shutdown_complete_rx.await;
    });
    let codex = Arc::new(Codex {
        tx_sub,
        rx_event,
        agent_status,
        session: Arc::new(session),
        session_loop_termination: session_loop_termination_from_handle(session_loop_handle),
    });

    let waiter = {
        let codex = Arc::clone(&codex);
        tokio::spawn(async move { codex.shutdown_and_wait().await })
    };

    tokio::time::sleep(StdDuration::from_millis(10)).await;
    assert!(!waiter.is_finished());

    shutdown_complete_tx
        .send(())
        .expect("session loop should still be waiting to terminate");

    waiter
        .await
        .expect("shutdown waiter join")
        .expect("shutdown waiter");
}

#[tokio::test]
async fn shutdown_and_wait_shuts_down_cached_guardian_subagent() {
    let (parent_session, parent_turn_context) = make_session_and_context().await;
    let parent_session = Arc::new(parent_session);
    let parent_config = Arc::clone(&parent_turn_context.config);
    let (parent_tx_sub, parent_rx_sub) = async_channel::bounded(4);
    let (_parent_tx_event, parent_rx_event) = async_channel::unbounded();
    let (_parent_status_tx, parent_agent_status) = watch::channel(AgentStatus::PendingInit);
    let parent_session_for_loop = Arc::clone(&parent_session);
    let parent_session_loop_handle = tokio::spawn(async move {
        submission_loop(parent_session_for_loop, parent_config, parent_rx_sub).await;
    });
    let parent_codex = Codex {
        tx_sub: parent_tx_sub,
        rx_event: parent_rx_event,
        agent_status: parent_agent_status,
        session: Arc::clone(&parent_session),
        session_loop_termination: session_loop_termination_from_handle(parent_session_loop_handle),
    };

    let (child_session, _child_turn_context) = make_session_and_context().await;
    let (child_tx_sub, child_rx_sub) = async_channel::bounded(4);
    let (_child_tx_event, child_rx_event) = async_channel::unbounded();
    let (_child_status_tx, child_agent_status) = watch::channel(AgentStatus::PendingInit);
    let (child_shutdown_tx, child_shutdown_rx) = tokio::sync::oneshot::channel();
    let child_session_loop_handle = tokio::spawn(async move {
        let shutdown: Submission = child_rx_sub
            .recv()
            .await
            .expect("child shutdown submission");
        assert_eq!(shutdown.op, Op::Shutdown);
        child_shutdown_tx
            .send(())
            .expect("child shutdown signal should be delivered");
    });
    let child_codex = Codex {
        tx_sub: child_tx_sub,
        rx_event: child_rx_event,
        agent_status: child_agent_status,
        session: Arc::new(child_session),
        session_loop_termination: session_loop_termination_from_handle(child_session_loop_handle),
    };
    parent_session
        .guardian_review_session
        .cache_for_test(child_codex)
        .await;

    parent_codex
        .shutdown_and_wait()
        .await
        .expect("parent shutdown should succeed");

    child_shutdown_rx
        .await
        .expect("guardian subagent should receive a shutdown op");
}

#[tokio::test]
async fn shutdown_and_wait_shuts_down_tracked_ephemeral_guardian_review() {
    let (parent_session, parent_turn_context) = make_session_and_context().await;
    let parent_session = Arc::new(parent_session);
    let parent_config = Arc::clone(&parent_turn_context.config);
    let (parent_tx_sub, parent_rx_sub) = async_channel::bounded(4);
    let (_parent_tx_event, parent_rx_event) = async_channel::unbounded();
    let (_parent_status_tx, parent_agent_status) = watch::channel(AgentStatus::PendingInit);
    let parent_session_for_loop = Arc::clone(&parent_session);
    let parent_session_loop_handle = tokio::spawn(async move {
        submission_loop(parent_session_for_loop, parent_config, parent_rx_sub).await;
    });
    let parent_codex = Codex {
        tx_sub: parent_tx_sub,
        rx_event: parent_rx_event,
        agent_status: parent_agent_status,
        session: Arc::clone(&parent_session),
        session_loop_termination: session_loop_termination_from_handle(parent_session_loop_handle),
    };

    let (child_session, _child_turn_context) = make_session_and_context().await;
    let (child_tx_sub, child_rx_sub) = async_channel::bounded(4);
    let (_child_tx_event, child_rx_event) = async_channel::unbounded();
    let (_child_status_tx, child_agent_status) = watch::channel(AgentStatus::PendingInit);
    let (child_shutdown_tx, child_shutdown_rx) = tokio::sync::oneshot::channel();
    let child_session_loop_handle = tokio::spawn(async move {
        let shutdown: Submission = child_rx_sub
            .recv()
            .await
            .expect("child shutdown submission");
        assert_eq!(shutdown.op, Op::Shutdown);
        child_shutdown_tx
            .send(())
            .expect("child shutdown signal should be delivered");
    });
    let child_codex = Codex {
        tx_sub: child_tx_sub,
        rx_event: child_rx_event,
        agent_status: child_agent_status,
        session: Arc::new(child_session),
        session_loop_termination: session_loop_termination_from_handle(child_session_loop_handle),
    };
    parent_session
        .guardian_review_session
        .register_ephemeral_for_test(child_codex)
        .await;

    parent_codex
        .shutdown_and_wait()
        .await
        .expect("parent shutdown should succeed");

    child_shutdown_rx
        .await
        .expect("ephemeral guardian review should receive a shutdown op");
}

#[tokio::test]
async fn spawn_task_does_not_update_previous_turn_settings_for_non_run_turn_tasks() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    sess.set_previous_turn_settings(/*previous_turn_settings*/ None)
        .await;
    let input = vec![UserInput::Text {
        text: "hello".to_string(),
        text_elements: Vec::new(),
    }];

    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
    assert_eq!(sess.previous_turn_settings().await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_finish_emits_turn_item_lifecycle_for_leftover_pending_user_input() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![UserInput::Text {
        text: "hello".to_string(),
        text_elements: Vec::new(),
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    while rx.try_recv().is_ok() {}

    sess.inject_response_items(vec![ResponseInputItem::Message {
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "late pending input".to_string(),
        }],
        phase: None,
    }])
    .await
    .expect("inject pending input into active turn");

    sess.on_task_finished(Arc::clone(&tc), /*last_agent_message*/ None)
        .await;

    let history = sess.clone_history().await;
    let expected = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "late pending input".to_string(),
        }],
        phase: None,
    };
    assert!(
        history.raw_items().iter().any(|item| item == &expected),
        "expected pending input to be persisted into history on turn completion"
    );

    let first = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected raw response item event")
        .expect("channel open");
    assert!(matches!(first.msg, EventMsg::RawResponseItem(_)));

    let second = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected item started event")
        .expect("channel open");
    assert!(matches!(
        second.msg,
        EventMsg::ItemStarted(ItemStartedEvent {
            item: TurnItem::UserMessage(UserMessageItem { content, .. }),
            ..
        }) if content == vec![UserInput::Text {
            text: "late pending input".to_string(),
            text_elements: Vec::new(),
        }]
    ));

    let third = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected item completed event")
        .expect("channel open");
    assert!(matches!(
        third.msg,
        EventMsg::ItemCompleted(ItemCompletedEvent {
            item: TurnItem::UserMessage(UserMessageItem { content, .. }),
            ..
        }) if content == vec![UserInput::Text {
            text: "late pending input".to_string(),
            text_elements: Vec::new(),
        }]
    ));

    let fourth = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected legacy user message event")
        .expect("channel open");
    assert!(matches!(
        fourth.msg,
        EventMsg::UserMessage(UserMessageEvent {
            message,
            images,
            text_elements,
            local_images,
            ..
        }) if message == "late pending input"
            && images == Some(Vec::new())
            && text_elements.is_empty()
            && local_images.is_empty()
    ));

    let fifth = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected turn complete event")
        .expect("channel open");
    assert!(matches!(
        fifth.msg,
        EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id,
            last_agent_message: None,
            time_to_first_token_ms: None,
            ..
        }) if turn_id == tc.sub_id
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_request_user_input_does_not_spawn_extra_goal_continuation() -> anyhow::Result<()> {
    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::Goals)
            .expect("goal mode should be enableable in tests");
        config
            .features
            .enable(Feature::DefaultModeRequestUserInput)
            .expect("default-mode request_user_input should be enableable in tests");
    });
    let test = builder.build(&server).await?;
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(
                    "call-create-goal",
                    "create_goal",
                    r#"{"objective":"write a benchmark note"}"#,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_assistant_message("msg-1", "Draft ready."),
                ev_completed("resp-2"),
            ]),
            sse(vec![
                ev_response_created("resp-3"),
                ev_function_call(
                    "call-ask-user",
                    "request_user_input",
                    r#"{"questions":[{"header":"Choice","id":"next_step","question":"Pick one","options":[{"label":"Outline","description":"Start with an outline."},{"label":"Draft","description":"Write a full draft."}]}]}"#,
                ),
                ev_completed("resp-3"),
            ]),
            sse(vec![
                ev_response_created("resp-4"),
                ev_function_call(
                    "call-complete-goal",
                    "update_goal",
                    r#"{"status":"complete"}"#,
                ),
                ev_completed("resp-4"),
            ]),
            sse(vec![
                ev_assistant_message("msg-2", "Goal complete."),
                ev_completed("resp-5"),
            ]),
        ],
    )
    .await;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "write a benchmark note".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let request_user_input_event = wait_for_event_match(&test.codex, |event| match event {
        EventMsg::RequestUserInput(event) => Some(event.clone()),
        _ => None,
    })
    .await;
    assert_eq!(3, responses.requests().len());
    assert!(
        timeout(Duration::from_millis(200), test.codex.next_event())
            .await
            .is_err(),
        "waiting for request_user_input should keep the turn open without emitting more events"
    );
    assert_eq!(
        3,
        responses.requests().len(),
        "waiting for request_user_input should not start another continuation request"
    );

    test.codex
        .submit(Op::UserInputAnswer {
            id: request_user_input_event.turn_id,
            response: RequestUserInputResponse {
                answers: std::collections::HashMap::from([(
                    "next_step".to_string(),
                    RequestUserInputAnswer {
                        answers: vec!["Outline".to_string()],
                    },
                )]),
            },
        })
        .await?;

    let mut completed_turns = 0;
    timeout(Duration::from_secs(8), async {
        loop {
            let event = test.codex.next_event().await?;
            if matches!(event.msg, EventMsg::TurnComplete(_)) {
                completed_turns += 1;
                if completed_turns == 1 {
                    return anyhow::Ok(());
                }
            }
        }
    })
    .await??;

    assert_eq!(5, responses.requests().len());

    Ok(())
}

#[tokio::test]
async fn session_start_hooks_only_load_from_trusted_project_layers() -> std::io::Result<()> {
    let temp = tempfile::tempdir()?;
    let codex_home = temp.path().join("home");
    let project_root = temp.path().join("project");
    let nested = project_root.join("nested");
    let root_dot_codex = project_root.join(".codex");
    let nested_dot_codex = nested.join(".codex");

    std::fs::create_dir_all(&codex_home)?;
    std::fs::create_dir_all(&nested_dot_codex)?;
    std::fs::write(project_root.join(".git"), "gitdir: here")?;
    write_project_hooks(&root_dot_codex)?;
    write_project_hooks(&nested_dot_codex)?;
    write_project_trust_config(&codex_home, &[(&nested, TrustLevel::Trusted)]).await?;

    let config = ConfigBuilder::default()
        .codex_home(codex_home)
        .fallback_cwd(Some(nested))
        .build()
        .await?;

    let hook_list = codex_hooks::list_hooks(codex_hooks::HooksConfig {
        feature_enabled: true,
        config_layer_stack: Some(config.config_layer_stack.clone()),
        ..codex_hooks::HooksConfig::default()
    });
    let expected_source_path = codex_utils_absolute_path::AbsolutePathBuf::from_absolute_path(
        nested_dot_codex.join("hooks.json"),
    )?;
    assert_eq!(
        hook_list
            .hooks
            .iter()
            .map(|hook| &hook.source_path)
            .collect::<Vec<_>>(),
        vec![&expected_source_path],
    );
    assert_eq!(
        hook_list.hooks[0].trust_status,
        codex_protocol::protocol::HookTrustStatus::Untrusted
    );
    assert!(preview_session_start_hooks(&config).await?.is_empty());

    Ok(())
}

#[tokio::test]
async fn session_start_hooks_require_project_trust_without_config_toml() -> std::io::Result<()> {
    let temp = tempfile::tempdir()?;
    let project_root = temp.path().join("project");
    let nested = project_root.join("nested");
    let dot_codex = project_root.join(".codex");
    std::fs::create_dir_all(&nested)?;
    std::fs::write(project_root.join(".git"), "gitdir: here")?;
    write_project_hooks(&dot_codex)?;

    let cases = [
        ("unknown", Vec::<(&Path, TrustLevel)>::new(), 0_usize),
        (
            "untrusted",
            vec![(&project_root as &Path, TrustLevel::Untrusted)],
            0_usize,
        ),
        (
            "trusted",
            vec![(&project_root as &Path, TrustLevel::Trusted)],
            1_usize,
        ),
    ];

    for (name, trust_entries, expected_hooks) in cases {
        let codex_home = temp.path().join(format!("home_{name}"));
        std::fs::create_dir_all(&codex_home)?;
        write_project_trust_config(&codex_home, &trust_entries).await?;

        let config = ConfigBuilder::default()
            .codex_home(codex_home)
            .fallback_cwd(Some(nested.clone()))
            .build()
            .await?;

        let hook_list = codex_hooks::list_hooks(codex_hooks::HooksConfig {
            feature_enabled: true,
            config_layer_stack: Some(config.config_layer_stack.clone()),
            ..codex_hooks::HooksConfig::default()
        });
        assert_eq!(
            hook_list.hooks.len(),
            expected_hooks,
            "unexpected discovered hook count for {name}",
        );
        assert!(preview_session_start_hooks(&config).await?.is_empty());
        if expected_hooks == 1 {
            assert_eq!(
                hook_list.hooks[0].trust_status,
                codex_protocol::protocol::HookTrustStatus::Untrusted
            );
        }
    }

    Ok(())
}
