use super::*;

#[test]
fn validated_network_policy_amendment_host_allows_normalized_match() {
    let amendment = NetworkPolicyAmendment {
        host: "ExAmPlE.Com.:443".to_string(),
        action: NetworkPolicyRuleAction::Allow,
    };
    let context = NetworkApprovalContext {
        host: "example.com".to_string(),
        protocol: NetworkApprovalProtocol::Https,
    };

    let host = Session::validated_network_policy_amendment_host(&amendment, &context)
        .expect("normalized hosts should match");

    assert_eq!(host, "example.com");
}

#[test]
fn validated_network_policy_amendment_host_rejects_mismatch() {
    let amendment = NetworkPolicyAmendment {
        host: "evil.example.com".to_string(),
        action: NetworkPolicyRuleAction::Deny,
    };
    let context = NetworkApprovalContext {
        host: "api.example.com".to_string(),
        protocol: NetworkApprovalProtocol::Https,
    };

    let err = Session::validated_network_policy_amendment_host(&amendment, &context)
        .expect_err("mismatched hosts should be rejected");

    let message = err.to_string();
    assert!(message.contains("does not match approved host"));
}

#[tokio::test]
async fn start_managed_network_proxy_applies_execpolicy_network_rules() -> anyhow::Result<()> {
    let spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        /*requirements*/ None,
        &permission_profile_for_sandbox_policy(&SandboxPolicy::new_workspace_write_policy()),
    )?;
    let mut exec_policy = Policy::empty();
    exec_policy.add_network_rule(
        "example.com",
        NetworkRuleProtocol::Https,
        Decision::Allow,
        /*justification*/ None,
    )?;

    let (started_proxy, _) = Session::start_managed_network_proxy(
        &spec,
        &exec_policy,
        &permission_profile_for_sandbox_policy(&SandboxPolicy::new_workspace_write_policy()),
        /*network_policy_decider*/ None,
        /*blocked_request_observer*/ None,
        /*managed_network_requirements_enabled*/ false,
        crate::config::NetworkProxyAuditMetadata::default(),
    )
    .await?;

    let current_cfg = started_proxy.proxy().current_cfg().await?;
    assert_eq!(
        current_cfg.network.allowed_domains(),
        Some(vec!["example.com".to_string()])
    );
    Ok(())
}

#[tokio::test]
async fn start_managed_network_proxy_ignores_invalid_execpolicy_network_rules() -> anyhow::Result<()>
{
    let spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            domains: Some(NetworkDomainPermissionsToml {
                entries: std::collections::BTreeMap::from([(
                    "managed.example.com".to_string(),
                    NetworkDomainPermissionToml::Allow,
                )]),
            }),
            managed_allowed_domains_only: Some(true),
            ..Default::default()
        }),
        &permission_profile_for_sandbox_policy(&SandboxPolicy::new_workspace_write_policy()),
    )?;
    let mut exec_policy = Policy::empty();
    exec_policy.add_network_rule(
        "example.com",
        NetworkRuleProtocol::Https,
        Decision::Allow,
        /*justification*/ None,
    )?;

    let (started_proxy, _) = Session::start_managed_network_proxy(
        &spec,
        &exec_policy,
        &permission_profile_for_sandbox_policy(&SandboxPolicy::new_workspace_write_policy()),
        /*network_policy_decider*/ None,
        /*blocked_request_observer*/ None,
        /*managed_network_requirements_enabled*/ false,
        crate::config::NetworkProxyAuditMetadata::default(),
    )
    .await?;

    let current_cfg = started_proxy.proxy().current_cfg().await?;
    assert_eq!(
        current_cfg.network.allowed_domains(),
        Some(vec!["managed.example.com".to_string()])
    );
    Ok(())
}

#[tokio::test]
async fn managed_network_proxy_decider_survives_full_access_start() -> anyhow::Result<()> {
    let spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            enabled: Some(true),
            ..Default::default()
        }),
        &permission_profile_for_sandbox_policy(&SandboxPolicy::DangerFullAccess),
    )?;
    let exec_policy = Policy::empty();
    let decider_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let network_policy_decider: Arc<dyn codex_network_proxy::NetworkPolicyDecider> = Arc::new({
        let decider_calls = Arc::clone(&decider_calls);
        move |_request| {
            decider_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async { codex_network_proxy::NetworkDecision::ask("not_allowed") }
        }
    });

    let (started_proxy, _) = Session::start_managed_network_proxy(
        &spec,
        &exec_policy,
        &permission_profile_for_sandbox_policy(&SandboxPolicy::DangerFullAccess),
        Some(network_policy_decider),
        /*blocked_request_observer*/ None,
        /*managed_network_requirements_enabled*/ true,
        crate::config::NetworkProxyAuditMetadata::default(),
    )
    .await?;

    let spec = spec.recompute_for_permission_profile(&permission_profile_for_sandbox_policy(
        &SandboxPolicy::new_workspace_write_policy(),
    ))?;
    spec.apply_to_started_proxy(&started_proxy).await?;
    let current_cfg = started_proxy.proxy().current_cfg().await?;
    assert_eq!(current_cfg.network.allowed_domains(), None);

    use tokio::io::AsyncReadExt as _;
    use tokio::io::AsyncWriteExt as _;

    let mut stream = tokio::net::TcpStream::connect(started_proxy.proxy().http_addr()).await?;
    stream
        .write_all(
            b"GET http://example.com/ HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n",
        )
        .await?;
    let mut buffer = [0_u8; 4096];
    let bytes_read = tokio::time::timeout(StdDuration::from_secs(2), stream.read(&mut buffer))
        .await
        .expect("timed out waiting for proxy response")?;
    let response = String::from_utf8_lossy(&buffer[..bytes_read]);

    assert!(
        response.starts_with("HTTP/1.1 403 Forbidden"),
        "unexpected proxy response: {response}"
    );
    assert!(
        response.contains("x-proxy-error: blocked-by-allowlist"),
        "unexpected proxy response: {response}"
    );
    assert_eq!(
        decider_calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "unexpected proxy response: {response}"
    );
    Ok(())
}

