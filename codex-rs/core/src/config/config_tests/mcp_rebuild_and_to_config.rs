use super::*;
use super::common::*;

#[tokio::test]
async fn rebuild_preserving_session_layers_refreshes_requirements() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let user_file = AbsolutePathBuf::resolve_path_against_base(CONFIG_TOML_FILE, codex_home.path());
    let project_dot_codex =
        AbsolutePathBuf::resolve_path_against_base("project/.codex", codex_home.path());
    let mcp_requirements = BTreeMap::from([
        (
            "session_overrides_user".to_string(),
            McpServerRequirement {
                identity: McpServerIdentity::Command {
                    command: "session-command".to_string(),
                },
            },
        ),
        (
            "managed_overrides_session".to_string(),
            McpServerRequirement {
                identity: McpServerIdentity::Command {
                    command: "managed-command".to_string(),
                },
            },
        ),
        (
            "fresh_global".to_string(),
            McpServerRequirement {
                identity: McpServerIdentity::Command {
                    command: "fresh-global-command".to_string(),
                },
            },
        ),
        (
            "fresh_project".to_string(),
            McpServerRequirement {
                identity: McpServerIdentity::Command {
                    command: "fresh-project-command".to_string(),
                },
            },
        ),
    ]);
    let requirements_toml = codex_config::ConfigRequirementsToml {
        mcp_servers: Some(mcp_requirements.clone()),
        ..Default::default()
    };
    let requirements = codex_config::ConfigRequirements {
        mcp_servers: Some(Sourced::new(mcp_requirements, RequirementSource::Unknown)),
        ..Default::default()
    };
    let refreshed_layer_stack = ConfigLayerStack::new(
        vec![
            ConfigLayerEntry::new(
                codex_config::ConfigLayerSource::User {
                    file: user_file.clone(),
                    profile: None,
                },
                toml::toml! {
                    [mcp_servers.session_overrides_user]
                    command = "new-user-command"
                    [mcp_servers.managed_overrides_session]
                    command = "new-user-command"
                    [mcp_servers.fresh_global]
                    command = "fresh-global-command"
                }
                .into(),
            ),
            ConfigLayerEntry::new(
                codex_config::ConfigLayerSource::Project {
                    dot_codex_folder: project_dot_codex.clone(),
                },
                toml::toml! {
                    [mcp_servers.fresh_project]
                    command = "fresh-project-command"
                }
                .into(),
            ),
            ConfigLayerEntry::new(
                codex_config::ConfigLayerSource::LegacyManagedConfigTomlFromMdm,
                toml::toml! {
                    [mcp_servers.managed_overrides_session]
                    command = "managed-command"
                }
                .into(),
            ),
        ],
        requirements,
        requirements_toml,
    )
    .map_err(std::io::Error::other)?;
    let refreshed_toml = refreshed_layer_stack
        .effective_config()
        .try_into()
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    let refreshed_config = Config::load_config_with_layer_stack(
        LOCAL_FS.as_ref(),
        refreshed_toml,
        ConfigOverrides {
            cwd: Some(codex_home.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
        refreshed_layer_stack,
    )
    .await?;
    let thread_layer_stack = ConfigLayerStack::new(
        vec![
            ConfigLayerEntry::new(
                codex_config::ConfigLayerSource::User {
                    file: user_file.clone(),
                    profile: None,
                },
                toml::toml! {
                    [mcp_servers.session_overrides_user]
                    command = "old-user-command"
                    [mcp_servers.managed_overrides_session]
                    command = "old-user-command"
                    [mcp_servers.fresh_global]
                    command = "old-global-command"
                }
                .into(),
            ),
            ConfigLayerEntry::new(
                codex_config::ConfigLayerSource::Project {
                    dot_codex_folder: project_dot_codex,
                },
                toml::toml! {
                    [mcp_servers.fresh_project]
                    command = "old-project-command"
                }
                .into(),
            ),
            ConfigLayerEntry::new(
                codex_config::ConfigLayerSource::SessionFlags,
                toml::toml! {
                    [mcp_servers.session_overrides_user]
                    command = "session-command"
                    [mcp_servers.managed_overrides_session]
                    command = "session-command"
                    [mcp_servers.blocked_session]
                    command = "blocked-session-command"
                }
                .into(),
            ),
            ConfigLayerEntry::new(
                codex_config::ConfigLayerSource::LegacyManagedConfigTomlFromMdm,
                toml::toml! {
                    [mcp_servers.managed_overrides_session]
                    command = "old-managed-command"
                }
                .into(),
            ),
        ],
        Default::default(),
        Default::default(),
    )
    .map_err(std::io::Error::other)?;
    let thread_toml = thread_layer_stack
        .effective_config()
        .try_into()
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    let thread_config = Config::load_config_with_layer_stack(
        LOCAL_FS.as_ref(),
        thread_toml,
        ConfigOverrides {
            cwd: Some(codex_home.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
        thread_layer_stack,
    )
    .await?;
    let config = thread_config
        .rebuild_preserving_session_layers(&refreshed_config)
        .await?;

    assert_eq!(
        config.mcp_servers.get(),
        &HashMap::from([
            (
                "session_overrides_user".to_string(),
                stdio_mcp("session-command"),
            ),
            (
                "managed_overrides_session".to_string(),
                stdio_mcp("managed-command"),
            ),
            (
                "fresh_global".to_string(),
                stdio_mcp("fresh-global-command"),
            ),
            (
                "fresh_project".to_string(),
                stdio_mcp("fresh-project-command"),
            ),
            (
                "blocked_session".to_string(),
                McpServerConfig {
                    enabled: false,
                    disabled_reason: Some(McpServerDisabledReason::Requirements {
                        source: RequirementSource::Unknown,
                    }),
                    ..stdio_mcp("blocked-session-command")
                },
            ),
        ])
    );

    Ok(())
}

#[tokio::test]
async fn rebuild_preserving_session_layers_refreshes_plugin_derived_mcp_config()
-> anyhow::Result<()> {
    let codex_home = TempDir::new()?;
    let plugin_root = codex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");
    std::fs::create_dir_all(plugin_root.join(".codex-plugin"))?;
    std::fs::write(
        plugin_root.join(".codex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    )?;
    std::fs::write(
        plugin_root.join(".mcp.json"),
        r#"{
  "mcpServers": {
    "sample": {
      "type": "http",
      "url": "https://sample.example/mcp"
    }
  }
}"#,
    )?;

    let user_file = AbsolutePathBuf::resolve_path_against_base(CONFIG_TOML_FILE, codex_home.path());
    let refreshed_layer_stack = ConfigLayerStack::new(
        vec![ConfigLayerEntry::new(
            codex_config::ConfigLayerSource::User {
                file: user_file.clone(),
                profile: None,
            },
            toml::toml! {
                [features]
                plugins = true

                [plugins."sample@test"]
                enabled = true
            }
            .into(),
        )],
        Default::default(),
        Default::default(),
    )?;
    let refreshed_config = Config::load_config_with_layer_stack(
        LOCAL_FS.as_ref(),
        refreshed_layer_stack.effective_config().try_into()?,
        ConfigOverrides {
            cwd: Some(codex_home.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
        refreshed_layer_stack,
    )
    .await?;
    let thread_layer_stack = ConfigLayerStack::new(
        vec![ConfigLayerEntry::new(
            codex_config::ConfigLayerSource::User {
                file: user_file,
                profile: None,
            },
            toml::toml! {
                [features]
                plugins = false

                [plugins."sample@test"]
                enabled = true
            }
            .into(),
        )],
        Default::default(),
        Default::default(),
    )?;
    let thread_config = Config::load_config_with_layer_stack(
        LOCAL_FS.as_ref(),
        thread_layer_stack.effective_config().try_into()?,
        ConfigOverrides {
            cwd: Some(codex_home.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
        thread_layer_stack,
    )
    .await?;
    let config = thread_config
        .rebuild_preserving_session_layers(&refreshed_config)
        .await?;
    let plugins_manager = PluginsManager::new(codex_home.path().to_path_buf());
    let mcp_config = config.to_mcp_config(&plugins_manager).await;

    assert_eq!(
        mcp_config.configured_mcp_servers.get("sample"),
        Some(&http_mcp("https://sample.example/mcp"))
    );
    assert_eq!(
        mcp_config.plugin_ids_by_mcp_server_name,
        HashMap::from([("sample".to_string(), "sample@test".to_string())])
    );

    Ok(())
}

#[tokio::test]
async fn to_mcp_config_omits_plugin_id_when_user_server_shadows_plugin_mcp() -> anyhow::Result<()> {
    let codex_home = TempDir::new()?;
    let plugin_root = codex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");
    std::fs::create_dir_all(plugin_root.join(".codex-plugin"))?;
    std::fs::write(
        plugin_root.join(".codex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    )?;
    std::fs::write(
        plugin_root.join(".mcp.json"),
        r#"{
  "mcpServers": {
    "sample": {
      "type": "http",
      "url": "https://plugin.example/mcp"
    }
  }
}"#,
    )?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"
[features]
plugins = true

[mcp_servers.sample]
url = "https://user.example/mcp"

[plugins."sample@test"]
enabled = true
"#,
    )?;

    let config = ConfigBuilder::default()
        .codex_home(codex_home.path().to_path_buf())
        .build()
        .await?;
    let plugins_manager = PluginsManager::new(codex_home.path().to_path_buf());
    let mcp_config = config.to_mcp_config(&plugins_manager).await;

    assert_eq!(
        mcp_config.configured_mcp_servers.get("sample"),
        Some(&http_mcp("https://user.example/mcp"))
    );
    assert!(mcp_config.plugin_ids_by_mcp_server_name.is_empty());

    Ok(())
}

#[tokio::test]
async fn to_mcp_config_applies_plugin_mcp_cloud_requirements() -> anyhow::Result<()> {
    let codex_home = TempDir::new()?;
    let plugin_root = codex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");
    std::fs::create_dir_all(plugin_root.join(".codex-plugin"))?;
    std::fs::write(
        plugin_root.join(".codex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    )?;
    std::fs::write(
        plugin_root.join(".mcp.json"),
        r#"{
  "mcpServers": {
    "sample": {
      "type": "http",
      "url": "https://sample.example/mcp"
    },
    "unlisted": {
      "type": "http",
      "url": "https://unlisted.example/mcp"
    }
  }
}"#,
    )?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"
[features]
plugins = true

[plugins."sample@test"]
enabled = true
"#,
    )?;

    let requirements = codex_config::ConfigRequirementsToml {
        plugins: Some(BTreeMap::from([(
            "sample@test".to_string(),
            codex_config::PluginRequirementsToml {
                mcp_servers: Some(BTreeMap::from([(
                    "sample".to_string(),
                    McpServerRequirement {
                        identity: McpServerIdentity::Url {
                            url: "https://sample.example/mcp".to_string(),
                        },
                    },
                )])),
            },
        )])),
        ..Default::default()
    };
    let config = ConfigBuilder::default()
        .codex_home(codex_home.path().to_path_buf())
        .cloud_requirements(CloudRequirementsLoader::new(async move {
            Ok(Some(requirements))
        }))
        .build()
        .await?;
    let plugins_manager = PluginsManager::new(codex_home.path().to_path_buf());
    let mcp_config = config.to_mcp_config(&plugins_manager).await;

    assert_eq!(
        mcp_config
            .configured_mcp_servers
            .get("sample")
            .map(|server| (server.enabled, server.disabled_reason.clone())),
        Some((true, None))
    );
    assert_eq!(
        mcp_config
            .configured_mcp_servers
            .get("unlisted")
            .map(|server| (server.enabled, server.disabled_reason.clone())),
        Some((
            false,
            Some(McpServerDisabledReason::Requirements {
                source: RequirementSource::CloudRequirements,
            })
        ))
    );
    Ok(())
}

#[tokio::test]
async fn to_mcp_config_empty_mcp_requirements_disable_plugin_mcps() -> anyhow::Result<()> {
    let codex_home = TempDir::new()?;
    let plugin_root = codex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");
    std::fs::create_dir_all(plugin_root.join(".codex-plugin"))?;
    std::fs::write(
        plugin_root.join(".codex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    )?;
    std::fs::write(
        plugin_root.join(".mcp.json"),
        r#"{
  "mcpServers": {
    "sample": {
      "type": "http",
      "url": "https://sample.example/mcp"
    }
  }
}"#,
    )?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"
[features]
plugins = true

[plugins."sample@test"]
enabled = true
"#,
    )?;

    let requirements = codex_config::ConfigRequirementsToml {
        mcp_servers: Some(BTreeMap::new()),
        ..Default::default()
    };
    let config = ConfigBuilder::default()
        .codex_home(codex_home.path().to_path_buf())
        .cloud_requirements(CloudRequirementsLoader::new(async move {
            Ok(Some(requirements))
        }))
        .build()
        .await?;
    let plugins_manager = PluginsManager::new(codex_home.path().to_path_buf());
    let mcp_config = config.to_mcp_config(&plugins_manager).await;

    assert_eq!(
        mcp_config
            .configured_mcp_servers
            .get("sample")
            .map(|server| (server.enabled, server.disabled_reason.clone())),
        Some((
            false,
            Some(McpServerDisabledReason::Requirements {
                source: RequirementSource::CloudRequirements,
            })
        ))
    );
    Ok(())
}

