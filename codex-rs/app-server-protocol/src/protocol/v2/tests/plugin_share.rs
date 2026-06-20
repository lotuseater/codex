use super::*;
use pretty_assertions::assert_eq;

#[test]
fn plugin_share_params_and_response_serialization_use_camel_case_fields() {
    let plugin_path = if cfg!(windows) {
        r"C:\plugins\gmail"
    } else {
        "/plugins/gmail"
    };
    let plugin_path = AbsolutePathBuf::try_from(PathBuf::from(plugin_path)).unwrap();
    let plugin_path_json = plugin_path.as_path().display().to_string();

    assert_eq!(
        serde_json::to_value(PluginShareSaveParams {
            plugin_path: plugin_path.clone(),
            remote_plugin_id: None,
            discoverability: None,
            share_targets: None,
        })
        .unwrap(),
        json!({
            "pluginPath": plugin_path_json,
            "remotePluginId": null,
            "discoverability": null,
            "shareTargets": null,
        }),
    );

    assert_eq!(
        serde_json::to_value(PluginShareSaveParams {
            plugin_path,
            remote_plugin_id: Some("plugins~Plugin_00000000000000000000000000000000".to_string(),),
            discoverability: Some(PluginShareDiscoverability::Private),
            share_targets: Some(vec![
                PluginShareTarget {
                    principal_type: PluginSharePrincipalType::User,
                    principal_id: "user-1".to_string(),
                    role: PluginShareTargetRole::Reader,
                },
                PluginShareTarget {
                    principal_type: PluginSharePrincipalType::Group,
                    principal_id: "group-1".to_string(),
                    role: PluginShareTargetRole::Reader,
                },
            ]),
        })
        .unwrap(),
        json!({
            "pluginPath": plugin_path_json,
            "remotePluginId": "plugins~Plugin_00000000000000000000000000000000",
            "discoverability": "PRIVATE",
            "shareTargets": [
                {
                    "principalType": "user",
                    "principalId": "user-1",
                    "role": "reader",
                },
                {
                    "principalType": "group",
                    "principalId": "group-1",
                    "role": "reader",
                },
            ],
        }),
    );

    assert_eq!(
        serde_json::to_value(PluginShareSaveResponse {
            remote_plugin_id: "plugins~Plugin_00000000000000000000000000000000".to_string(),
            share_url: String::new(),
        })
        .unwrap(),
        json!({
            "remotePluginId": "plugins~Plugin_00000000000000000000000000000000",
            "shareUrl": "",
        }),
    );

    assert_eq!(
        serde_json::to_value(PluginShareUpdateTargetsParams {
            remote_plugin_id: "plugins~Plugin_00000000000000000000000000000000".to_string(),
            discoverability: PluginShareUpdateDiscoverability::Unlisted,
            share_targets: vec![PluginShareTarget {
                principal_type: PluginSharePrincipalType::Group,
                principal_id: "group-1".to_string(),
                role: PluginShareTargetRole::Editor,
            }],
        })
        .unwrap(),
        json!({
            "remotePluginId": "plugins~Plugin_00000000000000000000000000000000",
            "discoverability": "UNLISTED",
            "shareTargets": [{
                "principalType": "group",
                "principalId": "group-1",
                "role": "editor",
            }],
        }),
    );

    assert_eq!(
        serde_json::to_value(PluginShareUpdateTargetsResponse {
            principals: vec![PluginSharePrincipal {
                principal_type: PluginSharePrincipalType::User,
                principal_id: "user-1".to_string(),
                role: PluginSharePrincipalRole::Owner,
                name: "Gavin".to_string(),
            }],
            discoverability: PluginShareDiscoverability::Unlisted,
        })
        .unwrap(),
        json!({
            "principals": [{
                "principalType": "user",
                "principalId": "user-1",
                "role": "owner",
                "name": "Gavin",
            }],
            "discoverability": "UNLISTED",
        }),
    );

    assert_eq!(
        serde_json::from_value::<PluginShareListParams>(json!({})).unwrap(),
        PluginShareListParams {},
    );

    assert_eq!(
        serde_json::to_value(PluginShareCheckoutParams {
            remote_plugin_id: "plugins~Plugin_00000000000000000000000000000000".to_string(),
        })
        .unwrap(),
        json!({
            "remotePluginId": "plugins~Plugin_00000000000000000000000000000000",
        }),
    );

    let plugin_path = if cfg!(windows) {
        r"C:\Users\me\plugins\gmail"
    } else {
        "/Users/me/plugins/gmail"
    };
    let plugin_path = AbsolutePathBuf::try_from(PathBuf::from(plugin_path)).unwrap();
    let plugin_path_json = plugin_path.as_path().display().to_string();
    let marketplace_path = if cfg!(windows) {
        r"C:\Users\me\.agents\plugins\marketplace.json"
    } else {
        "/Users/me/.agents/plugins/marketplace.json"
    };
    let marketplace_path = AbsolutePathBuf::try_from(PathBuf::from(marketplace_path)).unwrap();
    let marketplace_path_json = marketplace_path.as_path().display().to_string();
    assert_eq!(
        serde_json::to_value(PluginShareCheckoutResponse {
            remote_plugin_id: "plugins~Plugin_00000000000000000000000000000000".to_string(),
            plugin_id: "gmail@codex-curated".to_string(),
            plugin_name: "gmail".to_string(),
            plugin_path,
            marketplace_name: "codex-curated".to_string(),
            marketplace_path,
            remote_version: Some("1.2.3".to_string()),
        })
        .unwrap(),
        json!({
            "remotePluginId": "plugins~Plugin_00000000000000000000000000000000",
            "pluginId": "gmail@codex-curated",
            "pluginName": "gmail",
            "pluginPath": plugin_path_json,
            "marketplaceName": "codex-curated",
            "marketplacePath": marketplace_path_json,
            "remoteVersion": "1.2.3",
        }),
    );

    assert_eq!(
        serde_json::to_value(PluginShareDeleteParams {
            remote_plugin_id: "plugins~Plugin_00000000000000000000000000000000".to_string(),
        })
        .unwrap(),
        json!({
            "remotePluginId": "plugins~Plugin_00000000000000000000000000000000",
        }),
    );
}

