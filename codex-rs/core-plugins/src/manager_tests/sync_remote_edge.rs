use super::*;
use super::super::*;

#[tokio::test]
async fn sync_plugins_from_remote_ignores_unknown_remote_plugins() {
    let tmp = tempfile::tempdir().unwrap();
    let curated_root = curated_plugins_repo_path(tmp.path());
    write_openai_curated_marketplace(&curated_root, &["linear"]);
    write_curated_plugin_sha(tmp.path(), TEST_CURATED_PLUGIN_SHA);
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."linear@openai-curated"]
enabled = false
"#,
    );

    let server = MockServer::start().await;
    Mock::given(method("GET"))
            .and(path("/backend-api/plugins/list"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"[
  {"id":"1","name":"plugin-one","marketplace_name":"openai-curated","version":"1.0.0","enabled":true}
]"#,
            ))
            .mount(&server)
            .await;

    let mut config = load_config(tmp.path(), tmp.path()).await;
    config.chatgpt_base_url = format!("{}/backend-api/", server.uri());
    let manager = PluginsManager::new(tmp.path().to_path_buf());
    let result = manager
        .sync_plugins_from_remote(
            &config,
            Some(&CodexAuth::create_dummy_chatgpt_auth_for_testing()),
            /*additive_only*/ false,
        )
        .await
        .unwrap();

    assert_eq!(
        result,
        RemotePluginSyncResult {
            installed_plugin_ids: Vec::new(),
            enabled_plugin_ids: Vec::new(),
            disabled_plugin_ids: Vec::new(),
            uninstalled_plugin_ids: vec!["linear@openai-curated".to_string()],
        }
    );
    let config = fs::read_to_string(tmp.path().join(CONFIG_TOML_FILE)).unwrap();
    assert!(!config.contains(r#"[plugins."linear@openai-curated"]"#));
    assert!(
        !tmp.path()
            .join("plugins/cache/openai-curated/linear")
            .exists()
    );
}

#[tokio::test]
async fn sync_plugins_from_remote_keeps_existing_plugins_when_install_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let curated_root = curated_plugins_repo_path(tmp.path());
    write_openai_curated_marketplace(&curated_root, &["linear", "gmail"]);
    write_curated_plugin_sha(tmp.path(), TEST_CURATED_PLUGIN_SHA);
    fs::remove_dir_all(curated_root.join("plugins/gmail")).unwrap();
    write_plugin(
        &tmp.path().join("plugins/cache/openai-curated"),
        "linear/local",
        "linear",
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."linear@openai-curated"]
enabled = false
"#,
    );

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/backend-api/plugins/list"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[
  {"id":"1","name":"gmail","marketplace_name":"openai-curated","version":"1.0.0","enabled":true}
]"#,
        ))
        .mount(&server)
        .await;

    let mut config = load_config(tmp.path(), tmp.path()).await;
    config.chatgpt_base_url = format!("{}/backend-api/", server.uri());
    let manager = PluginsManager::new(tmp.path().to_path_buf());
    let err = manager
        .sync_plugins_from_remote(
            &config,
            Some(&CodexAuth::create_dummy_chatgpt_auth_for_testing()),
            /*additive_only*/ false,
        )
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        PluginRemoteSyncError::Store(PluginStoreError::Invalid(ref message))
            if message.contains("plugin source path is not a directory")
    ));
    assert!(
        tmp.path()
            .join("plugins/cache/openai-curated/linear/local")
            .is_dir()
    );
    assert!(
        !tmp.path()
            .join("plugins/cache/openai-curated/gmail")
            .exists()
    );

    let config = fs::read_to_string(tmp.path().join(CONFIG_TOML_FILE)).unwrap();
    assert!(config.contains(r#"[plugins."linear@openai-curated"]"#));
    assert!(!config.contains(r#"[plugins."gmail@openai-curated"]"#));
    assert!(config.contains("enabled = false"));
}

#[tokio::test]
async fn sync_plugins_from_remote_uses_first_duplicate_local_plugin_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let curated_root = curated_plugins_repo_path(tmp.path());
    write_curated_plugin_sha(tmp.path(), TEST_CURATED_PLUGIN_SHA);
    fs::create_dir_all(curated_root.join(".agents/plugins")).unwrap();
    fs::write(
        curated_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "openai-curated",
  "plugins": [
    {
      "name": "gmail",
      "source": {
        "source": "local",
        "path": "./plugins/gmail-first"
      }
    },
    {
      "name": "gmail",
      "source": {
        "source": "local",
        "path": "./plugins/gmail-second"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_plugin(&curated_root, "plugins/gmail-first", "gmail");
    write_plugin(&curated_root, "plugins/gmail-second", "gmail");
    fs::write(curated_root.join("plugins/gmail-first/marker.txt"), "first").unwrap();
    fs::write(
        curated_root.join("plugins/gmail-second/marker.txt"),
        "second",
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
"#,
    );

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/backend-api/plugins/list"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[
  {"id":"1","name":"gmail","marketplace_name":"openai-curated","version":"1.0.0","enabled":true}
]"#,
        ))
        .mount(&server)
        .await;

    let mut config = load_config(tmp.path(), tmp.path()).await;
    config.chatgpt_base_url = format!("{}/backend-api/", server.uri());
    let manager = PluginsManager::new(tmp.path().to_path_buf());
    let result = manager
        .sync_plugins_from_remote(
            &config,
            Some(&CodexAuth::create_dummy_chatgpt_auth_for_testing()),
            /*additive_only*/ false,
        )
        .await
        .unwrap();

    assert_eq!(
        result,
        RemotePluginSyncResult {
            installed_plugin_ids: vec!["gmail@openai-curated".to_string()],
            enabled_plugin_ids: vec!["gmail@openai-curated".to_string()],
            disabled_plugin_ids: Vec::new(),
            uninstalled_plugin_ids: Vec::new(),
        }
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join(format!(
            "plugins/cache/openai-curated/gmail/{TEST_CURATED_PLUGIN_CACHE_VERSION}/marker.txt"
        )))
        .unwrap(),
        "first"
    );
}
