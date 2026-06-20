use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn permissions_profiles_proxy_policy_does_not_start_managed_network_proxy_without_feature()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    std::fs::write(cwd.path().join(".git"), "gitdir: nowhere")?;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some("dev".to_string()),
            permissions: Some(PermissionsToml {
                entries: BTreeMap::from([(
                    "dev".to_string(),
                    PermissionProfileToml {
                        description: None,
                        workspace_roots: None,
                        filesystem: Some(FilesystemPermissionsToml {
                            glob_scan_max_depth: None,
                            entries: BTreeMap::from([(
                                ":minimal".to_string(),
                                FilesystemPermissionToml::Access(FileSystemAccessMode::Read),
                            )]),
                        }),
                        network: Some(NetworkToml {
                            enabled: Some(true),
                            ..Default::default()
                        }),
                    },
                )]),
            }),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;
    assert_eq!(
        config.permissions.network_sandbox_policy(),
        NetworkSandboxPolicy::Enabled
    );
    assert!(
        config.permissions.network.is_none(),
        "bare profile network.enabled should not start the managed network proxy"
    );
    Ok(())
}

#[tokio::test]
async fn permissions_profiles_proxy_policy_starts_managed_network_proxy() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    std::fs::write(cwd.path().join(".git"), "gitdir: nowhere")?;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some("dev".to_string()),
            permissions: Some(PermissionsToml {
                entries: BTreeMap::from([(
                    "dev".to_string(),
                    PermissionProfileToml {
                        description: None,
                        extends: None,
                        workspace_roots: None,
                        filesystem: Some(FilesystemPermissionsToml {
                            glob_scan_max_depth: None,
                            entries: BTreeMap::from([(
                                ":minimal".to_string(),
                                FilesystemPermissionToml::Access(FileSystemAccessMode::Read),
                            )]),
                        }),
                        network: Some(NetworkToml {
                            enabled: Some(true),
                            proxy_url: Some("http://127.0.0.1:43128".to_string()),
                            enable_socks5: Some(false),
                            ..Default::default()
                        }),
                    },
                )]),
            }),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;
    assert_eq!(
        config.permissions.network_sandbox_policy(),
        NetworkSandboxPolicy::Enabled
    );
    assert!(
        config.permissions.network.is_none(),
        "profile proxy policy should not start the managed network proxy without the feature"
    );
    Ok(())
}

#[tokio::test]
async fn network_proxy_feature_is_no_op_without_sandbox_network() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            features: Some(toml::from_str("network_proxy = true").expect("valid features")),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        config.permissions.network_sandbox_policy(),
        NetworkSandboxPolicy::Restricted
    );
    assert!(
        config.permissions.network.is_none(),
        "network_proxy should not start the managed network proxy while network access is off"
    );
    Ok(())
}