#[tokio::test]
async fn new_turn_refreshes_managed_network_proxy_for_sandbox_change() -> anyhow::Result<()> {
    let (mut session, _turn_context) = make_session_and_context().await;
    let initial_policy = SandboxPolicy::new_workspace_write_policy();

    let mut network_config = NetworkProxyConfig::default();
    network_config
        .network
        .set_allowed_domains(vec!["evil.com".to_string()]);
    let requirements = NetworkConstraints {
        domains: Some(NetworkDomainPermissionsToml {
            entries: std::collections::BTreeMap::from([(
                "*.example.com".to_string(),
                NetworkDomainPermissionToml::Allow,
            )]),
        }),
        ..Default::default()
    };
    let spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        network_config,
        Some(requirements),
        &permission_profile_for_sandbox_policy(&initial_policy),
    )?;
    let (started_proxy, _) = Session::start_managed_network_proxy(
        &spec,
        &Policy::empty(),
        &permission_profile_for_sandbox_policy(&initial_policy),
        /*network_policy_decider*/ None,
        /*blocked_request_observer*/ None,
        /*managed_network_requirements_enabled*/ false,
        crate::config::NetworkProxyAuditMetadata::default(),
    )
    .await?;
    assert_eq!(
        started_proxy
            .proxy()
            .current_cfg()
            .await?
            .network
            .allowed_domains(),
        Some(vec!["*.example.com".to_string(), "evil.com".to_string()])
    );

    {
        let mut state = session.state.lock().await;
        let mut config = (*state.session_configuration.original_config_do_not_use).clone();
        config.permissions.network = Some(spec);
        let cwd = config.cwd.clone();
        config
            .permissions
            .set_legacy_sandbox_policy(initial_policy.clone(), cwd.as_path())
            .expect("test setup should allow sandbox policy");
        state.session_configuration.original_config_do_not_use = Arc::new(config);
        state
            .session_configuration
            .permission_profile
            .set(PermissionProfile::from_legacy_sandbox_policy(
                &initial_policy,
            ))
            .expect("test setup should allow permission profile");
    }
    session.services.network_proxy = Some(started_proxy);

    session
        .new_turn_with_sub_id(
            "sandbox-policy-change".to_string(),
            SessionSettingsUpdate {
                sandbox_policy: Some(SandboxPolicy::DangerFullAccess),
                ..Default::default()
            },
        )
        .await?;

    let started_proxy = session
        .services
        .network_proxy
        .as_ref()
        .expect("managed network proxy should be present");
    assert_eq!(
        started_proxy
            .proxy()
            .current_cfg()
            .await?
            .network
            .allowed_domains(),
        Some(vec!["*.example.com".to_string()])
    );

    Ok(())
}

#[tokio::test]
async fn danger_full_access_turns_do_not_expose_managed_network_proxy() -> anyhow::Result<()> {
    let network_spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            enabled: Some(true),
            ..Default::default()
        }),
        &permission_profile_for_sandbox_policy(&SandboxPolicy::DangerFullAccess),
    )?;

    let session = make_session_with_config(move |config| {
        let cwd = config.cwd.clone();
        config
            .permissions
            .set_legacy_sandbox_policy(SandboxPolicy::DangerFullAccess, cwd.as_path())
            .expect("test setup should allow sandbox policy");
        config.permissions.network = Some(network_spec);
    })
    .await?;

    let turn_context = session.new_default_turn().await;
    assert!(turn_context.network.is_none());
    Ok(())
}

