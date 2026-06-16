use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn workspace_profile_applies_rules_to_runtime_and_profile_workspace_roots()
-> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let codex_home = temp_dir.path().join("codex-home");
    let cwd = temp_dir.path().join("frontend");
    let runtime_root = temp_dir.path().join("backend");
    let profile_root = temp_dir.path().join("shared");
    for root in [&cwd, &runtime_root, &profile_root] {
        std::fs::create_dir_all(root.join(".git"))?;
        std::fs::create_dir_all(root.join(".codex"))?;
    }

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some("dev".to_string()),
            permissions: Some(PermissionsToml {
                entries: BTreeMap::from([(
                    "dev".to_string(),
                    PermissionProfileToml {
                        description: None,
                        extends: None,
                        workspace_roots: Some(WorkspaceRootsToml {
                            entries: BTreeMap::from([(
                                profile_root.to_string_lossy().into_owned(),
                                true,
                            )]),
                        }),
                        filesystem: Some(FilesystemPermissionsToml {
                            glob_scan_max_depth: None,
                            entries: BTreeMap::from([(
                                ":workspace_roots".to_string(),
                                FilesystemPermissionToml::Scoped(BTreeMap::from([
                                    (".".to_string(), FileSystemAccessMode::Write),
                                    (".git".to_string(), FileSystemAccessMode::Read),
                                    (".codex".to_string(), FileSystemAccessMode::Read),
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
            cwd: Some(cwd.clone()),
            additional_writable_roots: vec![runtime_root.clone()],
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    let cwd_abs = cwd.abs();
    let runtime_root_abs = runtime_root.abs();
    let profile_root_abs = profile_root.abs();
    assert_eq!(
        config.workspace_roots,
        vec![cwd_abs.clone(), runtime_root_abs.clone()]
    );
    assert_eq!(
        config.permissions.workspace_roots(),
        &[cwd_abs.clone(), runtime_root_abs.clone()]
    );
    assert_eq!(
        config.effective_workspace_roots(),
        vec![
            cwd_abs.clone(),
            runtime_root_abs.clone(),
            profile_root_abs.clone()
        ]
    );

    let policy = config.permissions.file_system_sandbox_policy();
    for root in [cwd_abs, runtime_root_abs, profile_root_abs.clone()] {
        assert!(
            policy.can_write_path_with_cwd(root.as_path(), cwd.as_path()),
            "expected workspace root to be writable, policy: {policy:?}"
        );
        assert!(
            !policy.can_write_path_with_cwd(&root.join(".git"), cwd.as_path()),
            "expected .git carveout under {root:?}, policy: {policy:?}"
        );
        assert!(
            !policy.can_write_path_with_cwd(&root.join(".codex"), cwd.as_path()),
            "expected .codex carveout under {root:?}, policy: {policy:?}"
        );
    }
    assert_eq!(
        config.permissions.profile_workspace_roots(),
        std::slice::from_ref(&profile_root_abs)
    );
    assert_eq!(
        config.permissions.active_permission_profile(),
        Some(ActivePermissionProfile::new("dev"))
    );
    Ok(())
}

#[tokio::test]
async fn explicit_builtin_workspace_profile_ignores_legacy_workspace_write_settings()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let extra_root = TempDir::new()?;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some(BUILT_IN_PERMISSION_PROFILE_WORKSPACE.to_string()),
            sandbox_workspace_write: Some(SandboxWorkspaceWrite {
                writable_roots: vec![extra_root.path().abs()],
                network_access: true,
                exclude_tmpdir_env_var: true,
                exclude_slash_tmp: true,
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

    let policy = config.permissions.file_system_sandbox_policy();
    assert_eq!(
        config.permissions.network_sandbox_policy(),
        NetworkSandboxPolicy::Restricted
    );
    assert!(
        !policy.entries.iter().any(|entry| matches!(
            &entry.path,
            FileSystemPath::Path { path } if path.as_path() == extra_root.path()
        )),
        "explicit :workspace should not inherit sandbox_workspace_write roots as concrete grants, \
         policy: {policy:?}"
    );
    Ok(())
}

#[tokio::test]
async fn default_permissions_profile_can_extend_builtin_workspace() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some("workspace-with-network".to_string()),
            permissions: Some(PermissionsToml {
                entries: BTreeMap::from([(
                    "workspace-with-network".to_string(),
                    PermissionProfileToml {
                        description: None,
                        extends: Some(BUILT_IN_PERMISSION_PROFILE_WORKSPACE.to_string()),
                        workspace_roots: None,
                        filesystem: None,
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

    let policy = config.permissions.file_system_sandbox_policy();
    assert!(
        policy.can_write_path_with_cwd(cwd.path(), cwd.path()),
        "expected profile extending :workspace to keep project-root writes, policy: {policy:?}"
    );
    assert!(
        !policy.can_write_path_with_cwd(&cwd.path().join(".git"), cwd.path()),
        "expected profile extending :workspace to keep metadata carveouts, policy: {policy:?}"
    );
    assert_eq!(
        config.permissions.network_sandbox_policy(),
        NetworkSandboxPolicy::Enabled
    );
    assert_eq!(
        config.permissions.active_permission_profile(),
        Some(ActivePermissionProfile {
            id: "workspace-with-network".to_string(),
            extends: Some(BUILT_IN_PERMISSION_PROFILE_WORKSPACE.to_string()),
            modifications: Vec::new(),
        })
    );
    Ok(())
}

#[tokio::test]
async fn default_permissions_profile_can_extend_builtin_read_only() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some("read-only-with-network".to_string()),
            permissions: Some(PermissionsToml {
                entries: BTreeMap::from([(
                    "read-only-with-network".to_string(),
                    PermissionProfileToml {
                        description: None,
                        extends: Some(BUILT_IN_PERMISSION_PROFILE_READ_ONLY.to_string()),
                        workspace_roots: None,
                        filesystem: None,
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

    let policy = config.permissions.file_system_sandbox_policy();
    assert!(
        policy.can_read_path_with_cwd(cwd.path(), cwd.path()),
        "expected profile extending :read-only to keep read access, policy: {policy:?}"
    );
    assert!(
        !policy.can_write_path_with_cwd(cwd.path(), cwd.path()),
        "expected profile extending :read-only to stay non-writable, policy: {policy:?}"
    );
    assert_eq!(
        config.permissions.network_sandbox_policy(),
        NetworkSandboxPolicy::Enabled
    );
    assert_eq!(
        config.permissions.active_permission_profile(),
        Some(ActivePermissionProfile {
            id: "read-only-with-network".to_string(),
            extends: Some(BUILT_IN_PERMISSION_PROFILE_READ_ONLY.to_string()),
            modifications: Vec::new(),
        })
    );
    Ok(())
}

#[tokio::test]
async fn empty_config_defaults_to_builtin_profile_for_trusted_project() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let project_key = cwd.path().to_string_lossy().to_string();

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            projects: Some(HashMap::from([(
                project_key,
                ProjectConfig {
                    trust_level: Some(TrustLevel::Trusted),
                },
            )])),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    let policy = config.permissions.file_system_sandbox_policy();
    assert_eq!(
        config
            .permissions
            .active_permission_profile()
            .as_ref()
            .map(|active| active.id.as_str()),
        Some(if cfg!(target_os = "windows") {
            BUILT_IN_PERMISSION_PROFILE_READ_ONLY
        } else {
            BUILT_IN_PERMISSION_PROFILE_WORKSPACE
        })
    );
    if cfg!(target_os = "windows") {
        assert!(
            !policy.can_write_path_with_cwd(cwd.path(), cwd.path()),
            "expected trusted project fallback to stay read-only without Windows sandbox support, policy: {policy:?}"
        );
    } else {
        assert!(
            policy.can_write_path_with_cwd(cwd.path(), cwd.path()),
            "expected trusted project fallback to use :workspace, policy: {policy:?}"
        );
        assert!(
            !policy.can_write_path_with_cwd(&cwd.path().join(".codex"), cwd.path()),
            "expected :workspace metadata carveouts, policy: {policy:?}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn implicit_builtin_workspace_profile_preserves_sandbox_workspace_write_settings()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let extra_root = TempDir::new()?;
    let extra_root = extra_root.path().abs();
    let project_key = cwd.path().to_string_lossy().to_string();

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            projects: Some(HashMap::from([(
                project_key,
                ProjectConfig {
                    trust_level: Some(TrustLevel::Trusted),
                },
            )])),
            sandbox_workspace_write: Some(SandboxWorkspaceWrite {
                writable_roots: vec![extra_root.clone()],
                network_access: true,
                exclude_tmpdir_env_var: true,
                exclude_slash_tmp: false,
            }),
            windows: Some(WindowsToml {
                sandbox: Some(WindowsSandboxModeToml::Elevated),
                sandbox_private_desktop: None,
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

    let policy = config.permissions.file_system_sandbox_policy();
    assert!(
        policy.can_write_path_with_cwd(extra_root.as_path(), cwd.path()),
        "expected implicit :workspace to preserve sandbox_workspace_write.writable_roots, policy: {policy:?}"
    );
    assert_eq!(
        config.permissions.network_sandbox_policy(),
        NetworkSandboxPolicy::Enabled
    );
    assert_eq!(
        config.permissions.active_permission_profile(),
        None,
        "implicit :workspace cannot be faithfully re-selected when it includes \
         legacy sandbox_workspace_write settings"
    );
    match config.legacy_sandbox_policy() {
        SandboxPolicy::WorkspaceWrite {
            writable_roots,
            network_access,
            exclude_tmpdir_env_var,
            exclude_slash_tmp,
        } => {
            assert!(writable_roots.contains(&extra_root));
            assert!(network_access);
            assert!(exclude_tmpdir_env_var);
            assert!(!exclude_slash_tmp);
        }
        sandbox_policy => panic!("expected workspace-write projection, got {sandbox_policy:?}"),
    }
    Ok(())
}

#[tokio::test]
async fn implicit_builtin_workspace_profile_preserves_add_dir_metadata_carveouts()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let extra_root = TempDir::new()?;
    for subpath in [".git", ".agents", ".codex"] {
        std::fs::create_dir_all(extra_root.path().join(subpath))?;
    }
    let project_key = cwd.path().to_string_lossy().to_string();

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            projects: Some(HashMap::from([(
                project_key,
                ProjectConfig {
                    trust_level: Some(TrustLevel::Trusted),
                },
            )])),
            windows: Some(WindowsToml {
                sandbox: Some(WindowsSandboxModeToml::Elevated),
                sandbox_private_desktop: None,
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

    let policy = config.permissions.file_system_sandbox_policy();
    let extra_root = extra_root.path().abs();
    assert!(
        policy.can_write_path_with_cwd(extra_root.as_path(), cwd.path()),
        "expected implicit :workspace to preserve additional writable roots, policy: {policy:?}"
    );
    for subpath in [".git", ".agents", ".codex"] {
        assert!(
            !policy.can_write_path_with_cwd(&extra_root.join(subpath), cwd.path()),
            "expected implicit :workspace to preserve legacy metadata carveout for {subpath}, \
             policy: {policy:?}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn empty_config_defaults_to_builtin_read_only_without_trust_decision() -> std::io::Result<()>
{
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    let policy = config.permissions.file_system_sandbox_policy();
    assert!(
        policy.can_read_path_with_cwd(cwd.path(), cwd.path()),
        "expected :read-only to allow reads, policy: {policy:?}"
    );
    assert!(
        !policy.can_write_path_with_cwd(cwd.path(), cwd.path()),
        "expected :read-only to deny writes, policy: {policy:?}"
    );
    Ok(())
}

#[tokio::test]
async fn default_permissions_can_select_builtin_full_access_profile() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some(BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS.to_string()),
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
        config.permissions.permission_profile(),
        PermissionProfile::Disabled
    );
    assert_eq!(
        config
            .permissions
            .active_permission_profile()
            .as_ref()
            .map(|active| active.id.as_str()),
        Some(BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS)
    );
    Ok(())
}

#[tokio::test]
async fn legacy_danger_no_sandbox_is_rejected() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;

    let err = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some(":danger-no-sandbox".to_string()),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await
    .expect_err("legacy full-access alias should be rejected");

    assert_eq!(
        err.to_string(),
        "default_permissions refers to unknown built-in profile `:danger-no-sandbox`"
    );
    Ok(())
}

#[tokio::test]
async fn user_defined_permission_profile_names_cannot_use_builtin_prefix() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;

    let err = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some(":custom".to_string()),
            permissions: Some(PermissionsToml {
                entries: BTreeMap::from([(
                    ":custom".to_string(),
                    PermissionProfileToml::default(),
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
    .expect_err("reserved profile name should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "permissions profile `:custom` uses a reserved built-in profile prefix"
    );
    Ok(())
}

#[tokio::test]
async fn unknown_builtin_permission_profile_name_is_rejected() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;

    let err = Config::load_from_base_config_with_overrides(
        ConfigToml {
            default_permissions: Some(":unknown".to_string()),
            ..Default::default()
        },
        ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await
    .expect_err("unknown built-in profile name should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "default_permissions refers to unknown built-in profile `:unknown`"
    );
    Ok(())
}

