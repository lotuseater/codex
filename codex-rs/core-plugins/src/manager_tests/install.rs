use super::super::*;
use super::*;

#[tokio::test]
async fn install_plugin_updates_config_with_relative_path_and_plugin_key() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin(&repo_root, "sample-plugin", "sample-plugin");
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
      },
      "policy": {
        "authentication": "ON_USE"
      }
    }
  ]
}"#,
    )
    .unwrap();

    let result = PluginsManager::new(tmp.path().to_path_buf())
        .install_plugin(PluginInstallRequest {
            plugin_name: "sample-plugin".to_string(),
            marketplace_path: AbsolutePathBuf::try_from(
                repo_root.join(".agents/plugins/marketplace.json"),
            )
            .unwrap(),
        })
        .await
        .unwrap();

    let installed_path = tmp.path().join("plugins/cache/debug/sample-plugin/local");
    assert_eq!(
        result,
        PluginInstallOutcome {
            plugin_id: PluginId::new("sample-plugin".to_string(), "debug".to_string()).unwrap(),
            plugin_version: "local".to_string(),
            installed_path: AbsolutePathBuf::try_from(installed_path).unwrap(),
            auth_policy: MarketplacePluginAuthPolicy::OnUse,
        }
    );

    let config = fs::read_to_string(tmp.path().join("config.toml")).unwrap();
    assert!(config.contains(r#"[plugins."sample-plugin@debug"]"#));
    assert!(config.contains("enabled = true"));
}

#[tokio::test]
async fn install_openai_curated_plugin_uses_short_sha_cache_version() {
    let tmp = tempfile::tempdir().unwrap();
    let curated_root = curated_plugins_repo_path(tmp.path());
    write_openai_curated_marketplace(&curated_root, &["slack"]);
    write_curated_plugin_sha(tmp.path(), TEST_CURATED_PLUGIN_SHA);

    let result = PluginsManager::new(tmp.path().to_path_buf())
        .install_plugin(PluginInstallRequest {
            plugin_name: "slack".to_string(),
            marketplace_path: AbsolutePathBuf::try_from(
                curated_root.join(".agents/plugins/marketplace.json"),
            )
            .unwrap(),
        })
        .await
        .unwrap();

    let installed_path = tmp.path().join(format!(
        "plugins/cache/openai-curated/slack/{TEST_CURATED_PLUGIN_CACHE_VERSION}"
    ));
    assert_eq!(
        result,
        PluginInstallOutcome {
            plugin_id: PluginId::new(
                "slack".to_string(),
                OPENAI_CURATED_MARKETPLACE_NAME.to_string()
            )
            .unwrap(),
            plugin_version: TEST_CURATED_PLUGIN_CACHE_VERSION.to_string(),
            installed_path: AbsolutePathBuf::try_from(installed_path).unwrap(),
            auth_policy: MarketplacePluginAuthPolicy::OnInstall,
        }
    );
}

#[tokio::test]
async fn install_plugin_uses_manifest_version_for_non_curated_plugins() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin_with_version(
        &repo_root,
        "sample-plugin",
        "sample-plugin",
        Some("1.2.3-beta+7"),
    );
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

    let result = PluginsManager::new(tmp.path().to_path_buf())
        .install_plugin(PluginInstallRequest {
            plugin_name: "sample-plugin".to_string(),
            marketplace_path: AbsolutePathBuf::try_from(
                repo_root.join(".agents/plugins/marketplace.json"),
            )
            .unwrap(),
        })
        .await
        .unwrap();

    let installed_path = tmp
        .path()
        .join("plugins/cache/debug/sample-plugin/1.2.3-beta+7");
    assert_eq!(
        result,
        PluginInstallOutcome {
            plugin_id: PluginId::new("sample-plugin".to_string(), "debug".to_string()).unwrap(),
            plugin_version: "1.2.3-beta+7".to_string(),
            installed_path: AbsolutePathBuf::try_from(installed_path).unwrap(),
            auth_policy: MarketplacePluginAuthPolicy::OnInstall,
        }
    );
}

