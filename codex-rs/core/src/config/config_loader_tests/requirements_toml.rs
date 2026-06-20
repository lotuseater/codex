use super::*;
use codex_config::CloudConfigBundleLoader;
use codex_config::ConfigLoadOptions;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn top_level_allow_managed_hooks_only_in_user_config_does_not_enable_requirements_policy()
-> std::io::Result<()> {
    let tmp = tempdir().expect("tempdir");
    std::fs::write(
        tmp.path().join(CONFIG_TOML_FILE),
        "allow_managed_hooks_only = true",
    )
    .expect("write config");

    let cwd = AbsolutePathBuf::try_from(tmp.path()).expect("cwd");
    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(cwd),
        &[] as &[(String, TomlValue)],
        LoaderOverrides::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    assert_eq!(layers.requirements_toml().allow_managed_hooks_only, None);
    assert!(layers.requirements().allow_managed_hooks_only.is_none());

    Ok(())
}

#[tokio::test]
async fn hooks_allow_managed_hooks_only_in_user_config_does_not_enable_requirements_policy()
-> std::io::Result<()> {
    let tmp = tempdir().expect("tempdir");
    let contents = r#"
[hooks]
allow_managed_hooks_only = true

[[hooks.PreToolUse]]
matcher = "^Bash$"

[[hooks.PreToolUse.hooks]]
type = "command"
command = "python3 /tmp/user-hook.py"
"#;
    std::fs::write(tmp.path().join(CONFIG_TOML_FILE), contents).expect("write config");

    let cwd = AbsolutePathBuf::try_from(tmp.path()).expect("cwd");
    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(cwd),
        &[] as &[(String, TomlValue)],
        LoaderOverrides::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    assert!(
        layers
            .get_user_layer()
            .and_then(|layer| layer.config.get("hooks"))
            .is_some(),
        "hooks should still deserialize from config.toml"
    );
    assert_eq!(layers.requirements_toml().allow_managed_hooks_only, None);
    assert!(layers.requirements().allow_managed_hooks_only.is_none());

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn load_requirements_toml_produces_expected_constraints() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let requirements_file = tmp.path().join("requirements.toml");
    tokio::fs::write(
        &requirements_file,
        r#"
allowed_approval_policies = ["never", "on-request"]
allowed_web_search_modes = ["cached"]
enforce_residency = "us"

[features]
personality = true
"#,
    )
    .await?;

    let requirements_file = AbsolutePathBuf::try_from(requirements_file)?;
    let config_requirements_toml =
        compose_requirements(load_requirements_toml(LOCAL_FS.as_ref(), &requirements_file).await?)?
            .unwrap_or_default();

    assert_eq!(
        config_requirements_toml
            .allowed_approval_policies
            .as_deref()
            .cloned(),
        Some(vec![AskForApproval::Never, AskForApproval::OnRequest])
    );
    assert_eq!(
        config_requirements_toml
            .allowed_web_search_modes
            .as_deref()
            .cloned(),
        Some(vec![codex_config::WebSearchModeRequirement::Cached])
    );
    assert_eq!(
        config_requirements_toml
            .feature_requirements
            .as_ref()
            .map(|requirements| requirements.value.clone()),
        Some(codex_config::FeatureRequirementsToml {
            entries: BTreeMap::from([("personality".to_string(), true)]),
        })
    );
    let config_requirements: ConfigRequirements = config_requirements_toml.try_into()?;
    assert_eq!(
        config_requirements.approval_policy.value(),
        AskForApproval::Never
    );
    config_requirements
        .approval_policy
        .can_set(&AskForApproval::Never)?;
    assert!(
        config_requirements
            .approval_policy
            .can_set(&AskForApproval::OnFailure)
            .is_err()
    );
    assert_eq!(
        config_requirements.web_search_mode.value(),
        WebSearchMode::Cached
    );
    config_requirements
        .web_search_mode
        .can_set(&WebSearchMode::Cached)?;
    config_requirements
        .web_search_mode
        .can_set(&WebSearchMode::Cached)?;
    config_requirements
        .web_search_mode
        .can_set(&WebSearchMode::Disabled)?;
    assert!(
        config_requirements
            .web_search_mode
            .can_set(&WebSearchMode::Live)
            .is_err()
    );
    assert_eq!(
        config_requirements.enforce_residency.value(),
        Some(codex_config::ResidencyRequirement::Us)
    );
    assert_eq!(
        config_requirements
            .feature_requirements
            .as_ref()
            .map(|requirements| requirements.value.clone()),
        Some(codex_config::FeatureRequirementsToml {
            entries: BTreeMap::from([("personality".to_string(), true)]),
        })
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn cloud_requirements_are_not_overwritten_by_system_requirements() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let requirements_file = tmp.path().join("requirements.toml");
    tokio::fs::write(
        &requirements_file,
        r#"
allowed_approval_policies = ["on-request"]
"#,
    )
    .await?;

    let cloud_entry = RequirementsLayerEntry::from_toml(
        RequirementSource::EnterpriseManaged {
            id: "cloud_requirements".to_string(),
            name: "Cloud requirements".to_string(),
        },
        r#"
allowed_approval_policies = ["never"]
"#,
    );
    let system_entry = load_requirements_toml(
        LOCAL_FS.as_ref(),
        &AbsolutePathBuf::try_from(requirements_file)?,
    )
    .await?;
    // Cloud requirements win over system requirements, so compose them last.
    let config_requirements_toml =
        compose_requirements(system_entry.into_iter().chain(std::iter::once(cloud_entry)))?
            .unwrap_or_default();

    assert_eq!(
        config_requirements_toml
            .allowed_approval_policies
            .as_ref()
            .map(|sourced| sourced.value.clone()),
        Some(vec![AskForApproval::Never])
    );
    assert_eq!(
        config_requirements_toml
            .allowed_approval_policies
            .as_ref()
            .map(|sourced| sourced.source.clone()),
        Some(RequirementSource::EnterpriseManaged {
            id: "cloud_requirements".to_string(),
            name: "Cloud requirements".to_string(),
        })
    );

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn system_remote_sandbox_config_keeps_cloud_sandbox_modes() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let requirements_file = tmp.path().join("requirements.toml");
    tokio::fs::write(
        &requirements_file,
        r#"
[[remote_sandbox_config]]
hostname_patterns = ["*"]
allowed_sandbox_modes = ["read-only", "workspace-write"]
"#,
    )
    .await?;

    let cloud_source = RequirementSource::EnterpriseManaged {
        id: "cloud_requirements".to_string(),
        name: "Cloud requirements".to_string(),
    };
    let cloud_entry = RequirementsLayerEntry::from_toml(
        cloud_source.clone(),
        r#"
allowed_sandbox_modes = ["read-only"]
"#,
    );
    let system_entry = load_requirements_toml(
        LOCAL_FS.as_ref(),
        &AbsolutePathBuf::try_from(requirements_file)?,
    )
    .await?;
    // Cloud requirements win over system requirements, so compose them last.
    let config_requirements_toml =
        compose_requirements(system_entry.into_iter().chain(std::iter::once(cloud_entry)))?
            .unwrap_or_default();
    let config_requirements: ConfigRequirements = config_requirements_toml.try_into()?;

    assert_eq!(
        config_requirements
            .permission_profile
            .can_set(&PermissionProfile::workspace_write()),
        Err(ConstraintError::InvalidValue {
            field_name: "sandbox_mode",
            candidate: "WorkspaceWrite".into(),
            allowed: "[ReadOnly]".into(),
            requirement_source: cloud_source,
        })
    );

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn load_requirements_toml_resolves_deny_read_against_parent() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let requirements_dir = tmp.path().join("managed");
    tokio::fs::create_dir_all(&requirements_dir).await?;
    let requirements_file = requirements_dir.join("requirements.toml");
    tokio::fs::write(
        &requirements_file,
        r#"
[permissions.filesystem]
deny_read = ["./sensitive", "../shared/secret.txt"]
"#,
    )
    .await?;

    let requirements_file = AbsolutePathBuf::try_from(requirements_file)?;
    let config_requirements_toml =
        compose_requirements(load_requirements_toml(LOCAL_FS.as_ref(), &requirements_file).await?)?
            .unwrap_or_default();

    let permissions = config_requirements_toml
        .permissions
        .expect("permissions requirements should load");
    let filesystem = permissions
        .value
        .filesystem
        .expect("filesystem requirements should load");
    let deny_read = filesystem.deny_read.expect("deny_read paths should load");

    assert_eq!(
        deny_read,
        vec![
            FilesystemDenyReadPattern::from(AbsolutePathBuf::try_from(
                requirements_dir.join("sensitive")
            )?,),
            FilesystemDenyReadPattern::from(AbsolutePathBuf::try_from(
                tmp.path().join("shared").join("secret.txt"),
            )?),
        ]
    );
    assert_eq!(
        permissions.source,
        RequirementSource::SystemRequirementsToml {
            file: requirements_file,
        }
    );

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn load_requirements_toml_resolves_deny_read_glob_against_parent() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let requirements_dir = tmp.path().join("managed");
    tokio::fs::create_dir_all(&requirements_dir).await?;
    let requirements_file = requirements_dir.join("requirements.toml");
    tokio::fs::write(
        &requirements_file,
        r#"
[permissions.filesystem]
deny_read = ["./sensitive/**/*.txt"]
"#,
    )
    .await?;

    let requirements_file = AbsolutePathBuf::try_from(requirements_file)?;
    let config_requirements_toml =
        compose_requirements(load_requirements_toml(LOCAL_FS.as_ref(), &requirements_file).await?)?
            .unwrap_or_default();

    let permissions = config_requirements_toml
        .permissions
        .expect("permissions requirements should load");
    let filesystem = permissions
        .value
        .filesystem
        .expect("filesystem requirements should load");
    let deny_read = filesystem
        .deny_read
        .expect("deny_read patterns should load");

    assert_eq!(
        deny_read,
        vec![
            FilesystemDenyReadPattern::from_input(&format!(
                "{}/sensitive/**/*.txt",
                requirements_dir.display()
            ))
            .expect("normalize glob pattern")
        ]
    );
    assert_eq!(
        permissions.source,
        RequirementSource::SystemRequirementsToml {
            file: requirements_file,
        }
    );

    Ok(())
}

#[tokio::test]
async fn load_config_layers_can_ignore_managed_requirements() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    let cwd = AbsolutePathBuf::from_absolute_path(tmp.path())?;

    let managed_config_path = tmp.path().join("managed_config.toml");
    tokio::fs::write(
        &managed_config_path,
        r#"approval_policy = "never"
"#,
    )
    .await?;
    let system_requirements_path = tmp.path().join("requirements.toml");
    tokio::fs::write(
        &system_requirements_path,
        r#"allowed_sandbox_modes = ["read-only"]
"#,
    )
    .await?;

    let mut overrides = LoaderOverrides::with_managed_config_path_for_tests(managed_config_path);
    overrides.system_requirements_path = Some(system_requirements_path);
    overrides.ignore_managed_requirements = true;

    let cloud_requirements = CloudRequirementsLoader::new(async {
        Ok(Some(ConfigRequirementsToml {
            allowed_approval_policies: Some(vec![AskForApproval::Never]),
            ..Default::default()
        }))
    });

    let mut config = ConfigBuilder::default()
        .codex_home(codex_home)
        .fallback_cwd(Some(cwd.to_path_buf()))
        .loader_overrides(overrides)
        .cloud_requirements(cloud_requirements)
        .build()
        .await?;

    assert!(
        config
            .permissions
            .approval_policy
            .can_set(&AskForApproval::OnRequest)
            .is_ok(),
        "ignoring managed requirements should leave on-request approval allowed"
    );
    config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest)
        .expect("ignoring managed requirements should allow setting on-request approval");

    Ok(())
}

