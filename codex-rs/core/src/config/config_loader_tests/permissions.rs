use super::*;
use super::common::make_config_for_test;

#[tokio::test]
async fn system_requirements_define_managed_permission_profiles() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    tokio::fs::write(
        codex_home.join(CONFIG_TOML_FILE),
        r#"
default_permissions = "managed-standard"
"#,
    )
    .await?;
    let requirements_path = tmp.path().join("requirements.toml");
    tokio::fs::write(
        &requirements_path,
        r#"
allowed_permissions = ["managed-standard"]

[permissions.managed-standard]
extends = ":workspace"
"#,
    )
    .await?;

    let cwd = AbsolutePathBuf::from_absolute_path(tmp.path())?;
    let mut overrides = LoaderOverrides::without_managed_config_for_tests();
    overrides.system_requirements_path = Some(requirements_path);
    let config = ConfigBuilder::default()
        .codex_home(codex_home)
        .fallback_cwd(Some(cwd.to_path_buf()))
        .loader_overrides(overrides)
        .build()
        .await?;

    assert_eq!(
        config
            .config_layer_stack
            .requirements_toml()
            .allowed_permissions,
        Some(vec!["managed-standard".to_string()])
    );
    assert_eq!(
        config
            .permissions
            .active_permission_profile()
            .map(|profile| profile.id),
        Some("managed-standard".to_string())
    );
    Ok(())
}

#[tokio::test]
async fn system_allowed_permissions_keep_builtin_permission_fallbacks() -> anyhow::Result<()> {
    for (trust_level, expected_profile) in [
        (
            Some(TrustLevel::Trusted),
            if cfg!(target_os = "windows") {
                BUILT_IN_PERMISSION_PROFILE_READ_ONLY
            } else {
                BUILT_IN_PERMISSION_PROFILE_WORKSPACE
            },
        ),
        (
            Some(TrustLevel::Untrusted),
            if cfg!(target_os = "windows") {
                BUILT_IN_PERMISSION_PROFILE_READ_ONLY
            } else {
                BUILT_IN_PERMISSION_PROFILE_WORKSPACE
            },
        ),
        (None, BUILT_IN_PERMISSION_PROFILE_READ_ONLY),
    ] {
        let tmp = tempdir()?;
        let codex_home = tmp.path().join("home");
        tokio::fs::create_dir_all(&codex_home).await?;
        if let Some(trust_level) = trust_level {
            make_config_for_test(
                &codex_home,
                tmp.path(),
                trust_level,
                /*project_root_markers*/ None,
            )
            .await?;
        }
        let requirements_path = tmp.path().join("requirements.toml");
        tokio::fs::write(
            &requirements_path,
            r#"
allowed_permissions = ["managed-standard"]

[permissions.managed-standard.filesystem]
":workspace_roots" = "read"
"#,
        )
        .await?;

        let cwd = AbsolutePathBuf::from_absolute_path(tmp.path())?;
        let mut overrides = LoaderOverrides::without_managed_config_for_tests();
        overrides.system_requirements_path = Some(requirements_path);
        let config = ConfigBuilder::default()
            .codex_home(codex_home)
            .fallback_cwd(Some(cwd.to_path_buf()))
            .loader_overrides(overrides)
            .build()
            .await?;

        assert_eq!(
            config
                .permissions
                .active_permission_profile()
                .map(|profile| profile.id),
            Some(expected_profile.to_string()),
            "trust level {trust_level:?}",
        );
    }
    Ok(())
}

#[tokio::test]
async fn system_allowed_permissions_keep_explicit_builtin_defaults() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    tokio::fs::write(
        codex_home.join(CONFIG_TOML_FILE),
        r#"
default_permissions = ":workspace"
"#,
    )
    .await?;
    let requirements_path = tmp.path().join("requirements.toml");
    tokio::fs::write(
        &requirements_path,
        r#"
allowed_permissions = ["managed-standard"]

[permissions.managed-standard.filesystem]
":workspace_roots" = "read"
"#,
    )
    .await?;

    let cwd = AbsolutePathBuf::from_absolute_path(tmp.path())?;
    let mut overrides = LoaderOverrides::without_managed_config_for_tests();
    overrides.system_requirements_path = Some(requirements_path);
    let config = ConfigBuilder::default()
        .codex_home(codex_home)
        .fallback_cwd(Some(cwd.to_path_buf()))
        .loader_overrides(overrides)
        .build()
        .await?;

    assert_eq!(
        config
            .permissions
            .active_permission_profile()
            .map(|profile| profile.id),
        Some(BUILT_IN_PERMISSION_PROFILE_WORKSPACE.to_string())
    );
    Ok(())
}

#[tokio::test]
async fn system_requirements_preserve_allowed_configured_permission_default() -> anyhow::Result<()>
{
    let tmp = tempdir()?;
    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    tokio::fs::write(
        codex_home.join(CONFIG_TOML_FILE),
        r#"
default_permissions = "managed-build"
"#,
    )
    .await?;
    let requirements_path = tmp.path().join("requirements.toml");
    tokio::fs::write(
        &requirements_path,
        r#"
allowed_permissions = ["managed-standard", "managed-build"]

[permissions.managed-standard]
extends = ":read-only"

[permissions.managed-build]
extends = ":workspace"
"#,
    )
    .await?;

    let cwd = AbsolutePathBuf::from_absolute_path(tmp.path())?;
    let mut overrides = LoaderOverrides::without_managed_config_for_tests();
    overrides.system_requirements_path = Some(requirements_path);
    let config = ConfigBuilder::default()
        .codex_home(codex_home)
        .fallback_cwd(Some(cwd.to_path_buf()))
        .loader_overrides(overrides)
        .build()
        .await?;

    assert_eq!(
        config
            .permissions
            .active_permission_profile()
            .map(|profile| profile.id),
        Some("managed-build".to_string())
    );
    Ok(())
}

#[tokio::test]
async fn system_requirements_warn_for_disallowed_explicit_permission_override() -> anyhow::Result<()>
{
    let tmp = tempdir()?;
    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    let requirements_path = tmp.path().join("requirements.toml");
    tokio::fs::write(
        &requirements_path,
        r#"
allowed_permissions = ["managed-standard"]

[permissions.managed-standard]
extends = ":workspace"
"#,
    )
    .await?;

    let cwd = AbsolutePathBuf::from_absolute_path(tmp.path())?;
    let mut overrides = LoaderOverrides::without_managed_config_for_tests();
    overrides.system_requirements_path = Some(requirements_path);
    let config = ConfigBuilder::default()
        .codex_home(codex_home)
        .fallback_cwd(Some(cwd.to_path_buf()))
        .harness_overrides(ConfigOverrides {
            default_permissions: Some("managed-build".to_string()),
            ..ConfigOverrides::default()
        })
        .loader_overrides(overrides)
        .build()
        .await?;

    assert_eq!(
        config
            .permissions
            .active_permission_profile()
            .map(|profile| profile.id),
        Some("managed-standard".to_string())
    );
    assert!(
        config.startup_warnings.iter().any(|warning| warning
            .contains("Configured value for `permission_profile` is disallowed by requirements")),
        "{:?}",
        config.startup_warnings
    );
    Ok(())
}