#[tokio::test]
async fn danger_full_access_tool_attempts_do_not_enforce_managed_network() -> anyhow::Result<()> {
    #[derive(Default)]
    struct ProbeToolRuntime {
        enforce_managed_network: Vec<bool>,
    }

    impl crate::tools::sandboxing::Approvable<()> for ProbeToolRuntime {
        type ApprovalKey = String;

        fn approval_keys(&self, _req: &()) -> Vec<Self::ApprovalKey> {
            vec!["probe".to_string()]
        }

        fn start_approval_async<'a>(
            &'a mut self,
            _req: &'a (),
            _ctx: crate::tools::sandboxing::ApprovalCtx<'a>,
        ) -> futures::future::BoxFuture<'a, ReviewDecision> {
            Box::pin(async { ReviewDecision::Approved })
        }
    }

    impl crate::tools::sandboxing::Sandboxable for ProbeToolRuntime {
        fn sandbox_preference(&self) -> codex_sandboxing::SandboxablePreference {
            codex_sandboxing::SandboxablePreference::Auto
        }
    }

    impl crate::tools::sandboxing::ToolRuntime<(), ()> for ProbeToolRuntime {
        async fn run(
            &mut self,
            _req: &(),
            attempt: &crate::tools::sandboxing::SandboxAttempt<'_>,
            _ctx: &crate::tools::sandboxing::ToolCtx,
        ) -> Result<(), crate::tools::sandboxing::ToolError> {
            self.enforce_managed_network
                .push(attempt.enforce_managed_network);
            Ok(())
        }
    }

    let network_spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            enabled: Some(true),
            ..Default::default()
        }),
        &permission_profile_for_sandbox_policy(&SandboxPolicy::DangerFullAccess),
    )?;

    let session = make_session_with_config(move |config| {
        let cwd = config.cwd.clone();
        config
            .permissions
            .set_legacy_sandbox_policy(SandboxPolicy::DangerFullAccess, cwd.as_path())
            .expect("test setup should allow sandbox policy");
        config.permissions.network = Some(network_spec);

        let layers = config
            .config_layer_stack
            .get_layers(
                ConfigLayerStackOrdering::LowestPrecedenceFirst,
                /*include_disabled*/ true,
            )
            .into_iter()
            .cloned()
            .collect();
        let mut requirements = config.config_layer_stack.requirements().clone();
        requirements.network = Some(Sourced::new(
            NetworkConstraints {
                enabled: Some(true),
                ..Default::default()
            },
            RequirementSource::CloudRequirements,
        ));
        let mut requirements_toml = config.config_layer_stack.requirements_toml().clone();
        requirements_toml.network = Some(codex_config::NetworkRequirementsToml {
            enabled: Some(true),
            ..Default::default()
        });
        config.config_layer_stack = ConfigLayerStack::new(layers, requirements, requirements_toml)
            .expect("rebuild config layer stack with network requirements");
    })
    .await?;

    let turn = session.new_default_turn().await;
    assert!(turn.network.is_none());

    let mut orchestrator = crate::tools::orchestrator::ToolOrchestrator::new();
    let mut tool = ProbeToolRuntime::default();
    let tool_ctx = crate::tools::sandboxing::ToolCtx {
        session: Arc::clone(&session),
        turn: Arc::clone(&turn),
        call_id: "probe-call".to_string(),
        tool_name: plain_tool_name("probe"),
    };

    orchestrator
        .run(
            &mut tool,
            &(),
            &tool_ctx,
            turn.as_ref(),
            AskForApproval::Never,
        )
        .await
        .expect("probe runtime should succeed");

    assert_eq!(tool.enforce_managed_network, vec![false]);

    Ok(())
}

#[tokio::test]
async fn workspace_write_turns_continue_to_expose_managed_network_proxy() -> anyhow::Result<()> {
    let sandbox_policy = SandboxPolicy::new_workspace_write_policy();
    let network_spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            enabled: Some(true),
            ..Default::default()
        }),
        &permission_profile_for_sandbox_policy(&sandbox_policy),
    )?;

    let session = make_session_with_config(move |config| {
        let cwd = config.cwd.clone();
        config
            .permissions
            .set_legacy_sandbox_policy(sandbox_policy, cwd.as_path())
            .expect("test setup should allow sandbox policy");
        config.permissions.network = Some(network_spec);
    })
    .await?;

    let turn_context = session.new_default_turn().await;
    assert!(turn_context.network.is_some());
    Ok(())
}

