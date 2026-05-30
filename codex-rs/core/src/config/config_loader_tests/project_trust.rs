use super::*;
use super::common::make_config_for_test;

#[tokio::test]
async fn project_layers_disabled_when_untrusted_or_unknown() -> std::io::Result<()> {
    let tmp = tempdir()?;
    let project_root = tmp.path().join("project");
    let nested = project_root.join("child");
    tokio::fs::create_dir_all(nested.join(".codex")).await?;
    tokio::fs::write(
        nested.join(".codex").join(CONFIG_TOML_FILE),
        r#"foo = "child"
profile = "ignored"
"#,
    )
    .await?;

    let cwd = AbsolutePathBuf::from_absolute_path(&nested)?;

    let codex_home_untrusted = tmp.path().join("home_untrusted");
    tokio::fs::create_dir_all(&codex_home_untrusted).await?;
    make_config_for_test(
        &codex_home_untrusted,
        &project_root,
        TrustLevel::Untrusted,
        /*project_root_markers*/ None,
    )
    .await?;
    let untrusted_config_path = codex_home_untrusted.join(CONFIG_TOML_FILE);
    let untrusted_config_contents = tokio::fs::read_to_string(&untrusted_config_path).await?;
    tokio::fs::write(
        &untrusted_config_path,
        format!(
            r#"foo = "user"
{untrusted_config_contents}"#
        ),
    )
    .await?;

    let layers_untrusted = load_config_layers_state(
        LOCAL_FS.as_ref(),
        &codex_home_untrusted,
        Some(cwd.clone()),
        &[] as &[(String, TomlValue)],
        LoaderOverrides::default(),
        CloudRequirementsLoader::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;
    let project_layers_untrusted: Vec<_> = layers_untrusted
        .get_layers(
            ConfigLayerStackOrdering::HighestPrecedenceFirst,
            /*include_disabled*/ true,
        )
        .into_iter()
        .filter(|layer| matches!(layer.name, ConfigLayerSource::Project { .. }))
        .collect();
    assert_eq!(project_layers_untrusted.len(), 1);
    assert!(
        project_layers_untrusted[0].disabled_reason.is_some(),
        "expected untrusted project layer to be disabled"
    );
    assert_eq!(
        project_layers_untrusted[0].config.get("foo"),
        Some(&TomlValue::String("child".to_string()))
    );
    assert!(
        project_layers_untrusted[0].config.get("profile").is_none(),
        "expected unsupported project config keys to be ignored even when the layer is disabled"
    );
    assert_eq!(
        layers_untrusted.effective_config().get("foo"),
        Some(&TomlValue::String("user".to_string()))
    );
    let empty_warnings: &[String] = &[];
    assert_eq!(layers_untrusted.startup_warnings(), Some(empty_warnings));

    let codex_home_unknown = tmp.path().join("home_unknown");
    tokio::fs::create_dir_all(&codex_home_unknown).await?;
    tokio::fs::write(
        codex_home_unknown.join(CONFIG_TOML_FILE),
        r#"foo = "user"
"#,
    )
    .await?;

    let layers_unknown = load_config_layers_state(
        LOCAL_FS.as_ref(),
        &codex_home_unknown,
        Some(cwd),
        &[] as &[(String, TomlValue)],
        LoaderOverrides::default(),
        CloudRequirementsLoader::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;
    let project_layers_unknown: Vec<_> = layers_unknown
        .get_layers(
            ConfigLayerStackOrdering::HighestPrecedenceFirst,
            /*include_disabled*/ true,
        )
        .into_iter()
        .filter(|layer| matches!(layer.name, ConfigLayerSource::Project { .. }))
        .collect();
    assert_eq!(project_layers_unknown.len(), 1);
    assert!(
        project_layers_unknown[0].disabled_reason.is_some(),
        "expected unknown-trust project layer to be disabled"
    );
    assert_eq!(
        project_layers_unknown[0].config.get("foo"),
        Some(&TomlValue::String("child".to_string()))
    );
    assert!(
        project_layers_unknown[0].config.get("profile").is_none(),
        "expected unsupported project config keys to be ignored even when the layer is disabled"
    );
    assert_eq!(
        layers_unknown.effective_config().get("foo"),
        Some(&TomlValue::String("user".to_string()))
    );
    assert_eq!(layers_unknown.startup_warnings(), Some(empty_warnings));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn project_trust_does_not_match_configured_alias_for_canonical_cwd() -> std::io::Result<()> {
    let tmp = tempdir()?;
    let project_root = tmp.path().join("project");
    let alias_root = tmp.path().join("project_alias");
    tokio::fs::create_dir_all(project_root.join(".codex")).await?;
    tokio::fs::write(project_root.join(".git"), "gitdir: here").await?;
    tokio::fs::write(
        project_root.join(".codex").join(CONFIG_TOML_FILE),
        r#"foo = "project"
"#,
    )
    .await?;
    std::os::unix::fs::symlink(&project_root, &alias_root)?;

    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    tokio::fs::write(
        codex_home.join(CONFIG_TOML_FILE),
        toml::to_string(&ConfigToml {
            projects: Some(HashMap::from([(
                alias_root.to_string_lossy().to_string(),
                ProjectConfig {
                    trust_level: Some(TrustLevel::Trusted),
                },
            )])),
            ..Default::default()
        })
        .expect("serialize config"),
    )
    .await?;

    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        &codex_home,
        Some(AbsolutePathBuf::from_absolute_path(&project_root)?),
        &[] as &[(String, TomlValue)],
        LoaderOverrides::default(),
        CloudRequirementsLoader::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    let project_layers: Vec<_> = layers
        .get_layers(
            ConfigLayerStackOrdering::HighestPrecedenceFirst,
            /*include_disabled*/ true,
        )
        .into_iter()
        .filter(|layer| matches!(layer.name, ConfigLayerSource::Project { .. }))
        .collect();
    assert_eq!(project_layers.len(), 1);
    assert!(
        project_layers[0].disabled_reason.is_some(),
        "configured aliases must not collapse into the canonical project key"
    );
    assert_eq!(layers.effective_config().get("foo"), None);

    Ok(())
}

#[tokio::test]
async fn cli_override_can_update_project_local_mcp_server_when_project_is_trusted()
-> std::io::Result<()> {
    let tmp = tempdir()?;
    let project_root = tmp.path().join("project");
    let nested = project_root.join("child");
    let dot_codex = project_root.join(".codex");
    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&nested).await?;
    tokio::fs::create_dir_all(&dot_codex).await?;
    tokio::fs::create_dir_all(&codex_home).await?;
    tokio::fs::write(project_root.join(".git"), "gitdir: here").await?;
    tokio::fs::write(
        dot_codex.join(CONFIG_TOML_FILE),
        r#"
[mcp_servers.sentry]
url = "https://mcp.sentry.dev/mcp"
enabled = false
"#,
    )
    .await?;
    make_config_for_test(
        &codex_home,
        &project_root,
        TrustLevel::Trusted,
        /*project_root_markers*/ None,
    )
    .await?;

    let config = ConfigBuilder::default()
        .codex_home(codex_home)
        .cli_overrides(vec![(
            "mcp_servers.sentry.enabled".to_string(),
            TomlValue::Boolean(true),
        )])
        .fallback_cwd(Some(nested))
        .build()
        .await?;

    let server = config
        .mcp_servers
        .get()
        .get("sentry")
        .expect("trusted project MCP server should load");
    assert!(server.enabled);

    Ok(())
}

#[tokio::test]
async fn cli_override_for_disabled_project_local_mcp_server_returns_invalid_transport()
-> std::io::Result<()> {
    let tmp = tempdir()?;
    let project_root = tmp.path().join("project");
    let nested = project_root.join("child");
    let dot_codex = project_root.join(".codex");
    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&nested).await?;
    tokio::fs::create_dir_all(&dot_codex).await?;
    tokio::fs::create_dir_all(&codex_home).await?;
    tokio::fs::write(project_root.join(".git"), "gitdir: here").await?;
    tokio::fs::write(
        dot_codex.join(CONFIG_TOML_FILE),
        r#"
[mcp_servers.sentry]
url = "https://mcp.sentry.dev/mcp"
enabled = false
"#,
    )
    .await?;

    let err = ConfigBuilder::default()
        .codex_home(codex_home)
        .cli_overrides(vec![(
            "mcp_servers.sentry.enabled".to_string(),
            TomlValue::Boolean(true),
        )])
        .fallback_cwd(Some(nested))
        .build()
        .await
        .expect_err("untrusted project layer should not provide MCP transport");

    assert!(
        err.to_string().contains("invalid transport")
            && err.to_string().contains("mcp_servers.sentry"),
        "unexpected error: {err}"
    );

    Ok(())
}

#[tokio::test]
async fn invalid_project_config_ignored_when_untrusted_or_unknown() -> std::io::Result<()> {
    let tmp = tempdir()?;
    let project_root = tmp.path().join("project");
    let nested = project_root.join("child");
    tokio::fs::create_dir_all(nested.join(".codex")).await?;
    tokio::fs::write(project_root.join(".git"), "gitdir: here").await?;
    tokio::fs::write(nested.join(".codex").join(CONFIG_TOML_FILE), "foo =").await?;

    let cwd = AbsolutePathBuf::from_absolute_path(&nested)?;
    let cases = [
        ("untrusted", Some(TrustLevel::Untrusted)),
        ("unknown", None),
    ];

    for (name, trust_level) in cases {
        let codex_home = tmp.path().join(format!("home_{name}"));
        tokio::fs::create_dir_all(&codex_home).await?;
        let config_path = codex_home.join(CONFIG_TOML_FILE);

        if let Some(trust_level) = trust_level {
            make_config_for_test(
                &codex_home,
                &project_root,
                trust_level,
                /*project_root_markers*/ None,
            )
            .await?;
            let config_contents = tokio::fs::read_to_string(&config_path).await?;
            tokio::fs::write(
                &config_path,
                format!(
                    r#"foo = "user"
{config_contents}"#
                ),
            )
            .await?;
        } else {
            tokio::fs::write(
                &config_path,
                r#"foo = "user"
"#,
            )
            .await?;
        }

        let layers = load_config_layers_state(
            LOCAL_FS.as_ref(),
            &codex_home,
            Some(cwd.clone()),
            &[] as &[(String, TomlValue)],
            LoaderOverrides::default(),
            CloudRequirementsLoader::default(),
            &codex_config::NoopThreadConfigLoader,
        )
        .await?;
        let project_layers: Vec<_> = layers
            .get_layers(
                ConfigLayerStackOrdering::HighestPrecedenceFirst,
                /*include_disabled*/ true,
            )
            .into_iter()
            .filter(|layer| matches!(layer.name, ConfigLayerSource::Project { .. }))
            .collect();
        assert_eq!(
            project_layers.len(),
            1,
            "expected one project layer for {name}"
        );
        assert!(
            project_layers[0].disabled_reason.is_some(),
            "expected {name} project layer to be disabled"
        );
        assert_eq!(
            project_layers[0].config,
            TomlValue::Table(toml::map::Map::new())
        );
        assert_eq!(
            layers.effective_config().get("foo"),
            Some(&TomlValue::String("user".to_string()))
        );
    }

    Ok(())
}

#[tokio::test]
async fn project_layer_without_config_toml_is_disabled_when_untrusted_or_unknown()
-> std::io::Result<()> {
    let tmp = tempdir()?;
    let project_root = tmp.path().join("project");
    let nested = project_root.join("child");
    tokio::fs::create_dir_all(nested.join(".codex")).await?;
    tokio::fs::write(project_root.join(".git"), "gitdir: here").await?;

    let cwd = AbsolutePathBuf::from_absolute_path(&nested)?;
    let cases = [
        ("untrusted", Some(TrustLevel::Untrusted), true),
        ("unknown", None, true),
        ("trusted", Some(TrustLevel::Trusted), false),
    ];

    for (name, trust_level, expect_disabled) in cases {
        let codex_home = tmp.path().join(format!("home_no_config_{name}"));
        tokio::fs::create_dir_all(&codex_home).await?;
        if let Some(trust_level) = trust_level {
            make_config_for_test(
                &codex_home,
                &project_root,
                trust_level,
                /*project_root_markers*/ None,
            )
            .await?;
        }

        let layers = load_config_layers_state(
            LOCAL_FS.as_ref(),
            &codex_home,
            Some(cwd.clone()),
            &[] as &[(String, TomlValue)],
            LoaderOverrides::default(),
            CloudRequirementsLoader::default(),
            &codex_config::NoopThreadConfigLoader,
        )
        .await?;
        let project_layers: Vec<_> = layers
            .get_layers(
                ConfigLayerStackOrdering::HighestPrecedenceFirst,
                /*include_disabled*/ true,
            )
            .into_iter()
            .filter(|layer| matches!(layer.name, ConfigLayerSource::Project { .. }))
            .collect();
        assert_eq!(
            project_layers.len(),
            1,
            "expected one project layer for {name}"
        );
        assert_eq!(
            project_layers[0].disabled_reason.is_some(),
            expect_disabled,
            "unexpected disabled state for {name}",
        );
        assert_eq!(
            project_layers[0].config,
            TomlValue::Table(toml::map::Map::new())
        );
    }

    Ok(())
}

#[tokio::test]
async fn cli_overrides_with_relative_paths_do_not_break_trust_check() -> std::io::Result<()> {
    let tmp = tempdir()?;
    let project_root = tmp.path().join("project");
    let nested = project_root.join("child");
    tokio::fs::create_dir_all(&nested).await?;
    tokio::fs::write(project_root.join(".git"), "gitdir: here").await?;

    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    make_config_for_test(
        &codex_home,
        &project_root,
        TrustLevel::Trusted,
        /*project_root_markers*/ None,
    )
    .await?;

    let cwd = AbsolutePathBuf::from_absolute_path(&nested)?;
    let cli_overrides = vec![(
        "model_instructions_file".to_string(),
        TomlValue::String("relative.md".to_string()),
    )];

    load_config_layers_state(
        LOCAL_FS.as_ref(),
        &codex_home,
        Some(cwd),
        &cli_overrides,
        LoaderOverrides::default(),
        CloudRequirementsLoader::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    Ok(())
}
