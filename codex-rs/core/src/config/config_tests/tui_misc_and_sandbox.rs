use super::common::*;
use super::*;
use pretty_assertions::assert_eq;

#[test]
fn tui_theme_deserializes_from_toml() {
    let cfg = r#"
[tui]
theme = "dracula"
"#;
    let parsed = toml::from_str::<ConfigToml>(cfg).expect("TOML deserialization should succeed");
    assert_eq!(
        parsed.tui.as_ref().and_then(|t| t.theme.as_deref()),
        Some("dracula"),
    );
}

#[test]
fn tui_theme_defaults_to_none() {
    let cfg = r#"
[tui]
"#;
    let parsed = toml::from_str::<ConfigToml>(cfg).expect("TOML deserialization should succeed");
    assert_eq!(parsed.tui.as_ref().and_then(|t| t.theme.as_deref()), None);
}

#[test]
fn tui_session_picker_view_deserializes_from_toml() {
    let cfg = r#"
[tui]
session_picker_view = "dense"
"#;
    let parsed = toml::from_str::<ConfigToml>(cfg).expect("TOML deserialization should succeed");
    assert_eq!(
        parsed.tui.as_ref().and_then(|t| t.session_picker_view),
        Some(SessionPickerViewMode::Dense),
    );
}

#[test]
fn tui_pet_deserializes_from_toml() {
    let cfg = r#"
[tui]
pet = "chefito"
"#;
    let parsed = toml::from_str::<ConfigToml>(cfg).expect("TOML deserialization should succeed");
    assert_eq!(
        parsed.tui.as_ref().and_then(|t| t.pet.as_deref()),
        Some("chefito"),
    );
}

#[test]
fn tui_session_picker_view_defaults_to_none() {
    let cfg = r#"
[tui]
"#;
    let parsed = toml::from_str::<ConfigToml>(cfg).expect("TOML deserialization should succeed");
    assert_eq!(
        parsed.tui.as_ref().and_then(|t| t.session_picker_view),
        None,
    );
}

#[test]
fn tui_pet_defaults_to_none() {
    let cfg = r#"
[tui]
"#;
    let parsed = toml::from_str::<ConfigToml>(cfg).expect("TOML deserialization should succeed");
    assert_eq!(parsed.tui.as_ref().and_then(|t| t.pet.as_deref()), None);
}

#[test]
fn tui_pet_anchor_deserializes_from_toml() {
    let cfg = r#"
[tui]
pet_anchor = "screen-bottom"
"#;
    let parsed = toml::from_str::<ConfigToml>(cfg).expect("TOML deserialization should succeed");
    assert_eq!(
        parsed.tui.as_ref().map(|t| t.pet_anchor),
        Some(TuiPetAnchor::ScreenBottom),
    );
}

#[test]
fn tui_pet_anchor_defaults_to_composer() {
    let cfg = r#"
[tui]
"#;
    let parsed = toml::from_str::<ConfigToml>(cfg).expect("TOML deserialization should succeed");
    assert_eq!(
        parsed.tui.as_ref().map(|t| t.pet_anchor),
        Some(TuiPetAnchor::Composer),
    );
}

#[test]
fn tui_pet_anchor_rejects_unknown_value() {
    let cfg = r#"
[tui]
pet_anchor = "bottom"
"#;
    let err = toml::from_str::<ConfigToml>(cfg).expect_err("reject unknown pet anchor");
    let err = err.to_string();
    assert!(
        err.contains("unknown variant `bottom`")
            && err.contains("composer")
            && err.contains("screen-bottom"),
        "unexpected error: {err}"
    );
}

#[test]
fn tui_config_missing_notifications_field_defaults_to_enabled() {
    let cfg = r#"
[tui]
"#;

    let parsed =
        toml::from_str::<ConfigToml>(cfg).expect("TUI config without notifications should succeed");
    let tui = parsed.tui.expect("config should include tui section");

    assert_eq!(
        tui,
        Tui {
            notification_settings: TuiNotificationSettings::default(),
            animations: true,
            show_tooltips: true,
            vim_mode_default: false,
            raw_output_mode: false,
            alternate_screen: AltScreenMode::Auto,
            status_line: None,
            status_line_use_colors: true,
            terminal_title: None,
            theme: None,
            pet: None,
            pet_anchor: TuiPetAnchor::Composer,
            session_picker_view: None,
            keymap: TuiKeymap::default(),
            model_availability_nux: ModelAvailabilityNuxConfig::default(),
            terminal_resize_reflow_max_rows: None,
        }
    );
}

