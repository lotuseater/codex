use super::*;

#[test]
fn desktop_toml_round_trips_opaque_nested_values() -> anyhow::Result<()> {
    let parsed = toml::from_str::<ConfigToml>(
        r#"
[desktop]
appearanceTheme = "dark"
selected-avatar-id = "codex"
recentViews = ["threads", "settings"]

[desktop.workspace]
collapsed = true
width = 320
pane = { selected = "console", expanded = false }
"#,
    )?;

    let desktop = parsed
        .desktop
        .as_ref()
        .expect("desktop settings should deserialize");
    assert_eq!(
        desktop.get("appearanceTheme"),
        Some(&serde_json::json!("dark"))
    );
    assert_eq!(
        desktop.get("selected-avatar-id"),
        Some(&serde_json::json!("codex"))
    );
    assert_eq!(
        desktop.get("recentViews"),
        Some(&serde_json::json!(["threads", "settings"]))
    );
    assert_eq!(
        desktop.get("workspace"),
        Some(&serde_json::json!({
            "collapsed": true,
            "width": 320,
            "pane": {
                "selected": "console",
                "expanded": false,
            },
        }))
    );

    let serialized = toml::to_string(&parsed)?;
    let reparsed = toml::from_str::<ConfigToml>(&serialized)?;
    assert_eq!(reparsed.desktop, parsed.desktop);

    Ok(())
}

#[tokio::test]
async fn to_mcp_config_preserves_apps_feature_from_config() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let mut config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;
    let plugins_manager = PluginsManager::new(codex_home.path().to_path_buf());

    config.apps_mcp_path_override = Some("/custom/mcp".to_string());
    config.apps_mcp_product_sku = Some("tpp".to_string());
    let mcp_config = config.to_mcp_config(&plugins_manager).await;
    assert!(mcp_config.apps_enabled);
    assert_eq!(
        mcp_config.apps_mcp_path_override.as_deref(),
        Some("/custom/mcp")
    );
    assert_eq!(mcp_config.apps_mcp_product_sku.as_deref(), Some("tpp"));

    let _ = config.features.disable(Feature::Apps);
    let mcp_config = config.to_mcp_config(&plugins_manager).await;
    assert!(!mcp_config.apps_enabled);

    let _ = config.features.enable(Feature::Apps);
    let mcp_config = config.to_mcp_config(&plugins_manager).await;
    assert!(mcp_config.apps_enabled);

    Ok(())
}

#[tokio::test]
async fn to_mcp_config_preserves_auth_elicitation_feature_from_config() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let mut config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;
    let plugins_manager = PluginsManager::new(codex_home.path().to_path_buf());

    let mcp_config = config.to_mcp_config(&plugins_manager).await;
    assert_eq!(
        mcp_config.client_elicitation_capability,
        ElicitationCapability::default()
    );

    let _ = config.features.enable(Feature::AuthElicitation);
    let mcp_config = config.to_mcp_config(&plugins_manager).await;
    assert_eq!(
        mcp_config.client_elicitation_capability,
        ElicitationCapability {
            form: Some(FormElicitationCapability::default()),
            url: Some(UrlElicitationCapability::default()),
        }
    );

    Ok(())
}

#[tokio::test]
async fn load_global_mcp_servers_rejects_inline_bearer_token() -> anyhow::Result<()> {
    let codex_home = TempDir::new()?;
    let config_path = codex_home.path().join(CONFIG_TOML_FILE);

    std::fs::write(
        &config_path,
        r#"
[mcp_servers.docs]
url = "https://example.com/mcp"
bearer_token = "secret"
"#,
    )?;

    let err = load_global_mcp_servers(codex_home.path())
        .await
        .expect_err("bearer_token entries should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("bearer_token"));
    assert!(err.to_string().contains("bearer_token_env_var"));

    Ok(())
}