#[tokio::test]
async fn network_proxy_feature_matrix_preserves_sandbox_network_semantics() -> std::io::Result<()> {
    #[derive(Clone, Copy)]
    enum Surface {
        PermissionProfile,
        LegacyWorkspaceWrite,
    }

    struct Case {
        name: &'static str,
        surface: Surface,
        network_enabled: bool,
        proxy_enabled: bool,
        expected_network_policy: NetworkSandboxPolicy,
    }

    let cases = [
        Case {
            name: "permission profile network disabled without proxy",
            surface: Surface::PermissionProfile,
            network_enabled: false,
            proxy_enabled: false,
            expected_network_policy: NetworkSandboxPolicy::Restricted,
        },
        Case {
            name: "permission profile network disabled with proxy",
            surface: Surface::PermissionProfile,
            network_enabled: false,
            proxy_enabled: true,
            expected_network_policy: NetworkSandboxPolicy::Restricted,
        },
        Case {
            name: "permission profile network enabled without proxy",
            surface: Surface::PermissionProfile,
            network_enabled: true,
            proxy_enabled: false,
            expected_network_policy: NetworkSandboxPolicy::Enabled,
        },
        Case {
            name: "permission profile network enabled with proxy",
            surface: Surface::PermissionProfile,
            network_enabled: true,
            proxy_enabled: true,
            expected_network_policy: NetworkSandboxPolicy::Enabled,
        },
        Case {
            name: "legacy workspace write network disabled without proxy",
            surface: Surface::LegacyWorkspaceWrite,
            network_enabled: false,
            proxy_enabled: false,
            expected_network_policy: NetworkSandboxPolicy::Restricted,
        },
        Case {
            name: "legacy workspace write network disabled with proxy",
            surface: Surface::LegacyWorkspaceWrite,
            network_enabled: false,
            proxy_enabled: true,
            expected_network_policy: NetworkSandboxPolicy::Restricted,
        },
        Case {
            name: "legacy workspace write network enabled without proxy",
            surface: Surface::LegacyWorkspaceWrite,
            network_enabled: true,
            proxy_enabled: false,
            expected_network_policy: NetworkSandboxPolicy::Enabled,
        },
        Case {
            name: "legacy workspace write network enabled with proxy",
            surface: Surface::LegacyWorkspaceWrite,
            network_enabled: true,
            proxy_enabled: true,
            expected_network_policy: NetworkSandboxPolicy::Enabled,
        },
    ];

    for case in cases {
        let codex_home = TempDir::new()?;
        let cwd = TempDir::new()?;
        std::fs::write(cwd.path().join(".git"), "gitdir: nowhere")?;
        let features = case
            .proxy_enabled
            .then(|| toml::from_str("network_proxy = true").expect("valid features"));
        let base_config = match case.surface {
            Surface::PermissionProfile => ConfigToml {
                default_permissions: Some("dev".to_string()),
                permissions: Some(PermissionsToml {
                    entries: BTreeMap::from([(
                        "dev".to_string(),
                        PermissionProfileToml {
                            description: None,
                            extends: None,
                            workspace_roots: None,
                            filesystem: Some(FilesystemPermissionsToml {
                                glob_scan_max_depth: None,
                                entries: BTreeMap::from([(
                                    ":minimal".to_string(),
                                    FilesystemPermissionToml::Access(FileSystemAccessMode::Read),
                                )]),
                            }),
                            network: Some(NetworkToml {
                                enabled: Some(case.network_enabled),
                                ..Default::default()
                            }),
                        },
                    )]),
                }),
                features,
                ..Default::default()
            },
            Surface::LegacyWorkspaceWrite => ConfigToml {
                sandbox_mode: Some(SandboxMode::WorkspaceWrite),
                sandbox_workspace_write: Some(SandboxWorkspaceWrite {
                    network_access: case.network_enabled,
                    ..Default::default()
                }),
                windows: Some(WindowsToml {
                    sandbox: Some(WindowsSandboxModeToml::Elevated),
                    sandbox_private_desktop: None,
                }),
                features,
                ..Default::default()
            },
        };
        let config = Config::load_from_base_config_with_overrides(
            base_config,
            ConfigOverrides {
                cwd: Some(cwd.path().to_path_buf()),
                ..Default::default()
            },
            codex_home.abs(),
        )
        .await?;

        assert_eq!(
            config.permissions.network_sandbox_policy(),
            case.expected_network_policy,
            "{}",
            case.name
        );
        assert_eq!(
            config.permissions.network.is_some(),
            case.network_enabled && case.proxy_enabled,
            "{}",
            case.name
        );
    }

    Ok(())
}

#[tokio::test]
async fn network_proxy_cli_overrides_merge_toggle_with_proxy_config() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"
sandbox_mode = "workspace-write"

[sandbox_workspace_write]
network_access = true

[windows]
sandbox = "elevated"
"#,
    )?;
    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .cli_overrides(vec![
            (
                "features.network_proxy.enabled".to_string(),
                toml::Value::Boolean(true),
            ),
            (
                "features.network_proxy.enable_socks5".to_string(),
                toml::Value::Boolean(false),
            ),
        ])
        .harness_overrides(ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        })
        .build()
        .await?;

    assert_eq!(
        config.permissions.network_sandbox_policy(),
        NetworkSandboxPolicy::Enabled
    );
    let network = config
        .permissions
        .network
        .as_ref()
        .expect("network_proxy should start the managed network proxy");
    assert_eq!(network.proxy_host_and_port(), "127.0.0.1:3128");
    assert!(!network.socks_enabled());
    Ok(())
}

