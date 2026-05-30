use super::*;

#[test]
fn get_service_tier_does_not_use_model_default_when_absent_and_fast_mode_enabled() {
    let model_info = model_with_default_service_tier(Some(ServiceTier::Fast.request_value()));

    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        None
    );
}

#[test]
fn get_service_tier_does_not_use_model_default_when_fast_mode_disabled() {
    let model_info = model_with_default_service_tier(Some(ServiceTier::Fast.request_value()));

    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_mode_enabled*/ false,
            &model_info,
        ),
        None
    );
}

#[test]
fn get_service_tier_keeps_supported_explicit_tier() {
    let model_info = model_with_default_service_tier(Some(ServiceTier::Fast.request_value()));

    assert_eq!(
        get_service_tier(
            Some(ServiceTier::Fast.request_value().to_string()),
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
}

#[test]
fn get_service_tier_does_not_default_when_model_has_no_default() {
    let model_info = model_with_default_service_tier(/*default_service_tier*/ None);

    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        None
    );
}

#[test]
fn get_service_tier_drops_unsupported_configured_tier_when_fast_mode_enabled() {
    let model_info = model_with_default_service_tier(Some(ServiceTier::Fast.request_value()));

    assert_eq!(
        get_service_tier(
            Some("unsupported".to_string()),
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            Some(ServiceTier::Flex.request_value().to_string()),
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string()),
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string())
    );
}

#[test]
fn get_service_tier_ignores_configured_tier_when_fast_mode_disabled() {
    let model_info = model_with_default_service_tier(Some(ServiceTier::Fast.request_value()));

    assert_eq!(
        get_service_tier(
            Some(ServiceTier::Fast.request_value().to_string()),
            /*fast_mode_enabled*/ false,
            &model_info,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string()),
            /*fast_mode_enabled*/ false,
            &model_info,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            Some("unsupported".to_string()),
            /*fast_mode_enabled*/ false,
            &model_info,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_mode_enabled*/ false,
            &model_info,
        ),
        None
    );
}

#[tokio::test]
async fn session_settings_null_service_tier_update_uses_default_service_tier() {
    let session_configuration = make_session_configuration_for_tests().await;

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            service_tier: Some(None),
            ..Default::default()
        })
        .expect("null service tier update should apply");

    assert_eq!(
        updated.service_tier,
        Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string())
    );
}

#[tokio::test]
async fn session_settings_legacy_fast_service_tier_update_uses_priority_request_value() {
    let session_configuration = make_session_configuration_for_tests().await;

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            service_tier: Some(Some("fast".to_string())),
            ..Default::default()
        })
        .expect("legacy fast service tier update should apply");

    assert_eq!(
        updated.service_tier,
        Some(ServiceTier::Fast.request_value().to_string())
    );
}

pub(crate) async fn make_session_configuration_for_tests() -> SessionConfiguration {
    let codex_home = tempfile::tempdir().expect("create temp dir");
    let config = build_test_config(codex_home.path()).await;
    let config = Arc::new(config);
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let reasoning_effort = config.model_reasoning_effort;
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort,
            developer_instructions: None,
        },
    };

    SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode: collaboration_mode.clone(),
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        context_budget_mode: config.context_budget_mode,
        personality: config.personality,
        fork_features: ForkFeaturesState::new(
            collaboration_mode,
            config.context_budget_mode,
            config.personality,
        ),
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile: config.permissions.permission_profile.clone(),
        active_permission_profile: config.permissions.active_permission_profile(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: vec![config.cwd.clone()],
        profile_workspace_roots: Vec::new(),
        codex_home: config.codex_home.clone(),
        thread_name: None,
        environments: Vec::new(),
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        thread_source: None,
        dynamic_tools: Vec::new(),
        persist_extended_history: false,
        inherited_shell_snapshot: None,
        user_shell_override: None,
    }
}
