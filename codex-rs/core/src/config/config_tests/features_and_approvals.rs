use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn debug_config_lockfile_export_settings_load_from_nested_table() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[debug.config_lockfile]
export_dir = "locks"
allow_codex_version_mismatch = true
save_fields_resolved_from_model_catalog = false
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    assert_eq!(
        config.config_lock_export_dir,
        Some(AbsolutePathBuf::resolve_path_against_base(
            "locks",
            codex_home.path()
        ))
    );
    assert!(config.config_lock_allow_codex_version_mismatch);
    assert!(!config.config_lock_save_fields_resolved_from_model_catalog);

    Ok(())
}

#[tokio::test]
async fn debug_config_lockfile_load_path_loads_lock_from_nested_table() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let lock_path = codex_home.path().join("session.config.lock.toml");
    std::fs::write(
        &lock_path,
        format!(
            r#"version = {}
codex_version = "older-version"

[config]
"#,
            crate::config_lock::CONFIG_LOCK_VERSION
        ),
    )?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        format!(
            r#"[debug.config_lockfile]
load_path = '{}'
allow_codex_version_mismatch = true
save_fields_resolved_from_model_catalog = false
"#,
            lock_path.display()
        ),
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    assert!(config.config_lock_toml.is_some());
    assert!(config.config_lock_allow_codex_version_mismatch);
    assert!(!config.config_lock_save_fields_resolved_from_model_catalog);

    Ok(())
}

#[tokio::test]
async fn explicit_feature_config_is_normalized_by_requirements() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"
[features]
personality = false
shell_tool = true
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .cloud_requirements(CloudRequirementsLoader::new(async {
            Ok(Some(codex_config::ConfigRequirementsToml {
                feature_requirements: Some(codex_config::FeatureRequirementsToml {
                    entries: BTreeMap::from([
                        ("personality".to_string(), true),
                        ("shell_tool".to_string(), false),
                    ]),
                }),
                ..Default::default()
            }))
        }))
        .build()
        .await?;

    assert!(config.features.enabled(Feature::Personality));
    assert!(!config.features.enabled(Feature::ShellTool));
    assert!(
        !config
            .startup_warnings
            .iter()
            .any(|warning| warning.contains("Configured value for `features`")),
        "{:?}",
        config.startup_warnings
    );

    Ok(())
}

#[tokio::test]
async fn approvals_reviewer_defaults_to_manual_only_without_guardian_feature() -> std::io::Result<()>
{
    let codex_home = TempDir::new()?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    assert_eq!(config.approvals_reviewer, ApprovalsReviewer::User);
    Ok(())
}

#[tokio::test]
async fn prompt_instruction_blocks_can_be_disabled_from_config() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"include_permissions_instructions = false
include_apps_instructions = false
include_collaboration_mode_instructions = false
include_environment_context = false

[skills]
include_instructions = false
"#,
    )?;

    let config = ConfigBuilder::default()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    assert!(!config.include_permissions_instructions);
    assert!(!config.include_apps_instructions);
    assert!(!config.include_collaboration_mode_instructions);
    assert!(!config.include_skill_instructions);
    assert!(!config.include_environment_context);
    Ok(())
}

#[tokio::test]
async fn approvals_reviewer_stays_manual_only_when_guardian_feature_is_enabled()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features]
guardian_approval = true
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    assert_eq!(config.approvals_reviewer, ApprovalsReviewer::User);
    Ok(())
}

#[tokio::test]
async fn approvals_reviewer_can_be_set_in_config_without_guardian_approval() -> std::io::Result<()>
{
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"approvals_reviewer = "user"
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    assert_eq!(config.approvals_reviewer, ApprovalsReviewer::User);
    Ok(())
}

#[tokio::test]
async fn requirements_disallowing_default_approvals_reviewer_falls_back_to_required_default()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .cloud_requirements(CloudRequirementsLoader::new(async {
            Ok(Some(codex_config::ConfigRequirementsToml {
                allowed_approvals_reviewers: Some(vec![ApprovalsReviewer::AutoReview]),
                ..Default::default()
            }))
        }))
        .build()
        .await?;

    assert_eq!(config.approvals_reviewer, ApprovalsReviewer::AutoReview);
    Ok(())
}

#[tokio::test]
async fn root_approvals_reviewer_falls_back_when_disallowed_by_requirements() -> std::io::Result<()>
{
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"approvals_reviewer = "user"
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .cloud_requirements(CloudRequirementsLoader::new(async {
            Ok(Some(codex_config::ConfigRequirementsToml {
                allowed_approvals_reviewers: Some(vec![ApprovalsReviewer::AutoReview]),
                ..Default::default()
            }))
        }))
        .build()
        .await?;

    assert_eq!(config.approvals_reviewer, ApprovalsReviewer::AutoReview);
    assert!(
        config.startup_warnings.iter().any(|warning| {
            warning
                .contains("Configured value for `approvals_reviewer` is disallowed by requirements")
        }),
        "{:?}",
        config.startup_warnings
    );
    Ok(())
}

#[tokio::test]
async fn approvals_reviewer_preserves_valid_user_choice_when_allowed_by_requirements()
-> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"approvals_reviewer = "guardian_subagent"
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .cloud_requirements(CloudRequirementsLoader::new(async {
            Ok(Some(codex_config::ConfigRequirementsToml {
                allowed_approvals_reviewers: Some(vec![
                    ApprovalsReviewer::User,
                    ApprovalsReviewer::AutoReview,
                ]),
                ..Default::default()
            }))
        }))
        .build()
        .await?;

    assert_eq!(config.approvals_reviewer, ApprovalsReviewer::AutoReview);
    assert!(
        config
            .startup_warnings
            .iter()
            .all(|warning| !warning.contains("approvals_reviewer")),
        "{:?}",
        config.startup_warnings
    );
    Ok(())
}

#[tokio::test]
async fn smart_approvals_alias_is_ignored() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features]
smart_approvals = true
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    assert!(config.features.enabled(Feature::GuardianApproval));
    assert_eq!(config.approvals_reviewer, ApprovalsReviewer::User);

    let serialized = tokio::fs::read_to_string(codex_home.path().join(CONFIG_TOML_FILE)).await?;
    assert!(serialized.contains("smart_approvals = true"));
    assert!(!serialized.contains("guardian_approval"));
    assert!(!serialized.contains("approvals_reviewer"));

    Ok(())
}

#[tokio::test]
async fn smart_approvals_alias_is_ignored_in_profiles() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let config_toml = r#"[profiles.guardian.features]
smart_approvals = true
"#;
    std::fs::write(codex_home.path().join(CONFIG_TOML_FILE), config_toml)?;

    let cfg: ConfigToml = toml::from_str(config_toml).expect("TOML deserialization should succeed");
    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides {
            config_profile: Some("guardian".to_string()),
            cwd: Some(codex_home.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert!(config.features.enabled(Feature::GuardianApproval));
    assert_eq!(config.approvals_reviewer, ApprovalsReviewer::User);

    let serialized = tokio::fs::read_to_string(codex_home.path().join(CONFIG_TOML_FILE)).await?;
    assert!(serialized.contains("[profiles.guardian.features]"));
    assert!(serialized.contains("smart_approvals = true"));
    assert!(!serialized.contains("guardian_approval"));
    assert!(!serialized.contains("approvals_reviewer"));

    Ok(())
}
