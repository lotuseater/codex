use super::*;

#[tokio::test]
async fn update_memory_settings_persists_and_updates_widget_config() -> Result<()> {
    let (mut app, _app_event_rx, _op_rx) = Box::pin(make_test_app_with_channels()).await;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(&app.config)).await?;

    Box::pin(app.update_memory_settings_with_app_server(
        &mut app_server,
        /*use_memories*/ false,
        /*generate_memories*/ false,
    ))
    .await;

    assert!(!app.config.memories.use_memories);
    assert!(!app.config.memories.generate_memories);
    assert!(!app.chat_widget.config_ref().memories.use_memories);
    assert!(!app.chat_widget.config_ref().memories.generate_memories);

    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    let config_value = toml::from_str::<TomlValue>(&config)?;
    let memories = config_value
        .as_table()
        .and_then(|table| table.get("memories"))
        .and_then(TomlValue::as_table)
        .expect("memories table should exist");
    assert_eq!(
        memories.get("use_memories"),
        Some(&TomlValue::Boolean(false))
    );
    assert_eq!(
        memories.get("generate_memories"),
        Some(&TomlValue::Boolean(false))
    );
    assert!(
        !memories.contains_key("disable_on_external_context")
            && !memories.contains_key("no_memories_if_mcp_or_web_search"),
        "the TUI menu should not write the external-context memory setting"
    );
    app_server.shutdown().await?;
    Ok(())
}

#[test]
fn update_memory_settings_updates_current_thread_memory_mode() -> Result<()> {
    const WORKER_THREADS: usize = 1;
    const TEST_STACK_SIZE_BYTES: usize = 8 * 1024 * 1024;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(WORKER_THREADS)
        .thread_stack_size(TEST_STACK_SIZE_BYTES)
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let (mut app, _app_event_rx, _op_rx) = Box::pin(make_test_app_with_channels()).await;
        let codex_home = tempdir()?;
        app.config.codex_home = codex_home.path().to_path_buf().abs();
        app.config.sqlite_home = codex_home.path().to_path_buf();
        // Seed the previous setting so this test exercises the thread-mode update path.
        app.config.memories.generate_memories = true;

        let mut app_server =
            Box::pin(crate::start_embedded_app_server_for_picker(&app.config)).await?;
        let started = app_server.start_thread(&app.config).await?;
        let thread_id = started.session.thread_id;
        app.active_thread_id = Some(thread_id);

        Box::pin(app.update_memory_settings_with_app_server(
            &mut app_server,
            /*use_memories*/ true,
            /*generate_memories*/ false,
        ))
        .await;

        let state_db = codex_state::StateRuntime::init(
            codex_home.path().to_path_buf(),
            app.config.model_provider_id.clone(),
        )
        .await
        .expect("state db should initialize");
        let memory_mode = state_db
            .get_thread_memory_mode(thread_id)
            .await
            .expect("thread memory mode should be readable");
        assert_eq!(memory_mode.as_deref(), Some("disabled"));

        app_server.shutdown().await?;
        Ok(())
    })
}

#[tokio::test]
async fn reset_memories_clears_local_memory_directories() -> Result<()> {
    Box::pin(async {
        let (mut app, _app_event_rx, _op_rx) = Box::pin(make_test_app_with_channels()).await;
        let codex_home = tempdir()?;
        app.config.codex_home = codex_home.path().to_path_buf().abs();
        app.config.sqlite_home = codex_home.path().to_path_buf();

        let memory_root = codex_home.path().join("memories");
        let extensions_root = memory_root.join("extensions");
        std::fs::create_dir_all(memory_root.join("rollout_summaries"))?;
        std::fs::create_dir_all(&extensions_root)?;
        std::fs::write(memory_root.join("MEMORY.md"), "stale memory\n")?;
        std::fs::write(
            memory_root.join("rollout_summaries").join("stale.md"),
            "stale summary\n",
        )?;
        std::fs::write(extensions_root.join("stale.txt"), "stale extension\n")?;

        let mut app_server =
            Box::pin(crate::start_embedded_app_server_for_picker(&app.config)).await?;

        Box::pin(app.reset_memories_with_app_server(&mut app_server)).await;

        assert_eq!(std::fs::read_dir(&memory_root)?.count(), 0);

        app_server.shutdown().await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn apply_permission_profile_selection_preserves_loader_overrides() -> Result<()> {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let codex_home = tempdir()?;
    let selected_config = codex_home.path().join("work.config.toml");
    std::fs::write(
        &selected_config,
        r#"
default_permissions = "locked-down"

[permissions.locked-down.filesystem]
":minimal" = "read"
"#,
    )?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    app.loader_overrides.user_config_path = Some(selected_config.abs());
    app.harness_overrides.sandbox_mode = Some(SandboxMode::WorkspaceWrite);
    app.harness_overrides.permission_profile = Some(PermissionProfile::workspace_write());

    assert!(
        app.apply_permission_profile_selection(PermissionProfileSelection {
            profile_id: "locked-down".to_string(),
            approval_policy: None,
            approvals_reviewer: None,
            display_label: "locked-down".to_string(),
        })
        .await
    );

    assert_eq!(
        app.config
            .permissions
            .active_permission_profile()
            .as_ref()
            .map(|profile| profile.id.as_str()),
        Some("locked-down")
    );
    assert_eq!(
        app.chat_widget
            .config_ref()
            .permissions
            .active_permission_profile()
            .as_ref()
            .map(|profile| profile.id.as_str()),
        Some("locked-down")
    );
    assert_eq!(
        app.runtime_permission_profile_override,
        Some(RuntimePermissionProfileOverride::from_config(&app.config))
    );
    let op = match app_event_rx.try_recv() {
        Ok(AppEvent::CodexOp(op)) => op,
        other => panic!("expected CodexOp event, got {other:?}"),
    };
    assert_eq!(
        op,
        Op::OverrideTurnContext {
            cwd: None,
            approval_policy: None,
            approvals_reviewer: None,
            permission_profile: Some(app.config.permissions.permission_profile().clone()),
            active_permission_profile: app.config.permissions.active_permission_profile(),
            windows_sandbox_level: None,
            model: None,
            effort: None,
            summary: None,
            service_tier: None,
            collaboration_mode: None,
            personality: None,
        }
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
    assert!(rendered.contains("Permissions updated to locked-down"));
    Ok(())
}