#[tokio::test]
async fn load_config_layers_includes_cloud_hook_requirements() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    let managed_dir = tmp.path().join("managed-hooks");
    tokio::fs::create_dir_all(&managed_dir).await?;
    let cwd = AbsolutePathBuf::from_absolute_path(tmp.path())?;

    let requirements = ConfigRequirementsToml {
        hooks: Some(codex_config::ManagedHooksRequirementsToml {
            managed_dir: Some(managed_dir.clone()),
            windows_managed_dir: None,
            hooks: codex_config::HookEventsToml {
                pre_tool_use: vec![codex_config::MatcherGroup {
                    matcher: Some("^Bash$".to_string()),
                    hooks: vec![codex_config::HookHandlerConfig::Command {
                        command: format!("python3 {}/pre.py", managed_dir.display()),
                        command_windows: None,
                        timeout_sec: Some(10),
                        r#async: false,
                        status_message: Some("checking".to_string()),
                    }],
                }],
                ..Default::default()
            },
        }),
        ..ConfigRequirementsToml::default()
    };
    let expected = requirements.clone();
    let cloud_requirements = CloudRequirementsLoader::new(async move { Ok(Some(requirements)) });

    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        &codex_home,
        Some(cwd),
        &[] as &[(String, TomlValue)],
        ConfigLoadOptions {
            loader_overrides: LoaderOverrides::default(),
            strict_config: false,
            cloud_config_bundle: CloudConfigBundleLoader::from_requirements_loader(
                cloud_requirements,
            ),
        },
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    assert_eq!(layers.requirements_toml().hooks, expected.hooks);
    assert_eq!(
        layers
            .requirements()
            .managed_hooks
            .as_ref()
            .map(|hooks| hooks.source.clone()),
        Some(Some(RequirementSource::EnterpriseManaged {
            id: "cloud_requirements".to_string(),
            name: "Cloud requirements".to_string(),
        }))
    );

    Ok(())
}