#[test]
fn plugin_share_list_response_serializes_share_items() {
    assert_eq!(
        serde_json::to_value(PluginShareListResponse {
            data: vec![PluginShareListItem {
                plugin: PluginSummary {
                    id: "gmail@openai-curated-remote".to_string(),
                    remote_plugin_id: Some(
                        "plugins~Plugin_00000000000000000000000000000000".to_string(),
                    ),
                    local_version: None,
                    name: "gmail".to_string(),
                    share_context: None,
                    source: PluginSource::Remote,
                    installed: false,
                    enabled: false,
                    install_policy: PluginInstallPolicy::Available,
                    auth_policy: PluginAuthPolicy::OnUse,
                    availability: PluginAvailability::Available,
                    interface: None,
                    keywords: Vec::new(),
                },
                local_plugin_path: None,
            }],
        })
        .unwrap(),
        json!({
            "data": [{
                "plugin": {
                    "id": "gmail@openai-curated-remote",
                    "remotePluginId": "plugins~Plugin_00000000000000000000000000000000",
                    "localVersion": null,
                    "name": "gmail",
                    "shareContext": null,
                    "source": { "type": "remote" },
                    "installed": false,
                    "enabled": false,
                    "installPolicy": "AVAILABLE",
                    "authPolicy": "ON_USE",
                    "availability": "AVAILABLE",
                    "interface": null,
                    "keywords": [],
                },
                "localPluginPath": null,
            }],
        }),
    );
}

