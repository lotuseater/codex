use super::*;

#[tokio::test]
async fn ignore_user_config_keeps_empty_user_layer() -> std::io::Result<()> {
    let tmp = tempdir().expect("tempdir");
    std::fs::write(
        tmp.path().join(CONFIG_TOML_FILE),
        r#"model = "from-user-config"
invalid = ["#,
    )
    .expect("write config");

    let cwd = AbsolutePathBuf::try_from(tmp.path()).expect("cwd");
    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(cwd),
        &[] as &[(String, TomlValue)],
        LoaderOverrides {
            ignore_user_config: true,
            ..Default::default()
        },
        CloudRequirementsLoader::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    let user_layer = layers
        .get_active_user_layer()
        .expect("expected a user layer even when CODEX_HOME/config.toml is ignored");
    assert_eq!(
        user_layer.config,
        TomlValue::Table(toml::map::Map::new()),
        "expected ignored user config to preserve only layer metadata"
    );
    assert_eq!(layers.effective_config().get("model"), None);
    Ok(())
}

#[tokio::test]
async fn ignore_rules_marks_config_stack_for_exec_policy_rule_skip() -> std::io::Result<()> {
    let tmp = tempdir().expect("tempdir");
    let cwd = AbsolutePathBuf::try_from(tmp.path()).expect("cwd");

    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(cwd),
        &[] as &[(String, TomlValue)],
        LoaderOverrides {
            ignore_user_and_project_exec_policy_rules: true,
            ..Default::default()
        },
        CloudRequirementsLoader::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    assert!(layers.ignore_user_and_project_exec_policy_rules());
    Ok(())
}

#[tokio::test]
async fn merges_managed_config_layer_on_top() {
    let tmp = tempdir().expect("tempdir");
    let managed_path = tmp.path().join("managed_config.toml");

    std::fs::write(
        tmp.path().join(CONFIG_TOML_FILE),
        r#"foo = 1

[nested]
value = "base"
"#,
    )
    .expect("write base");
    std::fs::write(
        &managed_path,
        r#"foo = 2

[nested]
value = "managed_config"
extra = true
"#,
    )
    .expect("write managed config");

    let overrides = LoaderOverrides::with_managed_config_path_for_tests(managed_path);

    let cwd = AbsolutePathBuf::try_from(tmp.path()).expect("cwd");
    let state = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(cwd),
        &[] as &[(String, TomlValue)],
        overrides,
        CloudRequirementsLoader::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await
    .expect("load config");
    let loaded = state.effective_config();
    let table = loaded.as_table().expect("top-level table expected");

    assert_eq!(table.get("foo"), Some(&TomlValue::Integer(2)));
    let nested = table
        .get("nested")
        .and_then(|v| v.as_table())
        .expect("nested");
    assert_eq!(
        nested.get("value"),
        Some(&TomlValue::String("managed_config".to_string()))
    );
    assert_eq!(nested.get("extra"), Some(&TomlValue::Boolean(true)));
}

#[tokio::test]
async fn returns_empty_when_all_layers_missing() {
    let tmp = tempdir().expect("tempdir");
    let managed_path = tmp.path().join("managed_config.toml");

    let overrides = LoaderOverrides::with_managed_config_path_for_tests(managed_path);

    let cwd = AbsolutePathBuf::try_from(tmp.path()).expect("cwd");
    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(cwd),
        &[] as &[(String, TomlValue)],
        overrides,
        CloudRequirementsLoader::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await
    .expect("load layers");
    let user_layer = layers
        .get_active_user_layer()
        .expect("expected a user layer even when CODEX_HOME/config.toml does not exist");
    let expected_user_layer = ConfigLayerEntry::new(
        ConfigLayerSource::User {
            file: AbsolutePathBuf::resolve_path_against_base(CONFIG_TOML_FILE, tmp.path()),
            profile: None,
        },
        TomlValue::Table(toml::map::Map::new()),
    );
    assert_eq!(&expected_user_layer, user_layer);
    assert_eq!(
        user_layer.config,
        TomlValue::Table(toml::map::Map::new()),
        "expected empty config for user layer when config.toml does not exist"
    );

    let binding = layers.effective_config();
    let base_table = binding.as_table().expect("base table expected");
    assert!(
        base_table.is_empty(),
        "expected empty base layer when configs missing"
    );
    let num_system_layers = layers
        .layers_high_to_low()
        .iter()
        .filter(|layer| matches!(layer.name, ConfigLayerSource::System { .. }))
        .count();
    assert_eq!(
        num_system_layers, 1,
        "system layer should always be present"
    );

    #[cfg(not(target_os = "macos"))]
    {
        let effective = layers.effective_config();
        let table = effective.as_table().expect("top-level table expected");
        assert!(
            table.is_empty(),
            "expected empty table when configs missing"
        );
    }
}

#[tokio::test]
async fn selected_user_config_file_layers_over_base_user_config() {
    let tmp = tempdir().expect("tempdir");
    let managed_path = tmp.path().join("managed_config.toml");
    let selected_config = tmp.path().join("work.config.toml");

    std::fs::write(
        tmp.path().join(CONFIG_TOML_FILE),
        r#"
model = "gpt-main"
approval_policy = "on-failure"
"#,
    )
    .expect("write default user config");
    std::fs::write(&selected_config, r#"model = "gpt-work""#).expect("write selected user config");

    let mut overrides = LoaderOverrides::with_managed_config_path_for_tests(managed_path);
    overrides.user_config_path =
        Some(AbsolutePathBuf::from_absolute_path(&selected_config).expect("selected config path"));
    overrides.user_config_profile = Some("work".parse().expect("profile-v2 name"));

    let cwd = AbsolutePathBuf::try_from(tmp.path()).expect("cwd");
    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(cwd),
        &[] as &[(String, TomlValue)],
        overrides,
        CloudRequirementsLoader::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await
    .expect("load layers");

    let user_layers = layers.get_user_layers(
        super::ConfigLayerStackOrdering::LowestPrecedenceFirst,
        /*include_disabled*/ false,
    );
    assert_eq!(user_layers.len(), 2);
    assert_eq!(
        user_layers[0].name,
        ConfigLayerSource::User {
            file: AbsolutePathBuf::from_absolute_path(tmp.path().join(CONFIG_TOML_FILE))
                .expect("base user config path"),
            profile: None,
        }
    );
    let user_layer = layers.get_active_user_layer().expect("selected user layer");
    assert_eq!(
        user_layer.name,
        ConfigLayerSource::User {
            file: AbsolutePathBuf::from_absolute_path(&selected_config)
                .expect("selected user config path"),
            profile: Some("work".to_string()),
        }
    );
    assert_eq!(
        layers
            .effective_config()
            .get("model")
            .and_then(TomlValue::as_str),
        Some("gpt-work")
    );
    assert_eq!(
        layers
            .effective_config()
            .get("approval_policy")
            .and_then(TomlValue::as_str),
        Some("on-failure")
    );
}

#[tokio::test]
async fn includes_thread_config_layers_in_stack() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let cwd_dir = tmp.path().join("project");
    tokio::fs::create_dir_all(&cwd_dir).await?;
    let cwd = AbsolutePathBuf::from_absolute_path(&cwd_dir)?;
    let overrides = LoaderOverrides::without_managed_config_for_tests();
    let expected_system_config = AbsolutePathBuf::from_absolute_path(
        overrides
            .system_config_path
            .as_ref()
            .expect("test overrides should include a system config path"),
    )?;
    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(cwd),
        &[("features.plugins".to_string(), TomlValue::Boolean(true))],
        overrides,
        CloudRequirementsLoader::default(),
        &StaticThreadConfigLoader::new(vec![ThreadConfigSource::Session(SessionThreadConfig {
            features: BTreeMap::from([("plugins".to_string(), false)]),
            ..Default::default()
        })]),
    )
    .await?;

    let layer_sources = layers
        .layers_high_to_low()
        .into_iter()
        .map(|layer| layer.name.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        layer_sources,
        vec![
            ConfigLayerSource::SessionFlags,
            ConfigLayerSource::SessionFlags,
            ConfigLayerSource::User {
                file: AbsolutePathBuf::resolve_path_against_base(CONFIG_TOML_FILE, tmp.path()),
                profile: None,
            },
            ConfigLayerSource::System {
                file: expected_system_config,
            },
        ]
    );
    assert_eq!(
        layers
            .effective_config()
            .get("features")
            .and_then(TomlValue::as_table)
            .and_then(|features| features.get("plugins")),
        Some(&TomlValue::Boolean(false))
    );

    Ok(())
}

#[tokio::test]
async fn load_config_layers_includes_cloud_requirements() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    let cwd = AbsolutePathBuf::from_absolute_path(tmp.path())?;

    let requirements = ConfigRequirementsToml {
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
    };
    let expected = requirements.clone();
    let cloud_requirements = CloudRequirementsLoader::new(async move { Ok(Some(requirements)) });

    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        &codex_home,
        Some(cwd),
        &[] as &[(String, TomlValue)],
        LoaderOverrides::default(),
        cloud_requirements,
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    assert_eq!(
        layers.requirements_toml().allowed_approval_policies,
        expected.allowed_approval_policies
    );
    assert_eq!(
        layers
            .requirements()
            .approval_policy
            .can_set(&AskForApproval::OnRequest),
        Err(ConstraintError::InvalidValue {
            field_name: "approval_policy",
            candidate: "OnRequest".into(),
            allowed: "[Never]".into(),
            requirement_source: RequirementSource::CloudRequirements,
        })
    );

    Ok(())
}