#[tokio::test]
async fn user_shell_commands_do_not_inherit_managed_network_proxy() -> anyhow::Result<()> {
    let sandbox_policy = SandboxPolicy::new_workspace_write_policy();
    let network_spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            enabled: Some(true),
            ..Default::default()
        }),
        &permission_profile_for_sandbox_policy(&sandbox_policy),
    )?;

    let (session, rx) = make_session_with_config_and_rx(move |config| {
        let cwd = config.cwd.clone();
        config
            .permissions
            .set_legacy_sandbox_policy(sandbox_policy, cwd.as_path())
            .expect("test setup should allow sandbox policy");
        config.permissions.network = Some(network_spec);
    })
    .await?;

    let turn_context = session.new_default_turn().await;
    assert!(turn_context.network.is_some());

    #[cfg(windows)]
    let command = r#"$val = $env:HTTP_PROXY; if ([string]::IsNullOrEmpty($val)) { $val = 'not-set' } ; [System.Console]::Write($val)"#.to_string();
    #[cfg(not(windows))]
    let command = r#"sh -c "printf '%s' \"${HTTP_PROXY:-not-set}\"""#.to_string();

    execute_user_shell_command(
        Arc::clone(&session),
        turn_context,
        command,
        CancellationToken::new(),
        UserShellCommandMode::StandaloneTurn,
    )
    .await;

    loop {
        let event = rx.recv().await.expect("channel open");
        if let EventMsg::ExecCommandEnd(event) = event.msg {
            assert_eq!(event.exit_code, 0);
            assert_eq!(event.stdout.trim(), "not-set");
            break;
        }
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_configured_reports_permission_profile_for_external_sandbox() -> anyhow::Result<()>
{
    let server = start_mock_server().await;
    let sandbox_policy = SandboxPolicy::ExternalSandbox {
        network_access: codex_protocol::protocol::NetworkAccess::Restricted,
    };
    let expected_sandbox_policy = sandbox_policy.clone();
    let mut builder = test_codex().with_config(move |config| {
        config.permissions.permission_profile = codex_config::Constrained::allow_any(
            PermissionProfile::from_legacy_sandbox_policy(&sandbox_policy),
        );
        config
            .set_legacy_sandbox_policy(sandbox_policy)
            .expect("set sandbox policy");
    });

    let test = builder.build(&server).await?;

    let expected_permission_profile =
        codex_protocol::models::PermissionProfile::from_legacy_sandbox_policy(
            &expected_sandbox_policy,
        );
    assert_eq!(
        test.session_configured.permission_profile, expected_permission_profile,
        "ExternalSandbox is represented explicitly instead of as a lossy root-write profile"
    );
    Ok(())
}

#[tokio::test]
async fn session_configuration_apply_preserves_profile_file_system_policy_on_cwd_only_update() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let workspace = tempfile::tempdir().expect("create temp dir");
    let project_root = workspace.path().join("project");
    let original_cwd = project_root.join("subdir");
    let docs_dir = original_cwd.join("docs");
    std::fs::create_dir_all(&docs_dir).expect("create docs dir");
    let docs_dir = docs_dir.abs();

    session_configuration.cwd = original_cwd.abs();
    let sandbox_policy = SandboxPolicy::WorkspaceWrite {
        writable_roots: Vec::new(),
        network_access: false,
        exclude_tmpdir_env_var: true,
        exclude_slash_tmp: true,
    };
    let file_system_sandbox_policy = FileSystemSandboxPolicy::restricted(vec![
        FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
            },
            access: FileSystemAccessMode::Write,
        },
        FileSystemSandboxEntry {
            path: FileSystemPath::Path { path: docs_dir },
            access: FileSystemAccessMode::Read,
        },
    ]);
    let network_sandbox_policy = NetworkSandboxPolicy::from(&sandbox_policy);
    session_configuration.permission_profile = codex_config::Constrained::allow_any(
        PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::from_legacy_sandbox_policy(&sandbox_policy),
            &file_system_sandbox_policy,
            network_sandbox_policy,
        ),
    );

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            cwd: Some(project_root),
            ..Default::default()
        })
        .expect("cwd-only update should succeed");

    assert_eq!(
        updated.file_system_sandbox_policy(),
        file_system_sandbox_policy
    );
}

#[tokio::test]
async fn session_configuration_apply_permission_profile_preserves_existing_deny_read_entries() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let cwd = tempfile::tempdir().expect("create temp dir");
    session_configuration.cwd = cwd.path().abs();

    let workspace_policy = SandboxPolicy::new_workspace_write_policy();
    let deny_entry = FileSystemSandboxEntry {
        path: FileSystemPath::GlobPattern {
            pattern: "**/*.env".to_string(),
        },
        access: FileSystemAccessMode::None,
    };
    let mut existing_file_system_policy =
        FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(
            &workspace_policy,
            session_configuration.cwd.as_path(),
        );
    existing_file_system_policy.glob_scan_max_depth = Some(2);
    existing_file_system_policy.entries.push(deny_entry.clone());
    session_configuration.permission_profile = codex_config::Constrained::allow_any(
        PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::from_legacy_sandbox_policy(&workspace_policy),
            &existing_file_system_policy,
            NetworkSandboxPolicy::Restricted,
        ),
    );

    let requested_file_system_policy = FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(
        &workspace_policy,
        session_configuration.cwd.as_path(),
    );
    let permission_profile = codex_protocol::models::PermissionProfile::from_runtime_permissions(
        &requested_file_system_policy,
        NetworkSandboxPolicy::Restricted,
    );
    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            permission_profile: Some(permission_profile),
            ..Default::default()
        })
        .expect("permission profile update should succeed");

    let mut expected_file_system_policy = requested_file_system_policy;
    expected_file_system_policy.glob_scan_max_depth = Some(2);
    expected_file_system_policy.entries.push(deny_entry);
    assert_eq!(
        updated.file_system_sandbox_policy(),
        expected_file_system_policy
    );
}

