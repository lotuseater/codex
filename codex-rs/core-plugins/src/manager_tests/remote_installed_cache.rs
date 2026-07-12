use super::super::*;
use super::*;

#[tokio::test]
async fn remote_installed_cache_ignores_plugins_missing_local_cache() {
    let codex_home = TempDir::new().unwrap();
    write_file(
        &codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
remote_plugin = true
"#,
    );

    let config = load_config(codex_home.path(), codex_home.path()).await;
    let manager = PluginsManager::new(codex_home.path().to_path_buf());
    manager.write_remote_installed_plugins_cache(vec![remote_installed_linear_plugin()]);

    let outcome = manager.plugins_for_config(&config).await;
    assert_eq!(outcome, PluginLoadOutcome::default());
}

#[tokio::test]
async fn build_remote_installed_plugin_marketplaces_from_cache_uses_remote_metadata() {
    let codex_home = TempDir::new().unwrap();
    let manager = PluginsManager::new(codex_home.path().to_path_buf());
    let mut plugin = remote_installed_linear_plugin();
    plugin.install_policy = codex_app_server_protocol::PluginInstallPolicy::InstalledByDefault;
    plugin.auth_policy = codex_app_server_protocol::PluginAuthPolicy::OnInstall;
    plugin.interface = Some(codex_app_server_protocol::PluginInterface {
        display_name: Some("Linear".to_string()),
        short_description: Some("Track remote work".to_string()),
        long_description: None,
        developer_name: None,
        category: None,
        capabilities: Vec::new(),
        website_url: None,
        privacy_policy_url: None,
        terms_of_service_url: None,
        default_prompt: None,
        brand_color: Some("#111111".to_string()),
        composer_icon: None,
        composer_icon_url: None,
        logo: None,
        logo_dark: None,
        logo_url: None,
        logo_url_dark: None,
        screenshots: Vec::new(),
        screenshot_urls: Vec::new(),
    });
    plugin.keywords = vec!["issues".to_string()];
    manager.write_remote_installed_plugins_cache(vec![plugin]);

    let marketplaces = manager
        .build_remote_installed_plugin_marketplaces_from_cache(&[RemotePluginScope::Global])
        .expect("remote installed cache should be present");
    assert_eq!(marketplaces.len(), 1);
    assert_eq!(marketplaces[0].name, "openai-curated-remote");
    assert_eq!(marketplaces[0].display_name, "OpenAI Curated Remote");
    assert_eq!(marketplaces[0].plugins.len(), 1);
    let plugin = &marketplaces[0].plugins[0];
    assert_eq!(plugin.id, "linear@openai-curated-remote");
    assert_eq!(plugin.remote_plugin_id, "plugins~Plugin_linear");
    assert_eq!(plugin.name, "linear");
    assert_eq!(plugin.installed, true);
    assert_eq!(plugin.enabled, true);
    assert_eq!(
        plugin.install_policy,
        codex_app_server_protocol::PluginInstallPolicy::InstalledByDefault
    );
    assert_eq!(
        plugin.auth_policy,
        codex_app_server_protocol::PluginAuthPolicy::OnInstall
    );
    assert_eq!(plugin.keywords, vec!["issues".to_string()]);
    assert_eq!(
        plugin
            .interface
            .as_ref()
            .and_then(|interface| interface.display_name.as_deref()),
        Some("Linear")
    );
    assert_eq!(
        plugin
            .interface
            .as_ref()
            .and_then(|interface| interface.short_description.as_deref()),
        Some("Track remote work")
    );
    assert_eq!(
        manager
            .build_remote_installed_plugin_marketplaces_from_cache(&[RemotePluginScope::Workspace])
            .expect("remote installed cache should be present"),
        Vec::new()
    );
}