#[tokio::test]
async fn load_config_layers_applies_matching_remote_sandbox_config() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    let cwd = AbsolutePathBuf::from_absolute_path(tmp.path())?;

    let requirements: ConfigRequirementsToml = toml::from_str(
        r#"
            allowed_sandbox_modes = ["read-only"]

            [[remote_sandbox_config]]
            hostname_patterns = ["*"]
            allowed_sandbox_modes = ["read-only", "workspace-write"]
        "#,
    )?;
    let cloud_requirements = CloudRequirementsLoader::new(async move { Ok(Some(requirements)) });
    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        &codex_home,
        Some(cwd),
        &[] as &[(String, TomlValue)],
        ConfigLoadOptions {
            loader_overrides: LoaderOverrides::default(),
            strict_config: false,
            cloud_config_bundle: CloudConfigBundleLoader::from_requirements_loader(
                cloud_requirements,
            ),
        },
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    assert_eq!(
        layers.requirements_toml().allowed_sandbox_modes,
        Some(vec![
            codex_config::SandboxModeRequirement::ReadOnly,
            codex_config::SandboxModeRequirement::WorkspaceWrite,
        ])
    );
    assert!(
        layers
            .requirements()
            .permission_profile
            .can_set(&PermissionProfile::workspace_write())
            .is_ok()
    );

    Ok(())
}

#[tokio::test]
async fn load_config_layers_fails_when_cloud_requirements_loader_fails() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    let cwd = AbsolutePathBuf::from_absolute_path(tmp.path())?;

    let err = load_config_layers_state(
        LOCAL_FS.as_ref(),
        &codex_home,
        Some(cwd),
        &[] as &[(String, TomlValue)],
        ConfigLoadOptions {
            loader_overrides: LoaderOverrides::default(),
            strict_config: false,
            cloud_config_bundle: CloudConfigBundleLoader::from_requirements_loader(
                CloudRequirementsLoader::new(async {
                    Err(CloudRequirementsLoadError::new(
                        codex_config::CloudRequirementsLoadErrorCode::RequestFailed,
                        /*status_code*/ None,
                        "cloud requirements failed",
                    ))
                }),
            ),
        },
        &codex_config::NoopThreadConfigLoader,
    )
    .await
    .expect_err("cloud requirements failure should fail closed");

    assert_eq!(err.kind(), std::io::ErrorKind::Other);
    assert!(err.to_string().contains("cloud requirements failed"));

    Ok(())
}