#[tokio::test]
async fn session_configuration_apply_permission_profile_accepts_direct_write_roots() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let cwd = tempfile::tempdir().expect("create cwd");
    session_configuration.cwd = cwd.path().abs();
    let external_write_dir = tempfile::tempdir().expect("create external write root");
    let external_write_path = AbsolutePathBuf::from_absolute_path(
        codex_utils_absolute_path::canonicalize_preserving_symlinks(external_write_dir.path())
            .expect("canonical temp dir"),
    )
    .expect("canonical temp dir should be absolute");
    let file_system_sandbox_policy =
        FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
            path: FileSystemPath::Path {
                path: external_write_path.clone(),
            },
            access: FileSystemAccessMode::Write,
        }]);
    let permission_profile = PermissionProfile::from_runtime_permissions(
        &file_system_sandbox_policy,
        NetworkSandboxPolicy::Restricted,
    );

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            permission_profile: Some(permission_profile.clone()),
            ..Default::default()
        })
        .expect("permission profile update should accept direct runtime permissions");

    assert_eq!(updated.permission_profile(), permission_profile);
    assert_eq!(
        updated.file_system_sandbox_policy(),
        file_system_sandbox_policy
    );
    assert_eq!(
        updated.sandbox_policy(),
        SandboxPolicy::WorkspaceWrite {
            writable_roots: vec![external_write_path],
            network_access: false,
            exclude_tmpdir_env_var: true,
            exclude_slash_tmp: true,
        }
    );
}

#[tokio::test]
async fn session_configuration_apply_rederives_legacy_file_system_policy_on_cwd_update() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let workspace = tempfile::tempdir().expect("create temp dir");
    let project_root = workspace.path().join("project");
    let original_cwd = project_root.join("subdir");
    session_configuration.cwd = original_cwd.abs();
    let sandbox_policy = SandboxPolicy::WorkspaceWrite {
        writable_roots: Vec::new(),
        network_access: false,
        exclude_tmpdir_env_var: true,
        exclude_slash_tmp: true,
    };
    let file_system_sandbox_policy = FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(
        &sandbox_policy,
        &session_configuration.cwd,
    );
    session_configuration.permission_profile = codex_config::Constrained::allow_any(
        PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::from_legacy_sandbox_policy(&sandbox_policy),
            &file_system_sandbox_policy,
            NetworkSandboxPolicy::from(&sandbox_policy),
        ),
    );

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            cwd: Some(project_root.clone()),
            ..Default::default()
        })
        .expect("cwd-only update should succeed");

    let expected_file_system_policy = FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(
        &updated.sandbox_policy(),
        &project_root,
    );
    assert!(
        updated
            .file_system_sandbox_policy()
            .is_semantically_equivalent_to(&expected_file_system_policy, &project_root),
        "cwd-only update should rederive the legacy filesystem policy for the new cwd"
    );
}

#[tokio::test]
async fn notify_request_permissions_response_ignores_unmatched_call_id() {
    let (session, _turn_context) = make_session_and_context().await;
    *session.active_turn.lock().await = Some(ActiveTurn::default());

    session
        .notify_request_permissions_response(
            "missing",
            codex_protocol::request_permissions::RequestPermissionsResponse {
                permissions: RequestPermissionProfile {
                    network: Some(codex_protocol::models::NetworkPermissions {
                        enabled: Some(true),
                    }),
                    ..RequestPermissionProfile::default()
                },
                scope: PermissionGrantScope::Turn,
                strict_auto_review: false,
            },
        )
        .await;

    assert_eq!(session.granted_turn_permissions().await, None);
}

#[tokio::test]
async fn record_granted_request_permissions_for_turn_uses_originating_turn() {
    let (session, _turn_context) = make_session_and_context().await;
    let originating_active_turn = ActiveTurn::default();
    let originating_turn_state = Arc::clone(&originating_active_turn.turn_state);
    *session.active_turn.lock().await = Some(originating_active_turn);

    let current_active_turn = ActiveTurn::default();
    let current_turn_state = Arc::clone(&current_active_turn.turn_state);
    *session.active_turn.lock().await = Some(current_active_turn);

    let requested_permissions = RequestPermissionProfile {
        network: Some(codex_protocol::models::NetworkPermissions {
            enabled: Some(true),
        }),
        ..RequestPermissionProfile::default()
    };
    session
        .record_granted_request_permissions_for_turn(
            &codex_protocol::request_permissions::RequestPermissionsResponse {
                permissions: requested_permissions.clone(),
                scope: PermissionGrantScope::Turn,
                strict_auto_review: false,
            },
            Some(&originating_turn_state),
        )
        .await;

    assert_eq!(
        originating_turn_state.lock().await.granted_permissions(),
        Some(requested_permissions.into())
    );
    assert_eq!(current_turn_state.lock().await.granted_permissions(), None);
    assert_eq!(session.granted_turn_permissions().await, None);
}