#[tokio::test]
async fn experimental_network_requirements_enable_proxy_without_feature() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .cloud_requirements(CloudRequirementsLoader::new(async {
            Ok(Some(codex_config::ConfigRequirementsToml {
                network: Some(codex_config::NetworkRequirementsToml {
                    enabled: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }))
        }))
        .build()
        .await?;

    assert!(!config.features.enabled(Feature::NetworkProxy));
    assert!(config.managed_network_requirements_enabled());
    assert!(
        config
            .permissions
            .network
            .as_ref()
            .expect("experimental_network should configure the managed proxy")
            .enabled()
    );
    Ok(())
}

#[tokio::test]
async fn network_proxy_feature_uses_profile_network_proxy_settings() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            features: Some(toml::from_str("network_proxy = true").expect("valid features")),
            default_permissions: Some("dev".to_string()),
            permissions: Some(PermissionsToml {
                entries: BTreeMap::from([(
                    "dev".to_string(),
                    PermissionProfileToml {
                        description: None,
                        extends: None,
                        workspace_roots: None,
                        filesystem: Some(FilesystemPermissionsToml {
                            glob_scan_max_depth: None,
                            entries: BTreeMap::from([(
                                ":minimal".to_string(),
                                FilesystemPermissionToml::Access(FileSystemAccessMode::Read),
                            )]),
                        }),
                        network: Some(NetworkToml {
                            enabled: Some(true),
                            proxy_url: Some("http://127.0.0.1:43128".to_string()),
                            enable_socks5: Some(false),
                            ..Default::default()
                        }),
                    },
                )]),
            }),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        config.permissions.network_sandbox_policy(),
        NetworkSandboxPolicy::Enabled
    );
    let network = config
        .permissions
        .network
        .as_ref()
        .expect("network_proxy should start the managed network proxy");
    assert_eq!(network.proxy_host_and_port(), "127.0.0.1:43128");
    assert!(!network.socks_enabled());
    Ok(())
}

#[tokio::test]
async fn disabled_network_proxy_feature_does_not_start_profile_proxy_policy() -> std::io::Result<()>
{
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            features: Some(
                toml::from_str(
                    r#"
[network_proxy]
enabled = false
"#,
                )
                .expect("valid features"),
            ),
            default_permissions: Some("dev".to_string()),
            permissions: Some(PermissionsToml {
                entries: BTreeMap::from([(
                    "dev".to_string(),
                    PermissionProfileToml {
                        description: None,
                        extends: None,
                        workspace_roots: None,
                        filesystem: Some(FilesystemPermissionsToml {
                            glob_scan_max_depth: None,
                            entries: BTreeMap::from([(
                                ":minimal".to_string(),
                                FilesystemPermissionToml::Access(FileSystemAccessMode::Read),
                            )]),
                        }),
                        network: Some(NetworkToml {
                            enabled: Some(true),
                            proxy_url: Some("http://127.0.0.1:43128".to_string()),
                            enable_socks5: Some(false),
                            ..Default::default()
                        }),
                    },
                )]),
            }),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert!(!config.features.enabled(Feature::NetworkProxy));
    assert!(
        config.permissions.network.is_none(),
        "disabled feature should keep profile proxy policy from starting the managed proxy"
    );
    Ok(())
}

#[tokio::test]
async fn permissions_profiles_network_disabled_by_default_does_not_start_proxy()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    std::fs::write(cwd.path().join(".git"), "gitdir: nowhere")?;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some("dev".to_string()),
            permissions: Some(PermissionsToml {
                entries: BTreeMap::from([(
                    "dev".to_string(),
                    PermissionProfileToml {
                        description: None,
                        extends: None,
                        workspace_roots: None,
                        filesystem: Some(FilesystemPermissionsToml {
                            glob_scan_max_depth: None,
                            entries: BTreeMap::from([(
                                ":minimal".to_string(),
                                FilesystemPermissionToml::Access(FileSystemAccessMode::Read),
                            )]),
                        }),
                        network: Some(NetworkToml {
                            domains: Some(NetworkDomainPermissionsToml {
                                entries: BTreeMap::from([(
                                    "openai.com".to_string(),
                                    NetworkDomainPermissionToml::Allow,
                                )]),
                            }),
                            ..Default::default()
                        }),
                    },
                )]),
            }),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert!(config.permissions.network.is_none());
    Ok(())
}
