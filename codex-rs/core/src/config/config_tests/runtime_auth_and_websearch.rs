use super::*;

#[tokio::test]
async fn add_dir_override_extends_workspace_writable_roots() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let frontend = temp_dir.path().join("frontend");
    let backend = temp_dir.path().join("backend");
    std::fs::create_dir_all(&frontend)?;
    std::fs::create_dir_all(&backend)?;

    let overrides = ConfigOverrides {
        cwd: Some(frontend),
        sandbox_mode: Some(SandboxMode::WorkspaceWrite),
        additional_writable_roots: vec![PathBuf::from("../backend"), backend.clone()],
        ..Default::default()
    };

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        overrides,
        temp_dir.path().abs(),
    )
    .await?;

    let expected_backend = backend.abs();
    if cfg!(target_os = "windows") {
        match &config.legacy_sandbox_policy() {
            SandboxPolicy::ReadOnly { .. } => {}
            other => panic!("expected read-only policy on Windows, got {other:?}"),
        }
    } else {
        match &config.legacy_sandbox_policy() {
            SandboxPolicy::WorkspaceWrite { writable_roots, .. } => {
                assert_eq!(
                    writable_roots
                        .iter()
                        .filter(|root| **root == expected_backend)
                        .count(),
                    1,
                    "expected single writable root entry for {}",
                    expected_backend.display()
                );
            }
            other => panic!("expected workspace-write policy, got {other:?}"),
        }
    }

    Ok(())
}

#[tokio::test]
async fn default_zsh_path_sets_runtime_zsh_path() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let default_zsh_path = codex_home.path().join("packaged-zsh");

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides {
            default_zsh_path: Some(default_zsh_path.abs()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;
    assert_eq!(config.zsh_path, Some(default_zsh_path));

    Ok(())
}

#[tokio::test]
async fn sqlite_home_defaults_to_codex_home_for_workspace_write() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides {
            sandbox_mode: Some(SandboxMode::WorkspaceWrite),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert_eq!(config.sqlite_home, codex_home.path().to_path_buf());

    Ok(())
}

#[tokio::test]
async fn workspace_write_always_includes_memories_root_once() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let memories_root = codex_home.path().join("memories");
    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            sandbox_workspace_write: Some(SandboxWorkspaceWrite {
                writable_roots: vec![memories_root.abs()],
                ..Default::default()
            }),
            ..Default::default()
        },
        ConfigOverrides {
            sandbox_mode: Some(SandboxMode::WorkspaceWrite),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    if cfg!(target_os = "windows") {
        match &config.legacy_sandbox_policy() {
            SandboxPolicy::ReadOnly { .. } => {}
            other => panic!("expected read-only policy on Windows, got {other:?}"),
        }
    } else {
        assert!(
            memories_root.is_dir(),
            "expected memories root directory to exist at {}",
            memories_root.display()
        );
        let expected_memories_root = memories_root.abs();
        match &config.legacy_sandbox_policy() {
            SandboxPolicy::WorkspaceWrite { writable_roots, .. } => {
                assert_eq!(
                    writable_roots
                        .iter()
                        .filter(|root| **root == expected_memories_root)
                        .count(),
                    1,
                    "expected single writable root entry for {}",
                    expected_memories_root.display()
                );
            }
            other => panic!("expected workspace-write policy, got {other:?}"),
        }
    }

    Ok(())
}

#[tokio::test]
async fn config_defaults_to_file_cli_auth_store_mode() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cfg = ConfigToml::default();

    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        config.cli_auth_credentials_store_mode,
        AuthCredentialsStoreMode::File,
    );

    Ok(())
}

#[tokio::test]
async fn config_resolves_explicit_keyring_auth_store_mode() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cfg = ConfigToml {
        cli_auth_credentials_store: Some(AuthCredentialsStoreMode::Keyring),
        ..Default::default()
    };

    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        config.cli_auth_credentials_store_mode,
        resolve_cli_auth_credentials_store_mode(
            AuthCredentialsStoreMode::Keyring,
            env!("CARGO_PKG_VERSION"),
        ),
    );

    Ok(())
}

#[tokio::test]
async fn config_resolves_default_oauth_store_mode() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cfg = ConfigToml::default();

    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        config.mcp_oauth_credentials_store_mode,
        resolve_mcp_oauth_credentials_store_mode(
            OAuthCredentialsStoreMode::Auto,
            env!("CARGO_PKG_VERSION"),
        ),
    );

    Ok(())
}