#[tokio::test]
async fn install_plugin_supports_git_subdir_marketplace_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("marketplace");
    let remote_repo = tmp.path().join("remote-plugin-repo");
    let remote_repo_url = url::Url::from_directory_path(&remote_repo)
        .unwrap()
        .to_string();
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin(&remote_repo, "plugins/toolkit", "toolkit");
    init_git_repo(&remote_repo);
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        format!(
            r#"{{
  "name": "debug",
  "plugins": [
    {{
      "name": "toolkit",
      "source": {{
        "source": "git-subdir",
        "url": "{remote_repo_url}",
        "path": "plugins/toolkit"
      }}
    }}
  ]
}}"#
        ),
    )
    .unwrap();

    let result = PluginsManager::new(tmp.path().to_path_buf())
        .install_plugin(PluginInstallRequest {
            plugin_name: "toolkit".to_string(),
            marketplace_path: AbsolutePathBuf::try_from(
                repo_root.join(".agents/plugins/marketplace.json"),
            )
            .unwrap(),
        })
        .await
        .unwrap();

    let installed_path = tmp.path().join("plugins/cache/debug/toolkit/local");
    assert_eq!(
        result,
        PluginInstallOutcome {
            plugin_id: PluginId::new("toolkit".to_string(), "debug".to_string()).unwrap(),
            plugin_version: "local".to_string(),
            installed_path: AbsolutePathBuf::try_from(installed_path.clone()).unwrap(),
            auth_policy: MarketplacePluginAuthPolicy::OnInstall,
        }
    );
    assert!(installed_path.join(".codex-plugin/plugin.json").is_file());
}

#[tokio::test]
async fn install_plugin_supports_relative_git_subdir_marketplace_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("marketplace");
    let remote_repo = repo_root.join("remote-plugin-repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin(&remote_repo, "plugins/toolkit", "toolkit");
    init_git_repo(&remote_repo);
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "toolkit",
      "source": {
        "source": "git-subdir",
        "url": "./remote-plugin-repo",
        "path": "plugins/toolkit"
      }
    }
  ]
}"#,
    )
    .unwrap();

    let result = PluginsManager::new(tmp.path().to_path_buf())
        .install_plugin(PluginInstallRequest {
            plugin_name: "toolkit".to_string(),
            marketplace_path: AbsolutePathBuf::try_from(
                repo_root.join(".agents/plugins/marketplace.json"),
            )
            .unwrap(),
        })
        .await
        .unwrap();

    let installed_path = tmp.path().join("plugins/cache/debug/toolkit/local");
    assert_eq!(
        result,
        PluginInstallOutcome {
            plugin_id: PluginId::new("toolkit".to_string(), "debug".to_string()).unwrap(),
            plugin_version: "local".to_string(),
            installed_path: AbsolutePathBuf::try_from(installed_path.clone()).unwrap(),
            auth_policy: MarketplacePluginAuthPolicy::OnInstall,
        }
    );
    assert!(installed_path.join(".codex-plugin/plugin.json").is_file());
}

#[tokio::test]
async fn uninstall_plugin_removes_cache_and_config_entry() {
    let tmp = tempfile::tempdir().unwrap();
    write_plugin(
        &tmp.path().join("plugins/cache/debug"),
        "sample-plugin/local",
        "sample-plugin",
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    let manager = PluginsManager::new(tmp.path().to_path_buf());
    manager
        .uninstall_plugin("sample-plugin@debug".to_string())
        .await
        .unwrap();
    manager
        .uninstall_plugin("sample-plugin@debug".to_string())
        .await
        .unwrap();

    assert!(
        !tmp.path()
            .join("plugins/cache/debug/sample-plugin")
            .exists()
    );
    let config = fs::read_to_string(tmp.path().join(CONFIG_TOML_FILE)).unwrap();
    assert!(!config.contains(r#"[plugins."sample-plugin@debug"]"#));
}
