use super::*;

#[tokio::test]
async fn update_feature_flags_enabling_guardian_selects_auto_review() -> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    let auto_review = auto_review_mode();
    let mut app_server = start_config_write_test_app_server(&app).await?;

    app.update_feature_flags(&mut app_server, vec![(Feature::GuardianApproval, true)])
        .await;

    assert!(app.config.features.enabled(Feature::GuardianApproval));
    assert!(
        app.chat_widget
            .config_ref()
            .features
            .enabled(Feature::GuardianApproval)
    );
    assert_eq!(
        app.config.approvals_reviewer,
        auto_review.approvals_reviewer
    );
    assert_eq!(
        AskForApproval::from(app.config.permissions.approval_policy.value()),
        auto_review.approval_policy
    );
    assert_eq!(
        AskForApproval::from(
            app.chat_widget
                .config_ref()
                .permissions
                .approval_policy
                .value(),
        ),
        auto_review.approval_policy
    );
    assert_eq!(
        app.chat_widget
            .config_ref()
            .permissions
            .permission_profile(),
        &auto_review.permission_profile()
    );
    assert_eq!(
        app.config.permissions.active_permission_profile(),
        Some(auto_review.active_permission_profile.clone())
    );
    assert_eq!(
        app.chat_widget
            .config_ref()
            .permissions
            .active_permission_profile(),
        Some(auto_review.active_permission_profile.clone())
    );
    assert_eq!(
        app.chat_widget.config_ref().approvals_reviewer,
        auto_review.approvals_reviewer
    );
    assert_eq!(app.runtime_approval_policy_override, None);
    assert_eq!(
        app.runtime_permission_profile_override,
        Some(RuntimePermissionProfileOverride::from_config(&app.config))
    );
    assert_eq!(
        op_rx.try_recv(),
        Ok(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: Some(auto_review.approval_policy),
            approvals_reviewer: Some(auto_review.approvals_reviewer),
            permission_profile: Some(auto_review.permission_profile()),
            active_permission_profile: Some(auto_review.active_permission_profile.clone()),
            windows_sandbox_level: None,
            model: None,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
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
    assert!(rendered.contains("Permissions updated to Auto-review"));

    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    assert!(config.contains("guardian_approval = true"));
    assert!(config.contains("approvals_reviewer = \"guardian_subagent\""));
    assert!(config.contains("approval_policy = \"on-request\""));
    assert!(config.contains("sandbox_mode = \"workspace-write\""));
    app_server.shutdown().await?;
    Ok(())
}


#[tokio::test]
async fn update_feature_flags_disabling_guardian_clears_review_policy_and_restores_default()
-> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    let config_toml_path = codex_home.path().join("config.toml").abs();
    let config_toml = "approvals_reviewer = \"guardian_subagent\"\napproval_policy = \"on-request\"\nsandbox_mode = \"workspace-write\"\n\n[features]\nguardian_approval = true\n";
    std::fs::write(config_toml_path.as_path(), config_toml)?;
    let user_config = toml::from_str::<TomlValue>(config_toml)?;
    app.config.config_layer_stack = app
        .config
        .config_layer_stack
        .with_user_config(&config_toml_path, user_config);
    app.config
        .features
        .set_enabled(Feature::GuardianApproval, /*enabled*/ true)?;
    app.chat_widget
        .set_feature_enabled(Feature::GuardianApproval, /*enabled*/ true);
    app.config.approvals_reviewer = ApprovalsReviewer::AutoReview;
    app.chat_widget
        .set_approvals_reviewer(ApprovalsReviewer::AutoReview);
    app.config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())?;
    app.config
        .permissions
        .set_permission_profile(PermissionProfile::workspace_write())?;
    app.chat_widget
        .set_approval_policy(AskForApproval::OnRequest);
    app.chat_widget
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::legacy(
            PermissionProfile::workspace_write(),
        ))?;
    let mut app_server = start_config_write_test_app_server(&app).await?;

    app.update_feature_flags(&mut app_server, vec![(Feature::GuardianApproval, false)])
        .await;

    assert!(!app.config.features.enabled(Feature::GuardianApproval));
    assert!(
        !app.chat_widget
            .config_ref()
            .features
            .enabled(Feature::GuardianApproval)
    );
    assert_eq!(app.config.approvals_reviewer, ApprovalsReviewer::User);
    assert_eq!(
        AskForApproval::from(app.config.permissions.approval_policy.value()),
        AskForApproval::OnRequest
    );
    assert_eq!(
        app.chat_widget.config_ref().approvals_reviewer,
        ApprovalsReviewer::User
    );
    assert_eq!(app.runtime_approval_policy_override, None);
    assert_eq!(
        op_rx.try_recv(),
        Ok(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: None,
            approvals_reviewer: Some(ApprovalsReviewer::User),
            permission_profile: None,
            active_permission_profile: None,
            windows_sandbox_level: None,
            model: None,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
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
    assert!(rendered.contains("Permissions updated to Default"));

    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    assert!(!config.contains("guardian_approval = true"));
    assert!(!config.contains("approvals_reviewer ="));
    assert!(config.contains("approval_policy = \"on-request\""));
    assert!(config.contains("sandbox_mode = \"workspace-write\""));
    app_server.shutdown().await?;
    Ok(())
}


#[tokio::test]
async fn update_feature_flags_enabling_guardian_overrides_explicit_manual_review_policy()
-> Result<()> {
    let (mut app, _app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    let auto_review = auto_review_mode();
    let config_toml_path = codex_home.path().join("config.toml").abs();
    let config_toml = "approvals_reviewer = \"user\"\n";
    std::fs::write(config_toml_path.as_path(), config_toml)?;
    let user_config = toml::from_str::<TomlValue>(config_toml)?;
    app.config.config_layer_stack = app
        .config
        .config_layer_stack
        .with_user_config(&config_toml_path, user_config);
    app.config.approvals_reviewer = ApprovalsReviewer::User;
    app.chat_widget
        .set_approvals_reviewer(ApprovalsReviewer::User);
    let mut app_server = start_config_write_test_app_server(&app).await?;

    app.update_feature_flags(&mut app_server, vec![(Feature::GuardianApproval, true)])
        .await;

    assert!(app.config.features.enabled(Feature::GuardianApproval));
    assert_eq!(
        app.config.approvals_reviewer,
        auto_review.approvals_reviewer
    );
    assert_eq!(
        app.chat_widget.config_ref().approvals_reviewer,
        auto_review.approvals_reviewer
    );
    assert_eq!(
        AskForApproval::from(app.config.permissions.approval_policy.value()),
        auto_review.approval_policy
    );
    assert_eq!(
        app.chat_widget
            .config_ref()
            .permissions
            .permission_profile(),
        &auto_review.permission_profile()
    );
    assert_eq!(
        op_rx.try_recv(),
        Ok(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: Some(auto_review.approval_policy),
            approvals_reviewer: Some(auto_review.approvals_reviewer),
            permission_profile: Some(auto_review.permission_profile()),
            active_permission_profile: Some(auto_review.active_permission_profile.clone()),
            windows_sandbox_level: None,
            model: None,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
    );

    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    assert!(config.contains("approvals_reviewer = \"guardian_subagent\""));
    assert!(config.contains("guardian_approval = true"));
    assert!(config.contains("approval_policy = \"on-request\""));
    assert!(config.contains("sandbox_mode = \"workspace-write\""));
    app_server.shutdown().await?;
    Ok(())
}


#[tokio::test]
async fn update_feature_flags_disabling_guardian_clears_manual_review_policy_without_history()
-> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    let config_toml_path = codex_home.path().join("config.toml").abs();
    let config_toml = "approvals_reviewer = \"user\"\napproval_policy = \"on-request\"\nsandbox_mode = \"workspace-write\"\n\n[features]\nguardian_approval = true\n";
    std::fs::write(config_toml_path.as_path(), config_toml)?;
    let user_config = toml::from_str::<TomlValue>(config_toml)?;
    app.config.config_layer_stack = app
        .config
        .config_layer_stack
        .with_user_config(&config_toml_path, user_config);
    app.config
        .features
        .set_enabled(Feature::GuardianApproval, /*enabled*/ true)?;
    app.chat_widget
        .set_feature_enabled(Feature::GuardianApproval, /*enabled*/ true);
    app.config.approvals_reviewer = ApprovalsReviewer::User;
    app.chat_widget
        .set_approvals_reviewer(ApprovalsReviewer::User);
    let mut app_server = start_config_write_test_app_server(&app).await?;

    app.update_feature_flags(&mut app_server, vec![(Feature::GuardianApproval, false)])
        .await;

    assert!(!app.config.features.enabled(Feature::GuardianApproval));
    assert_eq!(app.config.approvals_reviewer, ApprovalsReviewer::User);
    assert_eq!(
        app.chat_widget.config_ref().approvals_reviewer,
        ApprovalsReviewer::User
    );
    assert_eq!(
        op_rx.try_recv(),
        Ok(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: None,
            approvals_reviewer: Some(ApprovalsReviewer::User),
            permission_profile: None,
            active_permission_profile: None,
            windows_sandbox_level: None,
            model: None,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
    );
    assert!(
        app_event_rx.try_recv().is_err(),
        "manual review should not emit a permissions history update when the effective state stays default"
    );

    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    assert!(!config.contains("guardian_approval = true"));
    assert!(!config.contains("approvals_reviewer ="));
    app_server.shutdown().await?;
    Ok(())
}


#[tokio::test]
async fn update_feature_flags_enabling_guardian_in_profile_sets_profile_auto_review_policy()
-> Result<()> {
    let (mut app, _app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    let auto_review = auto_review_mode();
    app.active_profile = Some("guardian".to_string());
    let config_toml_path = codex_home.path().join("config.toml").abs();
    let config_toml = "profile = \"guardian\"\napprovals_reviewer = \"user\"\n";
    std::fs::write(config_toml_path.as_path(), config_toml)?;
    let user_config = toml::from_str::<TomlValue>(config_toml)?;
    app.config.config_layer_stack = app
        .config
        .config_layer_stack
        .with_user_config(&config_toml_path, user_config);
    app.config.approvals_reviewer = ApprovalsReviewer::User;
    app.chat_widget
        .set_approvals_reviewer(ApprovalsReviewer::User);

    app.update_feature_flags(vec![(Feature::GuardianApproval, true)])
        .await;

    assert!(app.config.features.enabled(Feature::GuardianApproval));
    assert_eq!(
        app.config.approvals_reviewer,
        auto_review.approvals_reviewer
    );
    assert_eq!(
        app.chat_widget.config_ref().approvals_reviewer,
        auto_review.approvals_reviewer
    );
    assert_eq!(
        op_rx.try_recv(),
        Ok(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: Some(auto_review.approval_policy),
            approvals_reviewer: Some(auto_review.approvals_reviewer),
            active_permission_profile: Some(auto_review.active_permission_profile.clone()),
            windows_sandbox_level: None,
            model: None,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
    );

    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    let config_value = toml::from_str::<TomlValue>(&config)?;
    let profile_config = config_value
        .as_table()
        .and_then(|table| table.get("profiles"))
        .and_then(TomlValue::as_table)
        .and_then(|profiles| profiles.get("guardian"))
        .and_then(TomlValue::as_table)
        .expect("guardian profile should exist");
    assert_eq!(
        config_value
            .as_table()
            .and_then(|table| table.get("approvals_reviewer")),
        Some(&TomlValue::String("user".to_string()))
    );
    assert_eq!(
        profile_config.get("approvals_reviewer"),
        Some(&TomlValue::String("guardian_subagent".to_string()))
    );
    Ok(())
}


#[tokio::test]
async fn update_feature_flags_disabling_guardian_in_profile_allows_inherited_user_reviewer()
-> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    app.active_profile = Some("guardian".to_string());
    let config_toml_path = codex_home.path().join("config.toml").abs();
    let config_toml = r#"
profile = "guardian"
approvals_reviewer = "user"

[profiles.guardian]
approvals_reviewer = "guardian_subagent"

[profiles.guardian.features]
guardian_approval = true
"#;
    std::fs::write(config_toml_path.as_path(), config_toml)?;
    let user_config = toml::from_str::<TomlValue>(config_toml)?;
    app.config.config_layer_stack = app
        .config
        .config_layer_stack
        .with_user_config(&config_toml_path, user_config);
    app.config
        .features
        .set_enabled(Feature::GuardianApproval, /*enabled*/ true)?;
    app.chat_widget
        .set_feature_enabled(Feature::GuardianApproval, /*enabled*/ true);
    app.config.approvals_reviewer = ApprovalsReviewer::AutoReview;
    app.chat_widget
        .set_approvals_reviewer(ApprovalsReviewer::AutoReview);

    app.update_feature_flags(vec![(Feature::GuardianApproval, false)])
        .await;

    assert!(!app.config.features.enabled(Feature::GuardianApproval));
    assert!(
        !app.chat_widget
            .config_ref()
            .features
            .enabled(Feature::GuardianApproval)
    );
    assert_eq!(app.config.approvals_reviewer, ApprovalsReviewer::User);
    assert_eq!(
        app.chat_widget.config_ref().approvals_reviewer,
        ApprovalsReviewer::User
    );
    assert_eq!(
        op_rx.try_recv(),
        Ok(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: None,
            approvals_reviewer: Some(ApprovalsReviewer::User),
            active_permission_profile: None,
            windows_sandbox_level: None,
            model: None,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
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
    assert!(rendered.contains("Permissions updated to Default"));

    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    assert!(!config.contains("guardian_approval = true"));
    assert!(!config.contains("guardian_subagent"));
    assert_eq!(
        toml::from_str::<TomlValue>(&config)?
            .as_table()
            .and_then(|table| table.get("approvals_reviewer")),
        Some(&TomlValue::String("user".to_string()))
    );
    Ok(())
}


#[tokio::test]
async fn update_feature_flags_disabling_guardian_in_profile_keeps_inherited_non_user_reviewer_enabled()
-> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    app.active_profile = Some("guardian".to_string());
    let config_toml_path = codex_home.path().join("config.toml").abs();
    let config_toml = "profile = \"guardian\"\napprovals_reviewer = \"guardian_subagent\"\n\n[features]\nguardian_approval = true\n";
    std::fs::write(config_toml_path.as_path(), config_toml)?;
    let user_config = toml::from_str::<TomlValue>(config_toml)?;
    app.config.config_layer_stack = app
        .config
        .config_layer_stack
        .with_user_config(&config_toml_path, user_config);
    app.config
        .features
        .set_enabled(Feature::GuardianApproval, /*enabled*/ true)?;
    app.chat_widget
        .set_feature_enabled(Feature::GuardianApproval, /*enabled*/ true);
    app.config.approvals_reviewer = ApprovalsReviewer::AutoReview;
    app.chat_widget
        .set_approvals_reviewer(ApprovalsReviewer::AutoReview);

    app.update_feature_flags(vec![(Feature::GuardianApproval, false)])
        .await;

    assert!(app.config.features.enabled(Feature::GuardianApproval));
    assert!(
        app.chat_widget
            .config_ref()
            .features
            .enabled(Feature::GuardianApproval)
    );
    assert_eq!(app.config.approvals_reviewer, ApprovalsReviewer::AutoReview);
    assert_eq!(
        app.chat_widget.config_ref().approvals_reviewer,
        ApprovalsReviewer::AutoReview
    );
    assert!(
        op_rx.try_recv().is_err(),
        "disabling an inherited non-user reviewer should not patch the active session"
    );
    let app_events = std::iter::from_fn(|| app_event_rx.try_recv().ok()).collect::<Vec<_>>();
    assert!(
        !app_events.iter().any(|event| match event {
            AppEvent::InsertHistoryCell(cell) => cell
                .display_lines(/*width*/ 120)
                .iter()
                .any(|line| line.to_string().contains("Permissions updated to")),
            _ => false,
        }),
        "blocking disable with inherited guardian review should not emit a permissions history update: {app_events:?}"
    );

    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    assert!(config.contains("guardian_approval = true"));
    assert_eq!(
        toml::from_str::<TomlValue>(&config)?
            .as_table()
            .and_then(|table| table.get("approvals_reviewer")),
        Some(&TomlValue::String("guardian_subagent".to_string()))
    );
    Ok(())
}
