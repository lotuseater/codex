use super::*;
use codex_config::CloudConfigBundleLoader;
use codex_config::ConfigLoadOptions;

#[cfg(target_os = "macos")]
#[tokio::test]
async fn managed_preferences_take_highest_precedence() {
    use base64::Engine;

    let tmp = tempdir().expect("tempdir");
    let managed_path = tmp.path().join("managed_config.toml");

    std::fs::write(
        tmp.path().join(CONFIG_TOML_FILE),
        r#"[nested]
value = "base"
"#,
    )
    .expect("write base");
    std::fs::write(
        &managed_path,
        r#"[nested]
value = "managed_config"
flag = true
"#,
    )
    .expect("write managed config");
    let raw_managed_preferences = r#"
# managed profile
[nested]
value = "managed"
flag = false
"#;

    let mut overrides = LoaderOverrides::with_managed_config_path_for_tests(managed_path);
    overrides.managed_preferences_base64 =
        Some(base64::prelude::BASE64_STANDARD.encode(raw_managed_preferences.as_bytes()));

    let cwd = AbsolutePathBuf::try_from(tmp.path()).expect("cwd");
    let state = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(cwd),
        &[] as &[(String, TomlValue)],
        overrides,
        &codex_config::NoopThreadConfigLoader,
    )
    .await
    .expect("load config");
    let loaded = state.effective_config();
    let nested = loaded
        .get("nested")
        .and_then(|v| v.as_table())
        .expect("nested table");
    assert_eq!(
        nested.get("value"),
        Some(&TomlValue::String("managed".to_string()))
    );
    assert_eq!(nested.get("flag"), Some(&TomlValue::Boolean(false)));
    let mdm_layer = state
        .layers_high_to_low()
        .into_iter()
        .find(|layer| {
            matches!(
                layer.name,
                ConfigLayerSource::LegacyManagedConfigTomlFromMdm
            )
        })
        .expect("mdm layer");
    let raw = mdm_layer.raw_toml().expect("preserved mdm toml");
    assert!(raw.contains("# managed profile"));
    assert!(raw.contains("value = \"managed\""));
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn managed_preferences_expand_home_directory_in_workspace_write_roots() -> anyhow::Result<()>
{
    use base64::Engine;
    use codex_protocol::protocol::SandboxPolicy;

    let Some(home) = dirs::home_dir() else {
        return Ok(());
    };
    let tmp = tempdir()?;

    let mut loader_overrides =
        LoaderOverrides::with_managed_config_path_for_tests(tmp.path().join("managed_config.toml"));
    loader_overrides.managed_preferences_base64 = Some(
        base64::prelude::BASE64_STANDARD.encode(
            r#"
sandbox_mode = "workspace-write"
[sandbox_workspace_write]
writable_roots = ["~/code"]
"#
            .as_bytes(),
        ),
    );

    let config = ConfigBuilder::default()
        .codex_home(tmp.path().to_path_buf())
        .fallback_cwd(Some(tmp.path().to_path_buf()))
        .loader_overrides(loader_overrides)
        .build()
        .await?;

    let expected_root = AbsolutePathBuf::from_absolute_path(home.join("code"))?;
    match &config.legacy_sandbox_policy() {
        SandboxPolicy::WorkspaceWrite { writable_roots, .. } => {
            assert_eq!(
                writable_roots
                    .iter()
                    .filter(|root| **root == expected_root)
                    .count(),
                1,
            );
        }
        other => panic!("expected workspace-write policy, got {other:?}"),
    }

    Ok(())
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn managed_preferences_requirements_are_applied() -> anyhow::Result<()> {
    use base64::Engine;

    let tmp = tempdir()?;

    let mut loader_overrides =
        LoaderOverrides::with_managed_config_path_for_tests(tmp.path().join("managed_config.toml"));
    loader_overrides.macos_managed_config_requirements_base64 = Some(
        base64::prelude::BASE64_STANDARD.encode(
            r#"
allowed_approval_policies = ["never"]
allowed_sandbox_modes = ["read-only"]
"#
            .as_bytes(),
        ),
    );

    let state = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(AbsolutePathBuf::try_from(tmp.path())?),
        &[] as &[(String, TomlValue)],
        loader_overrides,
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    assert_eq!(
        state.requirements().approval_policy.value(),
        AskForApproval::Never
    );
    assert_eq!(
        state.requirements().permission_profile.get(),
        &PermissionProfile::read_only()
    );
    assert!(
        state
            .requirements()
            .approval_policy
            .can_set(&AskForApproval::OnRequest)
            .is_err()
    );
    assert!(
        state
            .requirements()
            .permission_profile
            .can_set(&PermissionProfile::workspace_write())
            .is_err()
    );

    Ok(())
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn managed_preferences_requirements_take_precedence() -> anyhow::Result<()> {
    use base64::Engine;

    let tmp = tempdir()?;
    let managed_path = tmp.path().join("managed_config.toml");

    tokio::fs::write(
        &managed_path,
        r#"approval_policy = "on-request"
"#,
    )
    .await?;

    let mut loader_overrides = LoaderOverrides::with_managed_config_path_for_tests(managed_path);
    loader_overrides.macos_managed_config_requirements_base64 = Some(
        base64::prelude::BASE64_STANDARD.encode(
            r#"
allowed_approval_policies = ["never"]
"#
            .as_bytes(),
        ),
    );

    let state = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(AbsolutePathBuf::try_from(tmp.path())?),
        &[] as &[(String, TomlValue)],
        loader_overrides,
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    assert_eq!(
        state.requirements().approval_policy.value(),
        AskForApproval::Never
    );
    assert!(
        state
            .requirements()
            .approval_policy
            .can_set(&AskForApproval::OnRequest)
            .is_err()
    );

    Ok(())
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn cloud_requirements_take_precedence_over_mdm_requirements() -> anyhow::Result<()> {
    use base64::Engine;

    let tmp = tempdir()?;
    let mut loader_overrides = LoaderOverrides::without_managed_config_for_tests();
    loader_overrides.macos_managed_config_requirements_base64 = Some(
        base64::prelude::BASE64_STANDARD.encode(
            r#"
allowed_approval_policies = ["on-request"]
"#
            .as_bytes(),
        ),
    );
    let state = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(AbsolutePathBuf::try_from(tmp.path())?),
        &[] as &[(String, TomlValue)],
        ConfigLoadOptions {
            loader_overrides,
            strict_config: false,
            cloud_config_bundle: CloudConfigBundleLoader::from_requirements_loader(
                CloudRequirementsLoader::new(async {
                    Ok(Some(ConfigRequirementsToml {
                        allowed_approval_policies: Some(vec![AskForApproval::Never]),
                        allowed_approvals_reviewers: None,
                        allowed_sandbox_modes: None,
                        allowed_permissions: None,
                        remote_sandbox_config: None,
                        allowed_web_search_modes: None,
                        allow_managed_hooks_only: None,
                        allow_appshots: None,
                        computer_use: None,
                        feature_requirements: None,
                        hooks: None,
                        mcp_servers: None,
                        plugins: None,
                        apps: None,
                        rules: None,
                        enforce_residency: None,
                        network: None,
                        permissions: None,
                        guardian_policy_config: None,
                    }))
                }),
            ),
        },
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    assert_eq!(
        state.requirements().approval_policy.value(),
        AskForApproval::Never
    );
    assert_eq!(
        state
            .requirements()
            .approval_policy
            .can_set(&AskForApproval::OnRequest),
        Err(ConstraintError::InvalidValue {
            field_name: "approval_policy",
            candidate: "OnRequest".into(),
            allowed: "[Never]".into(),
            requirement_source: RequirementSource::EnterpriseManaged {
                id: "cloud_requirements".to_string(),
                name: "Cloud requirements".to_string(),
            },
        })
    );

    Ok(())
}
