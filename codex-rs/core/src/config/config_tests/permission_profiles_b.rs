use super::*;

#[tokio::test]
async fn default_permissions_profile_populates_runtime_sandbox_policy() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    std::fs::create_dir_all(cwd.path().join("docs"))?;
    std::fs::write(cwd.path().join(".git"), "gitdir: nowhere")?;

    let cfg = ConfigToml {
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
                        entries: BTreeMap::from([
                            (
                                ":minimal".to_string(),
                                FilesystemPermissionToml::Access(FileSystemAccessMode::Read),
                            ),
                            (
                                ":workspace_roots".to_string(),
                                FilesystemPermissionToml::Scoped(BTreeMap::from([
                                    (".".to_string(), FileSystemAccessMode::Write),
                                    ("docs".to_string(), FileSystemAccessMode::Read),
                                ])),
                            ),
                        ]),
                    }),
                    network: None,
                },
            )]),
        }),
        ..Default::default()
    };

    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    let cwd_root = cwd.path().abs();
    let memories_root = codex_home.path().join("memories").abs();
    assert_eq!(
        config.permissions.file_system_sandbox_policy(),
        FileSystemSandboxPolicy::restricted(vec![
            FileSystemSandboxEntry {
                path: FileSystemPath::Special {
                    value: FileSystemSpecialPath::Minimal,
                },
                access: FileSystemAccessMode::Read,
            },
            FileSystemSandboxEntry {
                path: FileSystemPath::Path {
                    path: cwd_root.clone(),
                },
                access: FileSystemAccessMode::Write,
            },
            FileSystemSandboxEntry {
                path: FileSystemPath::Path {
                    path: cwd_root.join("docs"),
                },
                access: FileSystemAccessMode::Read,
            },
            FileSystemSandboxEntry {
                path: FileSystemPath::Path {
                    path: memories_root.clone(),
                },
                access: FileSystemAccessMode::Write,
            },
        ]),
    );
    assert_eq!(
        &config.legacy_sandbox_policy(),
        &SandboxPolicy::WorkspaceWrite {
            writable_roots: vec![memories_root],
            network_access: false,
            exclude_tmpdir_env_var: true,
            exclude_slash_tmp: true,
        }
    );
    assert!(
        !config
            .permissions
            .file_system_sandbox_policy()
            .can_write_path_with_cwd(&cwd.path().join(".git"), cwd.path())
    );
    assert_eq!(
        config.permissions.network_sandbox_policy(),
        NetworkSandboxPolicy::Restricted
    );
    assert_eq!(
        config
            .permissions
            .active_permission_profile()
            .as_ref()
            .map(|active| active.id.as_str()),
        Some("dev")
    );
    Ok(())
}

#[tokio::test]
async fn default_permissions_extended_profile_preserves_parent_metadata() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    std::fs::write(cwd.path().join(".git"), "gitdir: nowhere")?;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some("dev".to_string()),
            permissions: Some(PermissionsToml {
                entries: BTreeMap::from([
                    (
                        "base".to_string(),
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
                            network: None,
                        },
                    ),
                    (
                        "dev".to_string(),
                        PermissionProfileToml {
                            description: None,
                            extends: Some("base".to_string()),
                            workspace_roots: None,
                            filesystem: None,
                            network: None,
                        },
                    ),
                ]),
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
        config.permissions.active_permission_profile(),
        Some(ActivePermissionProfile {
            id: "dev".to_string(),
            extends: Some("base".to_string()),
            modifications: Vec::new(),
        })
    );
    Ok(())
}

#[tokio::test]
async fn permission_profile_override_populates_runtime_permissions() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let permission_profile = PermissionProfile::Disabled;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            permission_profile: Some(permission_profile.clone()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert_eq!(config.permissions.permission_profile(), permission_profile);
    assert_eq!(config.permissions.active_permission_profile(), None);
    assert_eq!(
        &config.legacy_sandbox_policy(),
        &SandboxPolicy::DangerFullAccess
    );
    Ok(())
}

