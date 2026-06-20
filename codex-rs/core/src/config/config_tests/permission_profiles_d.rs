use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn permissions_profiles_allow_direct_write_roots_outside_workspace_root()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    std::fs::write(cwd.path().join(".git"), "gitdir: nowhere")?;
    let external_write_dir = TempDir::new()?;
    let external_write_path =
        AbsolutePathBuf::from_absolute_path(std::fs::canonicalize(external_write_dir.path())?)?;

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
                                external_write_path.to_string_lossy().into_owned(),
                                FilesystemPermissionToml::Access(FileSystemAccessMode::Write),
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
    .await?;

    assert_eq!(
        config.custom_permission_profile_ids,
        vec!["dev".to_string()]
    );
    let memories_root = AbsolutePathBuf::from_absolute_path(std::fs::canonicalize(
        codex_home.path().join("memories"),
    )?)?;
    assert!(
        config
            .permissions
            .file_system_sandbox_policy()
            .can_write_path_with_cwd(external_write_path.as_path(), cwd.path())
    );
    assert_eq!(
        &config.legacy_sandbox_policy(),
        &SandboxPolicy::WorkspaceWrite {
            writable_roots: vec![external_write_path, memories_root],
            network_access: false,
            exclude_tmpdir_env_var: true,
            exclude_slash_tmp: true,
        }
    );
    Ok(())
}

#[tokio::test]
async fn permissions_profiles_reject_nested_entries_for_non_workspace_roots() -> std::io::Result<()>
{
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    std::fs::write(cwd.path().join(".git"), "gitdir: nowhere")?;

    let err = Config::load_from_base_config_with_overrides(
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
                                FilesystemPermissionToml::Scoped(BTreeMap::from([(
                                    "docs".to_string(),
                                    FileSystemAccessMode::Read,
                                )])),
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
    .expect_err("nested entries outside :workspace_roots should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "filesystem path `:minimal` does not support nested entries"
    );
    Ok(())
}

async fn load_workspace_permission_profile(
    profile: PermissionProfileToml,
) -> std::io::Result<Config> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    std::fs::write(cwd.path().join(".git"), "gitdir: nowhere")?;

    Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some("dev".to_string()),
            permissions: Some(PermissionsToml {
                entries: BTreeMap::from([("dev".to_string(), profile)]),
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
}

#[tokio::test]
async fn permissions_profiles_allow_unknown_special_paths() -> std::io::Result<()> {
    let config = load_workspace_permission_profile(PermissionProfileToml {
        description: None,
        extends: None,
        workspace_roots: None,
        filesystem: Some(FilesystemPermissionsToml {
            glob_scan_max_depth: None,
            entries: BTreeMap::from([(
                ":future_special_path".to_string(),
                FilesystemPermissionToml::Access(FileSystemAccessMode::Read),
            )]),
        }),
        network: None,
    })
    .await?;

    assert_eq!(
        config.permissions.file_system_sandbox_policy(),
        FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::unknown(
                    ":future_special_path",
                    /*subpath*/ None
                ),
            },
            access: FileSystemAccessMode::Read,
        }]),
    );
    assert_eq!(
        &config.legacy_sandbox_policy(),
        &SandboxPolicy::ReadOnly {
            network_access: false,
        }
    );
    assert!(
        config.startup_warnings.iter().any(|warning| warning.contains(
            "Configured filesystem path `:future_special_path` is not recognized by this version of Codex and will be ignored."
        )),
        "{:?}",
        config.startup_warnings
    );
    Ok(())
}

#[tokio::test]
async fn permissions_profiles_allow_unknown_special_paths_with_nested_entries()
-> std::io::Result<()> {
    let config = load_workspace_permission_profile(PermissionProfileToml {
        description: None,
        extends: None,
        workspace_roots: None,
        filesystem: Some(FilesystemPermissionsToml {
            glob_scan_max_depth: None,
            entries: BTreeMap::from([(
                ":future_special_path".to_string(),
                FilesystemPermissionToml::Scoped(BTreeMap::from([(
                    "docs".to_string(),
                    FileSystemAccessMode::Read,
                )])),
            )]),
        }),
        network: None,
    })
    .await?;

    assert_eq!(
        config.permissions.file_system_sandbox_policy(),
        FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::unknown(":future_special_path", Some("docs".into())),
            },
            access: FileSystemAccessMode::Read,
        }]),
    );
    assert!(
        config.startup_warnings.iter().any(|warning| warning.contains(
            "Configured filesystem path `:future_special_path` with nested entry `docs` is not recognized by this version of Codex and will be ignored."
        )),
        "{:?}",
        config.startup_warnings
    );
    Ok(())
}

#[tokio::test]
async fn permissions_profiles_allow_missing_filesystem_with_warning() -> std::io::Result<()> {
    let config = load_workspace_permission_profile(PermissionProfileToml {
        description: None,
        extends: None,
        workspace_roots: None,
        filesystem: None,
        network: None,
    })
    .await?;

    assert_eq!(
        config.permissions.file_system_sandbox_policy(),
        FileSystemSandboxPolicy::restricted(Vec::new())
    );
    assert_eq!(
        &config.legacy_sandbox_policy(),
        &SandboxPolicy::ReadOnly {
            network_access: false,
        }
    );
    assert!(
        config.startup_warnings.iter().any(|warning| warning.contains(
            "Permissions profile `dev` does not define any recognized filesystem entries for this version of Codex."
        )),
        "{:?}",
        config.startup_warnings
    );
    Ok(())
}

#[tokio::test]
async fn permissions_profiles_allow_empty_filesystem_with_warning() -> std::io::Result<()> {
    let config = load_workspace_permission_profile(PermissionProfileToml {
        description: None,
        extends: None,
        workspace_roots: None,
        filesystem: Some(FilesystemPermissionsToml {
            glob_scan_max_depth: None,
            entries: BTreeMap::new(),
        }),
        network: None,
    })
    .await?;

    assert_eq!(
        config.permissions.file_system_sandbox_policy(),
        FileSystemSandboxPolicy::restricted(Vec::new())
    );
    assert!(
        config.startup_warnings.iter().any(|warning| warning.contains(
            "Permissions profile `dev` does not define any recognized filesystem entries for this version of Codex."
        )),
        "{:?}",
        config.startup_warnings
    );
    Ok(())
}

#[tokio::test]
async fn permissions_profiles_reject_workspace_root_parent_traversal() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    std::fs::write(cwd.path().join(".git"), "gitdir: nowhere")?;

    let err = Config::load_from_base_config_with_overrides(
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
                                ":workspace_roots".to_string(),
                                FilesystemPermissionToml::Scoped(BTreeMap::from([(
                                    "../sibling".to_string(),
                                    FileSystemAccessMode::Read,
                                )])),
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
    .expect_err("parent traversal should be rejected for project root subpaths");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "filesystem subpath `../sibling` must be a descendant path without `.` or `..` components"
    );
    Ok(())
}

#[tokio::test]
async fn permissions_profiles_allow_network_enablement() -> std::io::Result<()> {
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

    assert!(
        config.permissions.network_sandbox_policy().is_enabled(),
        "expected network sandbox policy to be enabled",
    );
    assert!(config.legacy_sandbox_policy().has_full_network_access());
    Ok(())
}