#[test]
fn strict_auto_review_session_scope_grants_no_permissions() {
    let requested_permissions = RequestPermissionProfile {
        network: Some(codex_protocol::models::NetworkPermissions {
            enabled: Some(true),
        }),
        ..RequestPermissionProfile::default()
    };

    let response = Session::normalize_request_permissions_response(
        requested_permissions.clone(),
        codex_protocol::request_permissions::RequestPermissionsResponse {
            permissions: requested_permissions,
            scope: PermissionGrantScope::Session,
            strict_auto_review: true,
        },
        std::path::Path::new("/tmp"),
    );

    assert_eq!(
        response,
        codex_protocol::request_permissions::RequestPermissionsResponse {
            permissions: RequestPermissionProfile::default(),
            scope: PermissionGrantScope::Turn,
            strict_auto_review: false,
        }
    );
}

#[tokio::test]
async fn request_permissions_emits_event_when_granular_policy_allows_requests() {
    let (session, mut turn_context, rx) = make_session_and_context_with_rx().await;
    *session.active_turn.lock().await = Some(ActiveTurn::default());
    Arc::get_mut(&mut turn_context)
        .expect("single turn context ref")
        .approval_policy
        .set(AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: true,
            rules: true,
            skill_approval: true,
            request_permissions: true,
            mcp_elicitations: true,
        }))
        .expect("test setup should allow updating approval policy");

    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let call_id = "call-1".to_string();
    let expected_response = codex_protocol::request_permissions::RequestPermissionsResponse {
        permissions: RequestPermissionProfile {
            network: Some(codex_protocol::models::NetworkPermissions {
                enabled: Some(true),
            }),
            ..RequestPermissionProfile::default()
        },
        scope: PermissionGrantScope::Turn,
        strict_auto_review: false,
    };

    let handle = tokio::spawn({
        let session = Arc::clone(&session);
        let turn_context = Arc::clone(&turn_context);
        let call_id = call_id.clone();
        async move {
            session
                .request_permissions(
                    &turn_context,
                    call_id,
                    codex_protocol::request_permissions::RequestPermissionsArgs {
                        reason: Some("need network".to_string()),
                        permissions: RequestPermissionProfile {
                            network: Some(codex_protocol::models::NetworkPermissions {
                                enabled: Some(true),
                            }),
                            ..RequestPermissionProfile::default()
                        },
                    },
                    CancellationToken::new(),
                )
                .await
        }
    });

    let request_event = tokio::time::timeout(StdDuration::from_secs(1), rx.recv())
        .await
        .expect("request_permissions event timed out")
        .expect("request_permissions event missing");
    let EventMsg::RequestPermissions(request) = request_event.msg else {
        panic!("expected request_permissions event");
    };
    assert_eq!(request.call_id, call_id);
    assert_eq!(request.cwd, Some(turn_context.cwd.clone()));

    session
        .notify_request_permissions_response(&request.call_id, expected_response.clone())
        .await;

    let response = tokio::time::timeout(StdDuration::from_secs(1), handle)
        .await
        .expect("request_permissions future timed out")
        .expect("request_permissions join error");

    assert_eq!(response, Some(expected_response));
}

#[tokio::test]
async fn request_permissions_response_materializes_session_cwd_grants_before_recording() {
    let (session, mut turn_context, rx) = make_session_and_context_with_rx().await;
    *session.active_turn.lock().await = Some(ActiveTurn::default());
    Arc::get_mut(&mut turn_context)
        .expect("single turn context ref")
        .approval_policy
        .set(AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: true,
            rules: true,
            skill_approval: true,
            request_permissions: true,
            mcp_elicitations: true,
        }))
        .expect("test setup should allow updating approval policy");

    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let call_id = "call-1".to_string();
    let requested_permissions = RequestPermissionProfile {
        file_system: Some(FileSystemPermissions {
            entries: vec![FileSystemSandboxEntry {
                path: FileSystemPath::Special {
                    value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
                },
                access: FileSystemAccessMode::Write,
            }],
            glob_scan_max_depth: None,
        }),
        ..Default::default()
    };

    let handle = tokio::spawn({
        let session = Arc::clone(&session);
        let turn_context = Arc::clone(&turn_context);
        let call_id = call_id.clone();
        let requested_permissions = requested_permissions.clone();
        async move {
            session
                .request_permissions(
                    &turn_context,
                    call_id,
                    codex_protocol::request_permissions::RequestPermissionsArgs {
                        reason: Some("need cwd write".to_string()),
                        permissions: requested_permissions,
                    },
                    CancellationToken::new(),
                )
                .await
        }
    });

    let request_event = tokio::time::timeout(StdDuration::from_secs(1), rx.recv())
        .await
        .expect("request_permissions event timed out")
        .expect("request_permissions event missing");
    let EventMsg::RequestPermissions(request) = request_event.msg else {
        panic!("expected request_permissions event");
    };
    let request_cwd = request.cwd.clone().expect("request cwd");

    session
        .notify_request_permissions_response(
            &request.call_id,
            codex_protocol::request_permissions::RequestPermissionsResponse {
                permissions: request.permissions,
                scope: PermissionGrantScope::Session,
                strict_auto_review: false,
            },
        )
        .await;

    let expected_permissions = RequestPermissionProfile {
        file_system: Some(FileSystemPermissions::from_read_write_roots(
            /*read*/ None,
            Some(vec![request_cwd]),
        )),
        ..Default::default()
    };
    let expected_response = codex_protocol::request_permissions::RequestPermissionsResponse {
        permissions: expected_permissions.clone(),
        scope: PermissionGrantScope::Session,
        strict_auto_review: false,
    };

    let response = tokio::time::timeout(StdDuration::from_secs(1), handle)
        .await
        .expect("request_permissions future timed out")
        .expect("request_permissions join error");

    assert_eq!(response, Some(expected_response));
    assert_eq!(
        session.granted_session_permissions().await,
        Some(expected_permissions.into())
    );
}

