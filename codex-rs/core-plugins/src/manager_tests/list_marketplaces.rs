use super::super::*;
use super::*;

#[tokio::test]
async fn list_marketplaces_includes_enabled_state() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin(
        &tmp.path().join("plugins/cache/debug"),
        "enabled-plugin/local",
        "enabled-plugin",
    );
    write_plugin(
        &tmp.path().join("plugins/cache/debug"),
        "disabled-plugin/local",
        "disabled-plugin",
    );
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "enabled-plugin",
      "source": {
        "source": "local",
        "path": "./enabled-plugin"
      }
    },
    {
      "name": "disabled-plugin",
      "source": {
        "source": "local",
        "path": "./disabled-plugin"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."enabled-plugin@debug"]
enabled = true

[plugins."disabled-plugin@debug"]
enabled = false
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let marketplaces = PluginsManager::new(tmp.path().to_path_buf())
        .list_marketplaces_for_config(&config, &[AbsolutePathBuf::try_from(repo_root).unwrap()])
        .unwrap()
        .marketplaces;

    let marketplace = marketplaces
        .into_iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    tmp.path().join("repo/.agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("expected repo marketplace entry");

    assert_eq!(
        marketplace,
        ConfiguredMarketplace {
            name: "debug".to_string(),
            path: AbsolutePathBuf::try_from(
                tmp.path().join("repo/.agents/plugins/marketplace.json"),
            )
            .unwrap(),
            interface: None,
            plugins: vec![
                ConfiguredMarketplacePlugin {
                    id: "enabled-plugin@debug".to_string(),
                    name: "enabled-plugin".to_string(),
                    local_version: None,
                    installed_version: Some("local".to_string()),
                    source: MarketplacePluginSource::Local {
                        path: AbsolutePathBuf::try_from(tmp.path().join("repo/enabled-plugin"))
                            .unwrap(),
                    },
                    policy: MarketplacePluginPolicy {
                        installation: MarketplacePluginInstallPolicy::Available,
                        authentication: MarketplacePluginAuthPolicy::OnInstall,
                        products: None,
                    },
                    interface: None,
                    keywords: Vec::new(),
                    installed: true,
                    enabled: true,
                },
                ConfiguredMarketplacePlugin {
                    id: "disabled-plugin@debug".to_string(),
                    name: "disabled-plugin".to_string(),
                    local_version: None,
                    installed_version: Some("local".to_string()),
                    source: MarketplacePluginSource::Local {
                        path: AbsolutePathBuf::try_from(tmp.path().join("repo/disabled-plugin"),)
                            .unwrap(),
                    },
                    policy: MarketplacePluginPolicy {
                        installation: MarketplacePluginInstallPolicy::Available,
                        authentication: MarketplacePluginAuthPolicy::OnInstall,
                        products: None,
                    },
                    interface: None,
                    keywords: Vec::new(),
                    installed: true,
                    enabled: false,
                },
            ],
        }
    );
}

#[tokio::test]
async fn list_marketplaces_returns_empty_when_feature_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "enabled-plugin",
      "source": {
        "source": "local",
        "path": "./enabled-plugin"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = false

[plugins."enabled-plugin@debug"]
enabled = true
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let marketplaces = PluginsManager::new(tmp.path().to_path_buf())
        .list_marketplaces_for_config(&config, &[AbsolutePathBuf::try_from(repo_root).unwrap()])
        .unwrap()
        .marketplaces;

    assert_eq!(marketplaces, Vec::new());
}

#[tokio::test]
async fn list_marketplaces_excludes_plugins_with_explicit_empty_products() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "disabled-plugin",
      "source": {
        "source": "local",
        "path": "./disabled-plugin"
      },
      "policy": {
        "products": []
      }
    },
    {
      "name": "default-plugin",
      "source": {
        "source": "local",
        "path": "./default-plugin"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let marketplaces = PluginsManager::new(tmp.path().to_path_buf())
        .list_marketplaces_for_config(&config, &[AbsolutePathBuf::try_from(repo_root).unwrap()])
        .unwrap()
        .marketplaces;

    let marketplace = marketplaces
        .into_iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    tmp.path().join("repo/.agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("expected repo marketplace entry");
    assert_eq!(
        marketplace.plugins,
        vec![ConfiguredMarketplacePlugin {
            id: "default-plugin@debug".to_string(),
            name: "default-plugin".to_string(),
            local_version: None,
            installed_version: None,
            source: MarketplacePluginSource::Local {
                path: AbsolutePathBuf::try_from(tmp.path().join("repo/default-plugin")).unwrap(),
            },
            policy: MarketplacePluginPolicy {
                installation: MarketplacePluginInstallPolicy::Available,
                authentication: MarketplacePluginAuthPolicy::OnInstall,
                products: None,
            },
            interface: None,
            keywords: Vec::new(),
            installed: false,
            enabled: false,
        }]
    );
}

#[tokio::test]
async fn list_marketplaces_uses_first_duplicate_plugin_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_a_root = tmp.path().join("repo-a");
    let repo_b_root = tmp.path().join("repo-b");
    fs::create_dir_all(repo_a_root.join(".git")).unwrap();
    fs::create_dir_all(repo_b_root.join(".git")).unwrap();
    fs::create_dir_all(repo_a_root.join(".agents/plugins")).unwrap();
    fs::create_dir_all(repo_b_root.join(".agents/plugins")).unwrap();
    fs::write(
        repo_a_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "dup-plugin",
      "source": {
        "source": "local",
        "path": "./from-a"
      }
    }
  ]
}"#,
    )
    .unwrap();
    fs::write(
        repo_b_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "dup-plugin",
      "source": {
        "source": "local",
        "path": "./from-b"
      }
    },
    {
      "name": "b-only-plugin",
      "source": {
        "source": "local",
        "path": "./from-b-only"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."dup-plugin@debug"]
enabled = true

[plugins."b-only-plugin@debug"]
enabled = false
"#,
    );

    let config = load_config(tmp.path(), &repo_a_root).await;
    let marketplaces = PluginsManager::new(tmp.path().to_path_buf())
        .list_marketplaces_for_config(
            &config,
            &[
                AbsolutePathBuf::try_from(repo_a_root).unwrap(),
                AbsolutePathBuf::try_from(repo_b_root).unwrap(),
            ],
        )
        .unwrap()
        .marketplaces;

    let repo_a_marketplace = marketplaces
        .iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    tmp.path().join("repo-a/.agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("repo-a marketplace should be listed");
    assert_eq!(
        repo_a_marketplace.plugins,
        vec![ConfiguredMarketplacePlugin {
            id: "dup-plugin@debug".to_string(),
            name: "dup-plugin".to_string(),
            local_version: None,
            installed_version: None,
            source: MarketplacePluginSource::Local {
                path: AbsolutePathBuf::try_from(tmp.path().join("repo-a/from-a")).unwrap(),
            },
            policy: MarketplacePluginPolicy {
                installation: MarketplacePluginInstallPolicy::Available,
                authentication: MarketplacePluginAuthPolicy::OnInstall,
                products: None,
            },
            interface: None,
            keywords: Vec::new(),
            installed: false,
            enabled: true,
        }]
    );

    let repo_b_marketplace = marketplaces
        .iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    tmp.path().join("repo-b/.agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("repo-b marketplace should be listed");
    assert_eq!(
        repo_b_marketplace.plugins,
        vec![ConfiguredMarketplacePlugin {
            id: "b-only-plugin@debug".to_string(),
            name: "b-only-plugin".to_string(),
            local_version: None,
            installed_version: None,
            source: MarketplacePluginSource::Local {
                path: AbsolutePathBuf::try_from(tmp.path().join("repo-b/from-b-only")).unwrap(),
            },
            policy: MarketplacePluginPolicy {
                installation: MarketplacePluginInstallPolicy::Available,
                authentication: MarketplacePluginAuthPolicy::OnInstall,
                products: None,
            },
            interface: None,
            keywords: Vec::new(),
            installed: false,
            enabled: false,
        }]
    );

    let duplicate_plugin_count = marketplaces
        .iter()
        .flat_map(|marketplace| marketplace.plugins.iter())
        .filter(|plugin| plugin.name == "dup-plugin")
        .count();
    assert_eq!(duplicate_plugin_count, 1);
}

#[tokio::test]
async fn list_marketplaces_marks_configured_plugin_uninstalled_when_cache_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let marketplaces = PluginsManager::new(tmp.path().to_path_buf())
        .list_marketplaces_for_config(&config, &[AbsolutePathBuf::try_from(repo_root).unwrap()])
        .unwrap()
        .marketplaces;

    let marketplace = marketplaces
        .into_iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    tmp.path().join("repo/.agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("expected repo marketplace entry");

    assert_eq!(
        marketplace,
        ConfiguredMarketplace {
            name: "debug".to_string(),
            path: AbsolutePathBuf::try_from(
                tmp.path().join("repo/.agents/plugins/marketplace.json"),
            )
            .unwrap(),
            interface: None,
            plugins: vec![ConfiguredMarketplacePlugin {
                id: "sample-plugin@debug".to_string(),
                name: "sample-plugin".to_string(),
                local_version: None,
                installed_version: None,
                source: MarketplacePluginSource::Local {
                    path: AbsolutePathBuf::try_from(tmp.path().join("repo/sample-plugin")).unwrap(),
                },
                policy: MarketplacePluginPolicy {
                    installation: MarketplacePluginInstallPolicy::Available,
                    authentication: MarketplacePluginAuthPolicy::OnInstall,
                    products: None,
                },
                interface: None,
                keywords: Vec::new(),
                installed: false,
                enabled: true,
            }],
        }
    );
}
