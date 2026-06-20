use super::*;
use pretty_assertions::assert_eq;

#[test]
fn skills_list_params_serialization_uses_force_reload() {
    assert_eq!(
        serde_json::to_value(SkillsListParams {
            cwds: Vec::new(),
            force_reload: false,
        })
        .unwrap(),
        json!({}),
    );

    assert_eq!(
        serde_json::to_value(SkillsListParams {
            cwds: vec![PathBuf::from("/repo")],
            force_reload: true,
        })
        .unwrap(),
        json!({
            "cwds": ["/repo"],
            "forceReload": true,
        }),
    );
}

#[test]
fn plugin_source_serializes_local_git_and_remote_variants() {
    let local_path = if cfg!(windows) {
        r"C:\plugins\linear"
    } else {
        "/plugins/linear"
    };
    let local_path = AbsolutePathBuf::try_from(PathBuf::from(local_path)).unwrap();
    let local_path_json = local_path.as_path().display().to_string();

    assert_eq!(
        serde_json::to_value(PluginSource::Local { path: local_path }).unwrap(),
        json!({
            "type": "local",
            "path": local_path_json,
        }),
    );

    assert_eq!(
        serde_json::to_value(PluginSource::Git {
            url: "https://github.com/openai/example.git".to_string(),
            path: Some("plugins/example".to_string()),
            ref_name: Some("main".to_string()),
            sha: Some("abc123".to_string()),
        })
        .unwrap(),
        json!({
            "type": "git",
            "url": "https://github.com/openai/example.git",
            "path": "plugins/example",
            "refName": "main",
            "sha": "abc123",
        }),
    );

    assert_eq!(
        serde_json::to_value(PluginSource::Remote).unwrap(),
        json!({
            "type": "remote",
        }),
    );
}

#[test]
fn marketplace_add_params_serialization_uses_optional_ref_name_and_sparse_paths() {
    assert_eq!(
        serde_json::to_value(MarketplaceAddParams {
            source: "owner/repo".to_string(),
            ref_name: None,
            sparse_paths: None,
        })
        .unwrap(),
        json!({
            "source": "owner/repo",
            "refName": null,
            "sparsePaths": null,
        }),
    );

    assert_eq!(
        serde_json::to_value(MarketplaceAddParams {
            source: "owner/repo".to_string(),
            ref_name: Some("main".to_string()),
            sparse_paths: Some(vec!["plugins/foo".to_string()]),
        })
        .unwrap(),
        json!({
            "source": "owner/repo",
            "refName": "main",
            "sparsePaths": ["plugins/foo"],
        }),
    );
}

#[test]
fn marketplace_upgrade_params_serialization_uses_optional_marketplace_name() {
    assert_eq!(
        serde_json::to_value(MarketplaceUpgradeParams {
            marketplace_name: None,
        })
        .unwrap(),
        json!({
            "marketplaceName": null,
        }),
    );

    assert_eq!(
        serde_json::from_value::<MarketplaceUpgradeParams>(json!({})).unwrap(),
        MarketplaceUpgradeParams {
            marketplace_name: None,
        },
    );

    assert_eq!(
        serde_json::to_value(MarketplaceUpgradeParams {
            marketplace_name: Some("debug".to_string()),
        })
        .unwrap(),
        json!({
            "marketplaceName": "debug",
        }),
    );
}

#[test]
fn plugin_marketplace_entry_serializes_remote_only_path_as_null() {
    assert_eq!(
        serde_json::to_value(PluginMarketplaceEntry {
            name: "openai-curated-remote".to_string(),
            path: None,
            interface: None,
            plugins: Vec::new(),
        })
        .unwrap(),
        json!({
            "name": "openai-curated-remote",
            "path": null,
            "interface": null,
            "plugins": [],
        }),
    );
}

#[test]
fn plugin_interface_serializes_local_paths_and_remote_urls_separately() {
    let composer_icon = if cfg!(windows) {
        r"C:\plugins\linear\icon.png"
    } else {
        "/plugins/linear/icon.png"
    };
    let composer_icon = AbsolutePathBuf::try_from(PathBuf::from(composer_icon)).unwrap();
    let composer_icon_json = composer_icon.as_path().display().to_string();

    let interface = PluginInterface {
        display_name: Some("Linear".to_string()),
        short_description: None,
        long_description: None,
        developer_name: None,
        category: Some("Productivity".to_string()),
        capabilities: Vec::new(),
        website_url: None,
        privacy_policy_url: None,
        terms_of_service_url: None,
        default_prompt: None,
        brand_color: None,
        composer_icon: Some(composer_icon),
        composer_icon_url: Some("https://example.com/linear/icon.png".to_string()),
        logo: None,
        logo_url: Some("https://example.com/linear/logo.png".to_string()),
        screenshots: Vec::new(),
        screenshot_urls: vec!["https://example.com/linear/screenshot.png".to_string()],
    };

    assert_eq!(
        serde_json::to_value(interface).unwrap(),
        json!({
            "displayName": "Linear",
            "shortDescription": null,
            "longDescription": null,
            "developerName": null,
            "category": "Productivity",
            "capabilities": [],
            "websiteUrl": null,
            "privacyPolicyUrl": null,
            "termsOfServiceUrl": null,
            "defaultPrompt": null,
            "brandColor": null,
            "composerIcon": composer_icon_json,
            "composerIconUrl": "https://example.com/linear/icon.png",
            "logo": null,
            "logoUrl": "https://example.com/linear/logo.png",
            "screenshots": [],
            "screenshotUrls": ["https://example.com/linear/screenshot.png"],
        }),
    );
}