#[tokio::test]
async fn runtime_config_resolves_terminal_resize_reflow_defaults_and_overrides() {
    let cfg = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        tempdir().expect("tempdir").abs(),
    )
    .await
    .expect("load default config");

    assert_eq!(
        cfg.terminal_resize_reflow,
        TerminalResizeReflowConfig::default()
    );
    assert_eq!(
        cfg.terminal_resize_reflow.max_rows,
        TerminalResizeReflowMaxRows::Auto
    );

    let cfg = Config::load_from_base_config_with_overrides(
        ConfigToml {
            tui: Some(Tui {
                terminal_resize_reflow_max_rows: Some(9000),
                ..Default::default()
            }),
            ..Default::default()
        },
        ConfigOverrides::default(),
        tempdir().expect("tempdir").abs(),
    )
    .await
    .expect("load overridden config");

    assert_eq!(
        cfg.terminal_resize_reflow.max_rows,
        TerminalResizeReflowMaxRows::Limit(9000)
    );

    let cfg = Config::load_from_base_config_with_overrides(
        ConfigToml {
            tui: Some(Tui {
                terminal_resize_reflow_max_rows: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        },
        ConfigOverrides::default(),
        tempdir().expect("tempdir").abs(),
    )
    .await
    .expect("load config with disabled resize reflow limits");

    assert_eq!(
        cfg.terminal_resize_reflow.max_rows,
        TerminalResizeReflowMaxRows::Disabled
    );
}

#[tokio::test]
async fn forced_chatgpt_workspace_id_empty_values_disable_runtime_restriction()
-> std::io::Result<()> {
    let cases: Vec<(&str, &str, Option<Vec<&str>>)> = vec![
        ("unset", "", None),
        ("empty string", r#"forced_chatgpt_workspace_id = """#, None),
        (
            "whitespace string",
            r#"forced_chatgpt_workspace_id = "   ""#,
            None,
        ),
        ("empty list", r#"forced_chatgpt_workspace_id = []"#, None),
        (
            "blank list entries",
            r#"forced_chatgpt_workspace_id = ["", "  "]"#,
            None,
        ),
        (
            "mixed list entries",
            r#"forced_chatgpt_workspace_id = ["", " 123e4567-e89b-42d3-a456-426614174000 ", "123e4567-e89b-42d3-a456-426614174001"]"#,
            Some(vec![
                "123e4567-e89b-42d3-a456-426614174000",
                "123e4567-e89b-42d3-a456-426614174001",
            ]),
        ),
    ];

    for (name, toml, expected) in cases {
        let cfg_toml: ConfigToml = toml::from_str(toml)
            .unwrap_or_else(|err| panic!("{name} should parse forced_chatgpt_workspace_id: {err}"));
        let config = Config::load_from_base_config_with_overrides(
            cfg_toml,
            ConfigOverrides::default(),
            tempdir().expect("tempdir").abs(),
        )
        .await?;

        let expected = expected.map(|values| {
            values
                .into_iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        });
        assert_eq!(config.forced_chatgpt_workspace_id, expected, "{name}");
    }

    Ok(())
}

#[tokio::test]
async fn legacy_remote_thread_store_endpoint_is_rejected() {
    let cfg: ConfigToml =
        toml::from_str(r#"experimental_thread_store_endpoint = "https://example.com""#)
            .expect("legacy remote thread-store endpoint should still deserialize");

    let err = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides::default(),
        tempdir().expect("tempdir").abs(),
    )
    .await
    .expect_err("legacy remote thread-store endpoint should be rejected at load time");

    assert!(
        err.to_string()
            .contains("experimental_thread_store_endpoint")
    );
    assert!(err.to_string().contains("no longer supported"));
}

#[test]
fn profile_tui_rejects_unsupported_settings() {
    let err = toml::from_str::<ConfigToml>(
        r#"profile = "work"

[profiles.work.tui]
theme = "dark"
"#,
    )
    .expect_err("profile TUI config should only accept supported fields");

    assert!(err.to_string().contains("unknown field"));
    assert!(err.to_string().contains("theme"));
}

#[tokio::test]
async fn runtime_config_resolves_session_picker_view_default_and_override() {
    let cfg = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        tempdir().expect("tempdir").abs(),
    )
    .await
    .expect("load default config");

    assert_eq!(cfg.tui_session_picker_view, SessionPickerViewMode::Dense);

    let cfg = Config::load_from_base_config_with_overrides(
        ConfigToml {
            tui: Some(Tui {
                session_picker_view: Some(SessionPickerViewMode::Comfortable),
                ..Default::default()
            }),
            ..Default::default()
        },
        ConfigOverrides::default(),
        tempdir().expect("tempdir").abs(),
    )
    .await
    .expect("load root override config");

    assert_eq!(
        cfg.tui_session_picker_view,
        SessionPickerViewMode::Comfortable
    );
}

#[tokio::test]
async fn test_sandbox_config_parsing() {
    let sandbox_full_access = r#"
sandbox_mode = "danger-full-access"

[sandbox_workspace_write]
network_access = false  # This should be ignored.
"#;
    let sandbox_full_access_cfg = toml::from_str::<ConfigToml>(sandbox_full_access)
        .expect("TOML deserialization should succeed");
    let sandbox_mode_override = None;
    let resolution = derive_legacy_sandbox_policy_for_test(
        &sandbox_full_access_cfg,
        sandbox_mode_override,
        /*profile_sandbox_mode*/ None,
        WindowsSandboxLevel::Disabled,
        /*active_project*/ None,
        /*permission_profile_constraint*/ None,
    )
    .await;
    assert_eq!(resolution, SandboxPolicy::DangerFullAccess);

    let sandbox_read_only = r#"
sandbox_mode = "read-only"

[sandbox_workspace_write]
network_access = true  # This should be ignored.
"#;

    let sandbox_read_only_cfg = toml::from_str::<ConfigToml>(sandbox_read_only)
        .expect("TOML deserialization should succeed");
    let sandbox_mode_override = None;
    let resolution = derive_legacy_sandbox_policy_for_test(
        &sandbox_read_only_cfg,
        sandbox_mode_override,
        /*profile_sandbox_mode*/ None,
        WindowsSandboxLevel::Disabled,
        /*active_project*/ None,
        /*permission_profile_constraint*/ None,
    )
    .await;
    assert_eq!(resolution, SandboxPolicy::new_read_only_policy());

    let writable_root = test_absolute_path("/my/workspace");
    let sandbox_workspace_write = format!(
        r#"
sandbox_mode = "workspace-write"

[sandbox_workspace_write]
writable_roots = [
    {},
]
exclude_tmpdir_env_var = true
exclude_slash_tmp = true

[projects."/tmp/test"]
trust_level = "trusted"
"#,
        serde_json::json!(writable_root)
    );

    let sandbox_workspace_write_cfg = toml::from_str::<ConfigToml>(&sandbox_workspace_write)
        .expect("TOML deserialization should succeed");
    let sandbox_mode_override = None;
    let resolution = derive_legacy_sandbox_policy_for_test(
        &sandbox_workspace_write_cfg,
        sandbox_mode_override,
        /*profile_sandbox_mode*/ None,
        WindowsSandboxLevel::Disabled,
        /*active_project*/ None,
        /*permission_profile_constraint*/ None,
    )
    .await;
    if cfg!(target_os = "windows") {
        assert_eq!(resolution, SandboxPolicy::new_read_only_policy());
    } else {
        assert_eq!(
            resolution,
            SandboxPolicy::WorkspaceWrite {
                writable_roots: vec![writable_root.clone()],
                network_access: false,
                exclude_tmpdir_env_var: true,
                exclude_slash_tmp: true,
            }
        );
    }

    let sandbox_workspace_write = format!(
        r#"
sandbox_mode = "workspace-write"

[sandbox_workspace_write]
writable_roots = [
    {},
]
exclude_tmpdir_env_var = true
exclude_slash_tmp = true
"#,
        serde_json::json!(writable_root)
    );

    let sandbox_workspace_write_cfg = toml::from_str::<ConfigToml>(&sandbox_workspace_write)
        .expect("TOML deserialization should succeed");
    let sandbox_mode_override = None;
    let resolution = derive_legacy_sandbox_policy_for_test(
        &sandbox_workspace_write_cfg,
        sandbox_mode_override,
        /*profile_sandbox_mode*/ None,
        WindowsSandboxLevel::Disabled,
        /*active_project*/ None,
        /*permission_profile_constraint*/ None,
    )
    .await;
    if cfg!(target_os = "windows") {
        assert_eq!(resolution, SandboxPolicy::new_read_only_policy());
    } else {
        assert_eq!(
            resolution,
            SandboxPolicy::WorkspaceWrite {
                writable_roots: vec![writable_root],
                network_access: false,
                exclude_tmpdir_env_var: true,
                exclude_slash_tmp: true,
            }
        );
    }
}

#[tokio::test]
async fn legacy_sandbox_mode_builds_profiles_with_compatible_projection() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cwd = TempDir::new()?;
    let extra_root = test_absolute_path("/tmp/legacy-extra-root");
    let cases = vec![
        (
            "danger-full-access".to_string(),
            r#"sandbox_mode = "danger-full-access"
"#
            .to_string(),
        ),
        (
            "read-only".to_string(),
            r#"sandbox_mode = "read-only"
"#
            .to_string(),
        ),
        (
            "workspace-write".to_string(),
            format!(
                r#"sandbox_mode = "workspace-write"

[sandbox_workspace_write]
writable_roots = [{}]
exclude_tmpdir_env_var = true
exclude_slash_tmp = true
"#,
                serde_json::json!(extra_root)
            ),
        ),
    ];

    for (name, config_toml) in cases {
        let cfg = toml::from_str::<ConfigToml>(&config_toml)
            .unwrap_or_else(|err| panic!("case `{name}` should parse: {err}"));
        let config = Config::load_from_base_config_with_overrides(
            cfg,
            ConfigOverrides {
                cwd: Some(cwd.path().to_path_buf()),
                ..Default::default()
            },
            codex_home.abs(),
        )
        .await?;

        let sandbox_policy = config.legacy_sandbox_policy();
        let file_system_policy = config.permissions.file_system_sandbox_policy();
        let network_policy = config.permissions.network_sandbox_policy();

        assert_eq!(
            network_policy,
            NetworkSandboxPolicy::from(&sandbox_policy),
            "case `{name}` should preserve network semantics from legacy config"
        );
        assert_eq!(
            file_system_policy
                .to_legacy_sandbox_policy(network_policy, cwd.path())
                .unwrap_or_else(|err| panic!("case `{name}` should round-trip: {err}")),
            sandbox_policy,
            "case `{name}` should preserve its legacy compatibility projection"
        );

        match name.as_str() {
            "danger-full-access" | "read-only" => {
                assert_eq!(
                    file_system_policy,
                    FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(
                        &sandbox_policy,
                        cwd.path()
                    ),
                    "case `{name}` should match the legacy filesystem projection exactly"
                );
            }
            "workspace-write" => {
                if cfg!(target_os = "windows") {
                    assert_eq!(
                        sandbox_policy,
                        SandboxPolicy::new_read_only_policy(),
                        "legacy workspace-write should keep the existing Windows downgrade when \
                         the experimental Windows sandbox is disabled"
                    );
                    assert_eq!(
                        file_system_policy,
                        FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(
                            &sandbox_policy,
                            cwd.path()
                        ),
                        "downgraded workspace-write should match the legacy read-only projection"
                    );
                    continue;
                }
                assert_eq!(
                    config.permissions.workspace_roots(),
                    &[cwd.abs(), extra_root.clone()]
                );
                assert!(
                    file_system_policy
                        .entries
                        .contains(&FileSystemSandboxEntry {
                            path: FileSystemPath::Path { path: cwd.abs() },
                            access: FileSystemAccessMode::Write,
                        })
                );
                assert!(
                    file_system_policy
                        .entries
                        .contains(&FileSystemSandboxEntry {
                            path: FileSystemPath::Path {
                                path: extra_root.clone(),
                            },
                            access: FileSystemAccessMode::Write,
                        })
                );
                for subpath in [".git", ".agents", ".codex"] {
                    assert!(
                        file_system_policy
                            .entries
                            .contains(&FileSystemSandboxEntry {
                                path: FileSystemPath::Path {
                                    path: AbsolutePathBuf::resolve_path_against_base(
                                        subpath,
                                        cwd.path()
                                    ),
                                },
                                access: FileSystemAccessMode::Read,
                            }),
                        "case `{name}` should materialize `{subpath}` for the runtime workspace \
                         root"
                    );
                }
            }
            _ => unreachable!("unexpected test case `{name}`"),
        }
    }

    Ok(())
}