#[test]
fn plugin_summary_defaults_missing_availability_to_available() {
    let summary: PluginSummary = serde_json::from_value(json!({
        "id": "plugins~Plugin_00000000000000000000000000000000",
        "name": "gmail",
        "source": { "type": "remote" },
        "installed": false,
        "enabled": false,
        "installPolicy": "AVAILABLE",
        "authPolicy": "ON_USE",
        "interface": null,
    }))
    .unwrap();

    assert_eq!(summary.availability, PluginAvailability::Available);
    assert_eq!(summary.local_version, None);
    assert_eq!(summary.share_context, None);
}

#[test]
fn plugin_availability_deserializes_enabled_alias() {
    let availability: PluginAvailability = serde_json::from_value(json!("ENABLED")).unwrap();

    assert_eq!(availability, PluginAvailability::Available);
    assert_eq!(
        serde_json::to_value(availability).unwrap(),
        json!("AVAILABLE")
    );
}

#[test]
fn plugin_uninstall_params_serialization_omits_force_remote_sync() {
    assert_eq!(
        serde_json::to_value(PluginUninstallParams {
            plugin_id: "gmail@openai-curated".to_string(),
        })
        .unwrap(),
        json!({
            "pluginId": "gmail@openai-curated",
        }),
    );

    assert_eq!(
        serde_json::from_value::<PluginUninstallParams>(json!({
            "pluginId": "gmail@openai-curated",
            "forceRemoteSync": true,
        }))
        .unwrap(),
        PluginUninstallParams {
            plugin_id: "gmail@openai-curated".to_string(),
        },
    );

    assert_eq!(
        serde_json::to_value(PluginUninstallParams {
            plugin_id: "plugins~Plugin_gmail".to_string(),
        })
        .unwrap(),
        json!({
            "pluginId": "plugins~Plugin_gmail",
        }),
    );

    assert_eq!(
        serde_json::from_value::<PluginUninstallParams>(json!({
            "pluginId": "plugins~Plugin_gmail",
            "forceRemoteSync": true,
        }))
        .unwrap(),
        PluginUninstallParams {
            plugin_id: "plugins~Plugin_gmail".to_string(),
        },
    );
}

#[test]
fn marketplace_remove_response_serializes_nullable_installed_root() {
    let installed_root = if cfg!(windows) {
        r"C:\marketplaces\debug"
    } else {
        "/tmp/marketplaces/debug"
    };
    let installed_root = AbsolutePathBuf::try_from(PathBuf::from(installed_root)).unwrap();
    let installed_root_json = installed_root.as_path().display().to_string();
    assert_eq!(
        serde_json::to_value(MarketplaceRemoveResponse {
            marketplace_name: "debug".to_string(),
            installed_root: Some(installed_root),
        })
        .unwrap(),
        json!({
            "marketplaceName": "debug",
            "installedRoot": installed_root_json,
        }),
    );

    assert_eq!(
        serde_json::to_value(MarketplaceRemoveResponse {
            marketplace_name: "debug".to_string(),
            installed_root: None,
        })
        .unwrap(),
        json!({
            "marketplaceName": "debug",
            "installedRoot": null,
        }),
    );
}

#[test]
fn marketplace_upgrade_response_serializes_camel_case_fields() {
    let upgraded_root = if cfg!(windows) {
        r"C:\marketplaces\debug"
    } else {
        "/tmp/marketplaces/debug"
    };
    let upgraded_root = AbsolutePathBuf::try_from(PathBuf::from(upgraded_root)).unwrap();
    let upgraded_root_json = upgraded_root.as_path().display().to_string();

    assert_eq!(
        serde_json::to_value(MarketplaceUpgradeResponse {
            selected_marketplaces: vec!["debug".to_string()],
            upgraded_roots: vec![upgraded_root],
            errors: vec![MarketplaceUpgradeErrorInfo {
                marketplace_name: "broken".to_string(),
                message: "failed to clone".to_string(),
            }],
        })
        .unwrap(),
        json!({
            "selectedMarketplaces": ["debug"],
            "upgradedRoots": [upgraded_root_json],
            "errors": [{
                "marketplaceName": "broken",
                "message": "failed to clone",
            }],
        }),
    );
}