#[test]
fn plugin_list_params_ignore_removed_force_remote_sync_field() {
    assert_eq!(
        serde_json::from_value::<PluginListParams>(json!({
            "cwds": null,
            "forceRemoteSync": true,
        }))
        .unwrap(),
        PluginListParams {
            cwds: None,
            marketplace_kinds: None,
        },
    );
}

#[test]
fn plugin_list_params_serializes_marketplace_kind_filter() {
    assert_eq!(
        serde_json::to_value(PluginListParams {
            cwds: None,
            marketplace_kinds: Some(vec![
                PluginListMarketplaceKind::Local,
                PluginListMarketplaceKind::Vertical,
                PluginListMarketplaceKind::WorkspaceDirectory,
                PluginListMarketplaceKind::SharedWithMe,
            ]),
        })
        .unwrap(),
        json!({
            "cwds": null,
            "marketplaceKinds": [
                "local",
                "vertical",
                "workspace-directory",
                "shared-with-me",
            ],
        }),
    );
}

#[test]
fn plugin_installed_params_serializes_install_suggestion_names() {
    assert_eq!(
        serde_json::to_value(PluginInstalledParams {
            cwds: None,
            install_suggestion_plugin_names: Some(vec![
                "computer-use".to_string(),
                "chrome".to_string(),
            ]),
        })
        .unwrap(),
        json!({
            "cwds": null,
            "installSuggestionPluginNames": [
                "computer-use",
                "chrome",
            ],
        }),
    );
}

#[test]
fn plugin_read_params_serialization_uses_install_source_fields() {
    let marketplace_path = if cfg!(windows) {
        r"C:\plugins\marketplace.json"
    } else {
        "/plugins/marketplace.json"
    };
    let marketplace_path = AbsolutePathBuf::try_from(PathBuf::from(marketplace_path)).unwrap();
    let marketplace_path_json = marketplace_path.as_path().display().to_string();
    assert_eq!(
        serde_json::to_value(PluginReadParams {
            marketplace_path: Some(marketplace_path.clone()),
            remote_marketplace_name: None,
            plugin_name: "gmail".to_string(),
        })
        .unwrap(),
        json!({
            "marketplacePath": marketplace_path_json,
            "remoteMarketplaceName": null,
            "pluginName": "gmail",
        }),
    );

    assert_eq!(
        serde_json::from_value::<PluginReadParams>(json!({
            "marketplacePath": marketplace_path_json,
            "pluginName": "gmail",
            "forceRemoteSync": true,
        }))
        .unwrap(),
        PluginReadParams {
            marketplace_path: Some(marketplace_path),
            remote_marketplace_name: None,
            plugin_name: "gmail".to_string(),
        },
    );

    assert_eq!(
        serde_json::from_value::<PluginReadParams>(json!({
            "remoteMarketplaceName": "openai-curated-remote",
            "pluginName": "gmail",
        }))
        .unwrap(),
        PluginReadParams {
            marketplace_path: None,
            remote_marketplace_name: Some("openai-curated-remote".to_string()),
            plugin_name: "gmail".to_string(),
        },
    );
}

#[test]
fn plugin_install_params_serialization_omits_force_remote_sync() {
    let marketplace_path = if cfg!(windows) {
        r"C:\plugins\marketplace.json"
    } else {
        "/plugins/marketplace.json"
    };
    let marketplace_path = AbsolutePathBuf::try_from(PathBuf::from(marketplace_path)).unwrap();
    let marketplace_path_json = marketplace_path.as_path().display().to_string();
    assert_eq!(
        serde_json::to_value(PluginInstallParams {
            marketplace_path: Some(marketplace_path.clone()),
            remote_marketplace_name: None,
            plugin_name: "gmail".to_string(),
        })
        .unwrap(),
        json!({
            "marketplacePath": marketplace_path_json,
            "remoteMarketplaceName": null,
            "pluginName": "gmail",
        }),
    );

    assert_eq!(
        serde_json::from_value::<PluginInstallParams>(json!({
            "marketplacePath": marketplace_path_json,
            "pluginName": "gmail",
            "forceRemoteSync": true,
        }))
        .unwrap(),
        PluginInstallParams {
            marketplace_path: Some(marketplace_path),
            remote_marketplace_name: None,
            plugin_name: "gmail".to_string(),
        },
    );

    assert_eq!(
        serde_json::from_value::<PluginInstallParams>(json!({
            "remoteMarketplaceName": "openai-curated-remote",
            "pluginName": "gmail",
            "forceRemoteSync": true,
        }))
        .unwrap(),
        PluginInstallParams {
            marketplace_path: None,
            remote_marketplace_name: Some("openai-curated-remote".to_string()),
            plugin_name: "gmail".to_string(),
        },
    );
}

#[test]
fn plugin_skill_read_params_serialization_uses_remote_plugin_id() {
    assert_eq!(
        serde_json::to_value(PluginSkillReadParams {
            remote_marketplace_name: "openai-curated-remote".to_string(),
            remote_plugin_id: "plugins~Plugin_00000000000000000000000000000000".to_string(),
            skill_name: "plan-work".to_string(),
        })
        .unwrap(),
        json!({
            "remoteMarketplaceName": "openai-curated-remote",
            "remotePluginId": "plugins~Plugin_00000000000000000000000000000000",
            "skillName": "plan-work",
        }),
    );
}