#[test]
fn permission_snapshot_setter_preserves_permission_constraints() {
    let initial_profile = PermissionProfile::read_only();
    let mut permissions = Permissions::from_approval_and_profile(
        Constrained::allow_any(AskForApproval::Never),
        Constrained::allow_only(initial_profile.clone()),
    )
    .expect("initial permissions should satisfy constraints");

    let err = permissions
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::active(
            PermissionProfile::workspace_write(),
            ActivePermissionProfile::new(BUILT_IN_PERMISSION_PROFILE_WORKSPACE),
        ))
        .expect_err("workspace profile should violate read-only constraint");

    assert_eq!(permissions.permission_profile(), &initial_profile);
    assert_eq!(permissions.active_permission_profile(), None);
    assert!(
        matches!(err, ConstraintError::InvalidValue { .. }),
        "expected invalid value constraint error, got {err:?}"
    );
}

#[tokio::test]
async fn permission_profile_override_preserves_managed_unrestricted_filesystem()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let permission_profile = PermissionProfile::Managed {
        file_system: ManagedFileSystemPermissions::Unrestricted,
        network: NetworkSandboxPolicy::Restricted,
    };

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            permission_profile: Some(permission_profile.clone()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert_eq!(config.permissions.permission_profile(), permission_profile);
    assert_eq!(
        &config.legacy_sandbox_policy(),
        &SandboxPolicy::ExternalSandbox {
            network_access: NetworkAccess::Restricted,
        }
    );
    Ok(())
}

#[tokio::test]
async fn managed_unrestricted_permission_profile_still_enables_network_requirements()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let permission_profile = PermissionProfile::Managed {
        file_system: ManagedFileSystemPermissions::Unrestricted,
        network: NetworkSandboxPolicy::Enabled,
    };

    let mut config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            permission_profile: Some(permission_profile),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;
    assert_eq!(
        &config.legacy_sandbox_policy(),
        &SandboxPolicy::DangerFullAccess,
        "the legacy projection is intentionally lossy for managed unrestricted profiles"
    );

    let layers = config
        .config_layer_stack
        .get_layers(
            ConfigLayerStackOrdering::LowestPrecedenceFirst,
            /*include_disabled*/ true,
        )
        .into_iter()
        .cloned()
        .collect();
    let mut requirements = config.config_layer_stack.requirements().clone();
    requirements.network = Some(Sourced::new(
        codex_config::NetworkConstraints {
            enabled: Some(true),
            ..Default::default()
        },
        RequirementSource::CloudRequirements,
    ));
    let mut requirements_toml = config.config_layer_stack.requirements_toml().clone();
    requirements_toml.network = Some(codex_config::NetworkRequirementsToml {
        enabled: Some(true),
        ..Default::default()
    });
    config.config_layer_stack = ConfigLayerStack::new(layers, requirements, requirements_toml)
        .expect("config layer stack with network requirements");

    assert!(config.managed_network_requirements_enabled());
    Ok(())
}

#[tokio::test]
async fn permission_profile_override_applies_runtime_roots_to_legacy_projection()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let permission_profile = PermissionProfile::from_runtime_permissions(
        &FileSystemSandboxPolicy::restricted(vec![
            FileSystemSandboxEntry {
                path: FileSystemPath::Special {
                    value: FileSystemSpecialPath::Root,
                },
                access: FileSystemAccessMode::Read,
            },
            FileSystemSandboxEntry {
                path: FileSystemPath::Special {
                    value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
                },
                access: FileSystemAccessMode::Write,
            },
        ]),
        NetworkSandboxPolicy::Restricted,
    );

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            permission_profile: Some(permission_profile),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    let memories_root = codex_home.path().join("memories").abs();
    assert!(
        config
            .permissions
            .file_system_sandbox_policy()
            .can_write_path_with_cwd(memories_root.as_path(), cwd.path())
    );
    assert_eq!(
        &config.legacy_sandbox_policy(),
        &SandboxPolicy::WorkspaceWrite {
            writable_roots: vec![memories_root],
            network_access: false,
            exclude_tmpdir_env_var: true,
            exclude_slash_tmp: true,
        }
    );
    Ok(())
}

#[tokio::test]
async fn permission_profile_override_preserves_configured_network_policy_without_starting_proxy()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let permission_profile = PermissionProfile::Disabled;

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
                            allow_upstream_proxy: Some(false),
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
            permission_profile: Some(permission_profile.clone()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;
    assert!(
        config.permissions.network.is_none(),
        "profile network.enabled should not start the managed network proxy"
    );
    assert_eq!(config.permissions.permission_profile(), permission_profile);
    Ok(())
}