#[tokio::test]
async fn request_permissions_is_auto_denied_when_granular_policy_blocks_tool_requests() {
    let (session, mut turn_context, rx) = make_session_and_context_with_rx().await;
    *session.active_turn.lock().await = Some(ActiveTurn::default());
    Arc::get_mut(&mut turn_context)
        .expect("single turn context ref")
        .approval_policy
        .set(AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: true,
            rules: true,
            skill_approval: true,
            request_permissions: false,
            mcp_elicitations: true,
        }))
        .expect("test setup should allow updating approval policy");

    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let call_id = "call-1".to_string();
    let response = session
        .request_permissions(
            &turn_context,
            call_id,
            codex_protocol::request_permissions::RequestPermissionsArgs {
                reason: Some("need network".to_string()),
                permissions: RequestPermissionProfile {
                    network: Some(codex_protocol::models::NetworkPermissions {
                        enabled: Some(true),
                    }),
                    ..RequestPermissionProfile::default()
                },
            },
            CancellationToken::new(),
        )
        .await;

    assert_eq!(
        response,
        Some(
            codex_protocol::request_permissions::RequestPermissionsResponse {
                permissions: RequestPermissionProfile::default(),
                scope: PermissionGrantScope::Turn,
                strict_auto_review: false,
            }
        )
    );
    assert!(
        tokio::time::timeout(StdDuration::from_millis(100), rx.recv())
            .await
            .is_err(),
        "request_permissions should not emit an event when granular.request_permissions is false"
    );
}

#[tokio::test]
async fn build_settings_update_items_emits_environment_item_for_network_changes() {
    let (session, previous_context) = make_session_and_context().await;
    let previous_context = Arc::new(previous_context);
    let mut current_context = previous_context
        .with_model(
            previous_context.model_info.slug.clone(),
            &session.services.models_manager,
        )
        .await;

    let mut config = (*current_context.config).clone();
    let mut requirements = config.config_layer_stack.requirements().clone();
    requirements.network = Some(Sourced::new(
        NetworkConstraints {
            domains: Some(NetworkDomainPermissionsToml {
                entries: std::collections::BTreeMap::from([
                    (
                        "api.example.com".to_string(),
                        NetworkDomainPermissionToml::Allow,
                    ),
                    (
                        "blocked.example.com".to_string(),
                        NetworkDomainPermissionToml::Deny,
                    ),
                ]),
            }),
            ..Default::default()
        },
        RequirementSource::CloudRequirements,
    ));
    let layers = config
        .config_layer_stack
        .get_layers(
            ConfigLayerStackOrdering::LowestPrecedenceFirst,
            /*include_disabled*/ true,
        )
        .into_iter()
        .cloned()
        .collect();
    config.config_layer_stack = ConfigLayerStack::new(
        layers,
        requirements,
        config.config_layer_stack.requirements_toml().clone(),
    )
    .expect("rebuild config layer stack with network requirements");
    current_context.config = Arc::new(config);

    let reference_context_item = previous_context.to_turn_context_item();
    let update_items = session
        .build_settings_update_items(Some(&reference_context_item), &current_context)
        .await;

    let environment_update = user_input_texts(&update_items)
        .into_iter()
        .find(|text| text.contains("<environment_context>"))
        .expect("environment update item should be emitted");
    assert!(environment_update.contains(
        "<network enabled=\"true\"><allowed>api.example.com</allowed><denied>blocked.example.com</denied></network>"
    ));
}

#[tokio::test]
async fn turn_context_item_omits_legacy_equivalent_file_system_sandbox_policy() {
    let (_session, turn_context) = make_session_and_context().await;

    let item = turn_context.to_turn_context_item();

    assert_eq!(item.file_system_sandbox_policy, None);
    assert_eq!(
        item.permission_profile,
        Some(turn_context.permission_profile())
    );
}

#[tokio::test]
async fn turn_context_item_stores_split_file_system_sandbox_policy_when_different() {
    let (_session, mut turn_context) = make_session_and_context().await;
    let file_system_sandbox_policy = file_system_policy_with_unreadable_glob(&turn_context);
    turn_context.permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
        turn_context.permission_profile.enforcement(),
        &file_system_sandbox_policy,
        turn_context.network_sandbox_policy(),
    );

    let item = turn_context.to_turn_context_item();

    assert_eq!(
        item.file_system_sandbox_policy,
        Some(file_system_sandbox_policy)
    );
    assert_eq!(
        item.permission_profile,
        Some(turn_context.permission_profile())
    );
}