#[test]
fn local_dev_builds_force_file_cli_auth_store_modes() {
    assert_eq!(
        resolve_cli_auth_credentials_store_mode(
            AuthCredentialsStoreMode::Keyring,
            LOCAL_DEV_BUILD_VERSION,
        ),
        AuthCredentialsStoreMode::File,
    );
    assert_eq!(
        resolve_cli_auth_credentials_store_mode(
            AuthCredentialsStoreMode::Auto,
            LOCAL_DEV_BUILD_VERSION,
        ),
        AuthCredentialsStoreMode::File,
    );
    assert_eq!(
        resolve_cli_auth_credentials_store_mode(
            AuthCredentialsStoreMode::Ephemeral,
            LOCAL_DEV_BUILD_VERSION,
        ),
        AuthCredentialsStoreMode::Ephemeral,
    );
    assert_eq!(
        resolve_cli_auth_credentials_store_mode(AuthCredentialsStoreMode::Keyring, "1.2.3"),
        AuthCredentialsStoreMode::Keyring,
    );
}

#[test]
fn local_dev_builds_force_file_mcp_oauth_store_modes() {
    assert_eq!(
        resolve_mcp_oauth_credentials_store_mode(
            OAuthCredentialsStoreMode::Keyring,
            LOCAL_DEV_BUILD_VERSION,
        ),
        OAuthCredentialsStoreMode::File,
    );
    assert_eq!(
        resolve_mcp_oauth_credentials_store_mode(
            OAuthCredentialsStoreMode::Auto,
            LOCAL_DEV_BUILD_VERSION,
        ),
        OAuthCredentialsStoreMode::File,
    );
    assert_eq!(
        resolve_mcp_oauth_credentials_store_mode(OAuthCredentialsStoreMode::Keyring, "1.2.3"),
        OAuthCredentialsStoreMode::Keyring,
    );
}

#[tokio::test]
async fn feedback_enabled_defaults_to_true() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cfg = ConfigToml {
        feedback: Some(FeedbackConfigToml::default()),
        ..Default::default()
    };

    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;

    assert_eq!(config.feedback_enabled, true);

    Ok(())
}

#[test]
fn web_search_mode_defaults_to_none_if_unset() {
    let cfg = ConfigToml::default();
    let features = Features::with_defaults();

    assert_eq!(resolve_web_search_mode(&cfg, &features), None);
}

#[test]
fn web_search_mode_prefers_config_over_legacy_flags() {
    let cfg = ConfigToml {
        web_search: Some(WebSearchMode::Live),
        ..Default::default()
    };
    let mut features = Features::with_defaults();
    features.enable(Feature::WebSearchCached);

    assert_eq!(
        resolve_web_search_mode(&cfg, &features),
        Some(WebSearchMode::Live)
    );
}

#[test]
fn web_search_mode_disabled_overrides_legacy_request() {
    let cfg = ConfigToml {
        web_search: Some(WebSearchMode::Disabled),
        ..Default::default()
    };
    let mut features = Features::with_defaults();
    features.enable(Feature::WebSearchRequest);

    assert_eq!(
        resolve_web_search_mode(&cfg, &features),
        Some(WebSearchMode::Disabled)
    );
}

#[test]
fn web_search_mode_for_turn_uses_preference_for_read_only() {
    let web_search_mode = Constrained::allow_any(WebSearchMode::Cached);
    let permission_profile = PermissionProfile::read_only();
    let mode = resolve_web_search_mode_for_turn(&web_search_mode, &permission_profile);

    assert_eq!(mode, WebSearchMode::Cached);
}

#[test]
fn web_search_mode_for_turn_prefers_live_for_disabled_permissions() {
    let web_search_mode = Constrained::allow_any(WebSearchMode::Cached);
    let mode = resolve_web_search_mode_for_turn(&web_search_mode, &PermissionProfile::Disabled);

    assert_eq!(mode, WebSearchMode::Live);
}

#[test]
fn web_search_mode_for_turn_respects_disabled_for_disabled_permissions() {
    let web_search_mode = Constrained::allow_any(WebSearchMode::Disabled);
    let mode = resolve_web_search_mode_for_turn(&web_search_mode, &PermissionProfile::Disabled);

    assert_eq!(mode, WebSearchMode::Disabled);
}

#[test]
fn web_search_mode_for_turn_falls_back_when_live_is_disallowed() -> anyhow::Result<()> {
    let allowed = [WebSearchMode::Disabled, WebSearchMode::Cached];
    let web_search_mode = Constrained::new(WebSearchMode::Cached, move |candidate| {
        if allowed.contains(candidate) {
            Ok(())
        } else {
            Err(ConstraintError::InvalidValue {
                field_name: "web_search_mode",
                candidate: format!("{candidate:?}"),
                allowed: format!("{allowed:?}"),
                requirement_source: RequirementSource::Unknown,
            })
        }
    })?;
    let mode = resolve_web_search_mode_for_turn(&web_search_mode, &PermissionProfile::Disabled);

    assert_eq!(mode, WebSearchMode::Cached);
    Ok(())
}