#[tokio::test]
async fn workspace_root_glob_none_compiles_to_filesystem_pattern_entry() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let extra_root = TempDir::new()?;
    tokio::fs::write(cwd.path().join(".git"), "gitdir: nowhere").await?;
    tokio::fs::write(extra_root.path().join(".git"), "gitdir: nowhere").await?;

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
                            glob_scan_max_depth: Some(2),
                            entries: BTreeMap::from([(
                                ":workspace_roots".to_string(),
                                FilesystemPermissionToml::Scoped(BTreeMap::from([
                                    (".".to_string(), FileSystemAccessMode::Write),
                                    ("**/*.env".to_string(), FileSystemAccessMode::Deny),
                                ])),
                            )]),
                        }),
                        network: None,
                    },
                )]),
            }),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            additional_writable_roots: vec![extra_root.path().to_path_buf()],
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        config
            .permissions
            .file_system_sandbox_policy()
            .glob_scan_max_depth,
        Some(2)
    );
    for root in [cwd.path(), extra_root.path()] {
        let expected_pattern = AbsolutePathBuf::resolve_path_against_base("**/*.env", root)
            .to_string_lossy()
            .into_owned();
        assert!(
            config
                .permissions
                .file_system_sandbox_policy()
                .entries
                .contains(&FileSystemSandboxEntry {
                    path: FileSystemPath::GlobPattern {
                        pattern: expected_pattern,
                    },
                    access: FileSystemAccessMode::Deny,
                })
        );
    }
    assert!(
        !config
            .permissions
            .file_system_sandbox_policy()
            .entries
            .iter()
            .any(|entry| matches!(
                &entry.path,
                FileSystemPath::Special {
                    value: FileSystemSpecialPath::ProjectRoots { subpath: Some(subpath) },
                } if subpath == std::path::Path::new("**/*.env")
            )),
        "glob should compile to a filesystem pattern entry, not a literal filesystem entry"
    );
    Ok(())
}

#[tokio::test]
async fn permissions_profiles_require_default_permissions() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    std::fs::write(cwd.path().join(".git"), "gitdir: nowhere")?;

    let err = Config::load_from_base_config_with_overrides(
        ConfigToml {
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
                        network: None,
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
    .await
    .expect_err("missing default_permissions should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "config defines `[permissions]` profiles but does not set `default_permissions`"
    );
    Ok(())
}

#[tokio::test]
async fn default_permissions_can_select_builtin_profile_without_permissions_table()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some(BUILT_IN_PERMISSION_PROFILE_WORKSPACE.to_string()),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert!(config.explicit_permission_profile_mode);
    assert!(config.custom_permission_profile_ids.is_empty());
    let policy = config.permissions.file_system_sandbox_policy();
    assert_eq!(
        config
            .permissions
            .active_permission_profile()
            .as_ref()
            .map(|active| active.id.as_str()),
        Some(BUILT_IN_PERMISSION_PROFILE_WORKSPACE)
    );
    assert!(
        policy.can_write_path_with_cwd(cwd.path(), cwd.path()),
        "expected :workspace to allow writing the project root, policy: {policy:?}"
    );
    assert!(
        !policy.can_write_path_with_cwd(&cwd.path().join(".git"), cwd.path()),
        "expected :workspace to protect project metadata, policy: {policy:?}"
    );
    Ok(())
}

#[tokio::test]
async fn default_permissions_read_only_keeps_add_dir_read_only() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let extra_root = TempDir::new()?;
    let extra_root = extra_root.path().abs();

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some(BUILT_IN_PERMISSION_PROFILE_READ_ONLY.to_string()),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            additional_writable_roots: vec![extra_root.to_path_buf()],
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    let policy = config.permissions.file_system_sandbox_policy();
    assert!(
        !policy.can_write_path_with_cwd(extra_root.as_path(), cwd.path()),
        "expected :read-only to stay read-only for runtime workspace roots, policy: {policy:?}"
    );
    assert_eq!(
        config.permissions.active_permission_profile(),
        Some(ActivePermissionProfile::new(
            BUILT_IN_PERMISSION_PROFILE_READ_ONLY,
        ))
    );
    Ok(())
}