#[tokio::test]
async fn rejects_escalated_permissions_when_policy_not_on_request() {
    use crate::exec::ExecParams;
    use crate::exec_policy::ExecApprovalRequest;
    use crate::sandboxing::SandboxPermissions;
    use crate::tools::sandboxing::ExecApprovalRequirement;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use codex_protocol::protocol::AskForApproval;
    use std::collections::HashMap;

    let (session, mut turn_context_raw) = make_session_and_context().await;
    // Ensure policy is NOT OnRequest so the early rejection path triggers
    turn_context_raw
        .approval_policy
        .set(AskForApproval::OnFailure)
        .expect("test setup should allow updating approval policy");
    let session = Arc::new(session);
    let mut turn_context = Arc::new(turn_context_raw);

    let timeout_ms = 1000;
    let sandbox_permissions = SandboxPermissions::RequireEscalated;
    let params = ExecParams {
        command: if cfg!(windows) {
            vec![
                "cmd.exe".to_string(),
                "/C".to_string(),
                "echo hi".to_string(),
            ]
        } else {
            vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "echo hi".to_string(),
            ]
        },
        cwd: turn_context.cwd.clone(),
        expiration: timeout_ms.into(),
        capture_policy: ExecCapturePolicy::ShellTool,
        env: HashMap::new(),
        network: None,
        sandbox_permissions,
        windows_sandbox_level: turn_context.windows_sandbox_level,
        windows_sandbox_private_desktop: turn_context
            .config
            .permissions
            .windows_sandbox_private_desktop,
        justification: Some("test".to_string()),
        arg0: None,
    };

    let turn_diff_tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));

    let tool_name = "shell";
    let call_id = "test-call".to_string();

    let handler = ShellCommandHandler::from(ShellCommandBackendConfig::Classic);
    let resp = handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker: Arc::clone(&turn_diff_tracker),
            call_id,
            tool_name: plain_tool_name(tool_name),
            source: crate::tools::context::ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "command": params.command.clone(),
                    "workdir": Some(turn_context.cwd.to_string_lossy().to_string()),
                    "timeout_ms": params.expiration.timeout_ms(),
                    "sandbox_permissions": params.sandbox_permissions,
                    "justification": params.justification.clone(),
                })
                .to_string(),
            },
        })
        .await;

    let Err(FunctionCallError::RespondToModel(output)) = resp else {
        panic!("expected error result");
    };

    let expected = format!(
        "approval policy is {policy:?}; reject command — you should not ask for escalated permissions if the approval policy is {policy:?}",
        policy = turn_context.approval_policy.value()
    );

    pretty_assertions::assert_eq!(output, expected);
    pretty_assertions::assert_eq!(session.granted_turn_permissions().await, None);

    // The rejection should not poison the non-escalated path for the same
    // command. Force DangerFullAccess so this check stays focused on approval
    // policy rather than platform-specific sandbox behavior.
    let turn_context_mut = Arc::get_mut(&mut turn_context).expect("unique turn context Arc");
    turn_context_mut.permission_profile = PermissionProfile::Disabled;

    let file_system_sandbox_policy = turn_context.file_system_sandbox_policy();
    let exec_approval_requirement = session
        .services
        .exec_policy
        .create_exec_approval_requirement_for_command(ExecApprovalRequest {
            command: &params.command,
            approval_policy: turn_context.approval_policy.value(),
            permission_profile: turn_context.permission_profile(),
            file_system_sandbox_policy: &file_system_sandbox_policy,
            sandbox_cwd: turn_context.cwd.as_path(),
            sandbox_permissions: SandboxPermissions::UseDefault,
            prefix_rule: None,
        })
        .await;
    assert!(matches!(
        exec_approval_requirement,
        ExecApprovalRequirement::Skip { .. }
    ));
}

#[tokio::test]
async fn unified_exec_rejects_escalated_permissions_when_policy_not_on_request() {
    use crate::sandboxing::SandboxPermissions;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use codex_protocol::protocol::AskForApproval;

    let (session, mut turn_context_raw) = make_session_and_context().await;
    turn_context_raw
        .approval_policy
        .set(AskForApproval::OnFailure)
        .expect("test setup should allow updating approval policy");
    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context_raw);
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));

    let handler = ExecCommandHandler::default();
    let resp = handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker: Arc::clone(&tracker),
            call_id: "exec-call".to_string(),
            tool_name: plain_tool_name("exec_command"),
            source: crate::tools::context::ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "cmd": "echo hi",
                    "sandbox_permissions": SandboxPermissions::RequireEscalated,
                    "justification": "need unsandboxed execution",
                })
                .to_string(),
            },
        })
        .await;

    let Err(FunctionCallError::RespondToModel(output)) = resp else {
        panic!("expected error result");
    };

    let expected = format!(
        "approval policy is {policy:?}; reject command — you cannot ask for escalated permissions if the approval policy is {policy:?}",
        policy = turn_context.approval_policy.value()
    );

    pretty_assertions::assert_eq!(output, expected);
}
