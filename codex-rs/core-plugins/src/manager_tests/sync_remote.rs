use super::super::*;
use super::*;

#[tokio::test]
async fn sync_plugins_from_remote_returns_default_when_feature_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = false
"#,
    );

    let config = load_config(tmp.path(), tmp.path()).await;
    let outcome = PluginsManager::new(tmp.path().to_path_buf())
        .sync_plugins_from_remote(&config, /*auth*/ None, /*additive_only*/ false)
        .await
        .unwrap();

    assert_eq!(outcome, RemotePluginSyncResult::default());
}

#[tokio::test]
async fn sync_plugins_from_remote_reconciles_cache_and_config() {
    let tmp = tempfile::tempdir().unwrap();
    let curated_root = curated_plugins_repo_path(tmp.path());
    write_openai_curated_marketplace(&curated_root, &["linear", "gmail", "calendar"]);
    write_curated_plugin_sha(tmp.path(), TEST_CURATED_PLUGIN_SHA);
    write_plugin(
        &tmp.path().join("plugins/cache/openai-curated"),
        "linear/local",
        "linear",
    );
    write_plugin(
        &tmp.path().join("plugins/cache/openai-curated"),
        "gmail/local",
        "gmail",
    );
    write_plugin(
        &tmp.path().join("plugins/cache/openai-curated"),
        "calendar/local",
        "calendar",
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."linear@openai-curated"]
enabled = false

[plugins."gmail@openai-curated"]
enabled = false

[plugins."calendar@openai-curated"]
enabled = true
"#,
    );

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/backend-api/plugins/list"))
        .and(header("authorization", "Bearer Access Token"))
        .and(header("chatgpt-account-id", "account_id"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[
  {"id":"1","name":"linear","marketplace_name":"openai-curated","version":"1.0.0","enabled":true},
  {"id":"2","name":"gmail","marketplace_name":"openai-curated","version":"1.0.0","enabled":false}
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
            enabled_plugin_ids: vec!["linear@openai-curated".to_string()],
            disabled_plugin_ids: Vec::new(),
            uninstalled_plugin_ids: vec![
                "gmail@openai-curated".to_string(),
                "calendar@openai-curated".to_string(),
            ],
        }
    );

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
    assert!(
        !tmp.path()
            .join("plugins/cache/openai-curated/calendar")
            .exists()
    );

    let config = fs::read_to_string(tmp.path().join(CONFIG_TOML_FILE)).unwrap();
    assert!(config.contains(r#"[plugins."linear@openai-curated"]"#));
    assert!(config.contains("enabled = true"));
    assert!(!config.contains(r#"[plugins."gmail@openai-curated"]"#));
    assert!(!config.contains(r#"[plugins."calendar@openai-curated"]"#));

    let synced_config = load_config(tmp.path(), tmp.path()).await;
    let curated_marketplace = manager
        .list_marketplaces_for_config(&synced_config, &[])
        .unwrap()
        .marketplaces
        .into_iter()
        .find(|marketplace| marketplace.name == OPENAI_CURATED_MARKETPLACE_NAME)
        .unwrap();
    assert_eq!(
        curated_marketplace
            .plugins
            .into_iter()
            .map(|plugin| (plugin.id, plugin.installed, plugin.enabled))
            .collect::<Vec<_>>(),
        vec![
            ("linear@openai-curated".to_string(), true, true),
            ("gmail@openai-curated".to_string(), false, false),
            ("calendar@openai-curated".to_string(), false, false),
        ]
    );
}

#[tokio::test]
async fn sync_plugins_from_remote_additive_only_keeps_existing_plugins() {
    let tmp = tempfile::tempdir().unwrap();
    let curated_root = curated_plugins_repo_path(tmp.path());
    write_openai_curated_marketplace(&curated_root, &["linear", "gmail", "calendar"]);
    write_curated_plugin_sha(tmp.path(), TEST_CURATED_PLUGIN_SHA);
    write_plugin(
        &tmp.path().join("plugins/cache/openai-curated"),
        "linear/local",
        "linear",
    );
    write_plugin(
        &tmp.path().join("plugins/cache/openai-curated"),
        "gmail/local",
        "gmail",
    );
    write_plugin(
        &tmp.path().join("plugins/cache/openai-curated"),
        "calendar/local",
        "calendar",
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."linear@openai-curated"]
enabled = false

[plugins."gmail@openai-curated"]
enabled = false

[plugins."calendar@openai-curated"]
enabled = true
"#,
    );

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/backend-api/plugins/list"))
        .and(header("authorization", "Bearer Access Token"))
        .and(header("chatgpt-account-id", "account_id"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[
  {"id":"1","name":"linear","marketplace_name":"openai-curated","version":"1.0.0","enabled":true},
  {"id":"2","name":"gmail","marketplace_name":"openai-curated","version":"1.0.0","enabled":false}
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
            /*additive_only*/ true,
        )
        .await
        .unwrap();

    assert_eq!(
        result,
        RemotePluginSyncResult {
            installed_plugin_ids: Vec::new(),
            enabled_plugin_ids: vec!["linear@openai-curated".to_string()],
            disabled_plugin_ids: Vec::new(),
            uninstalled_plugin_ids: Vec::new(),
        }
    );

    assert!(
        tmp.path()
            .join("plugins/cache/openai-curated/linear/local")
            .is_dir()
    );
    assert!(
        tmp.path()
            .join("plugins/cache/openai-curated/gmail/local")
            .is_dir()
    );
    assert!(
        tmp.path()
            .join("plugins/cache/openai-curated/calendar/local")
            .is_dir()
    );

    let config = fs::read_to_string(tmp.path().join(CONFIG_TOML_FILE)).unwrap();
    assert!(config.contains(r#"[plugins."linear@openai-curated"]"#));
    assert!(config.contains(r#"[plugins."gmail@openai-curated"]"#));
    assert!(config.contains(r#"[plugins."calendar@openai-curated"]"#));
    assert!(config.contains("enabled = true"));
}
