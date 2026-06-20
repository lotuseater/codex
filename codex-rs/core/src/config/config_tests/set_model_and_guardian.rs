use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn set_model_updates_defaults() -> anyhow::Result<()> {
    let codex_home = TempDir::new()?;

    ConfigEditsBuilder::new(codex_home.path())
        .set_model(Some("gpt-5.4"), Some(ReasoningEffort::High))
        .apply()
        .await?;

    let serialized = tokio::fs::read_to_string(codex_home.path().join(CONFIG_TOML_FILE)).await?;
    let parsed: ConfigToml = toml::from_str(&serialized)?;

    assert_eq!(parsed.model.as_deref(), Some("gpt-5.4"));
    assert_eq!(parsed.model_reasoning_effort, Some(ReasoningEffort::High));

    Ok(())
}

#[tokio::test]
async fn for_config_writes_selected_user_config_file() -> anyhow::Result<()> {
    let codex_home = TempDir::new()?;
    let base_config = codex_home.path().join(CONFIG_TOML_FILE);
    let selected_config = codex_home.path().join("work.config.toml");
    tokio::fs::write(&base_config, r#"model_provider = "openai""#).await?;
    tokio::fs::write(&selected_config, r#"model = "gpt-old""#).await?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .loader_overrides(LoaderOverrides {
            user_config_path: Some(selected_config.abs()),
            user_config_profile: Some("work".parse().expect("profile-v2 name")),
            ..LoaderOverrides::without_managed_config_for_tests()
        })
        .build()
        .await?;

    ConfigEditsBuilder::for_config(&config)
        .set_model(Some("gpt-new"), Some(ReasoningEffort::High))
        .apply()
        .await?;

    let selected_serialized = tokio::fs::read_to_string(&selected_config).await?;
    let selected: ConfigToml = toml::from_str(&selected_serialized)?;
    assert_eq!(selected.model.as_deref(), Some("gpt-new"));
    assert_eq!(selected.model_reasoning_effort, Some(ReasoningEffort::High));
    assert_eq!(
        tokio::fs::read_to_string(&base_config).await?,
        r#"model_provider = "openai""#
    );

    Ok(())
}

#[test]
fn profile_v2_config_path_resolves_validated_names() -> anyhow::Result<()> {
    let codex_home = TempDir::new()?;
    let profile_name: ProfileV2Name = "work".parse()?;
    assert_eq!(
        resolve_profile_v2_config_path(codex_home.path(), &profile_name),
        codex_home.path().join("work.config.toml").abs()
    );
    Ok(())
}

#[tokio::test]
async fn set_model_overwrites_existing_model() -> anyhow::Result<()> {
    let codex_home = TempDir::new()?;
    let config_path = codex_home.path().join(CONFIG_TOML_FILE);

    tokio::fs::write(
        &config_path,
        r#"
model = "gpt-5.4"
model_reasoning_effort = "medium"

[profiles.dev]
model = "gpt-4.1"
"#,
    )
    .await?;

    ConfigEditsBuilder::new(codex_home.path())
        .set_model(Some("o4-mini"), Some(ReasoningEffort::High))
        .apply()
        .await?;

    let serialized = tokio::fs::read_to_string(config_path).await?;
    let parsed: ConfigToml = toml::from_str(&serialized)?;

    assert_eq!(parsed.model.as_deref(), Some("o4-mini"));
    assert_eq!(parsed.model_reasoning_effort, Some(ReasoningEffort::High));
    assert_eq!(
        parsed
            .profiles
            .get("dev")
            .and_then(|profile| profile.model.as_deref()),
        Some("gpt-4.1"),
    );

    Ok(())
}

#[tokio::test]
async fn cli_override_sets_compact_prompt() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let overrides = ConfigOverrides {
        compact_prompt: Some("Use the compact override".to_string()),
        ..Default::default()
    };

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        overrides,
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        config.compact_prompt.as_deref(),
        Some("Use the compact override")
    );

    Ok(())
}

#[tokio::test]
async fn loads_compact_prompt_from_file() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let workspace = codex_home.path().join("workspace");
    std::fs::create_dir_all(&workspace)?;

    let prompt_path = workspace.join("compact_prompt.txt");
    std::fs::write(&prompt_path, "  summarize differently  ")?;

    let cfg = ConfigToml {
        experimental_compact_prompt_file: Some(prompt_path.abs()),
        ..Default::default()
    };

    let overrides = ConfigOverrides {
        cwd: Some(workspace),
        ..Default::default()
    };

    let config =
        Config::load_from_base_config_with_overrides(cfg, overrides, codex_home.abs()).await?;

    assert_eq!(
        config.compact_prompt.as_deref(),
        Some("summarize differently")
    );

    Ok(())
}

#[tokio::test]
async fn load_config_uses_requirements_guardian_policy_config() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let config_layer_stack = ConfigLayerStack::new(
        Vec::new(),
        Default::default(),
        codex_config::ConfigRequirementsToml {
            guardian_policy_config: Some(
                "  Use the workspace-managed guardian policy.  ".to_string(),
            ),
            ..Default::default()
        },
    )
    .map_err(std::io::Error::other)?;

    let config = Config::load_config_with_layer_stack(
        LOCAL_FS.as_ref(),
        ConfigToml::default(),
        ConfigOverrides {
            cwd: Some(codex_home.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
        config_layer_stack,
    )
    .await?;

    assert_eq!(
        config.guardian_policy_config.as_deref(),
        Some("Use the workspace-managed guardian policy.")
    );

    Ok(())
}

#[test]
fn config_toml_deserializes_auto_review_policy() {
    let cfg = toml::from_str::<ConfigToml>(
        r#"
[auto_review]
policy = "Use the user-configured guardian policy."
"#,
    )
    .expect("TOML deserialization should succeed");

    assert_eq!(
        cfg.auto_review
            .as_ref()
            .and_then(|auto_review| auto_review.policy.as_deref()),
        Some("Use the user-configured guardian policy.")
    );
}

#[tokio::test]
async fn load_config_uses_auto_review_guardian_policy_config() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cfg = ConfigToml {
        auto_review: Some(AutoReviewToml {
            policy: Some("  Use the user-configured guardian policy.  ".to_string()),
        }),
        ..Default::default()
    };

    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides {
            cwd: Some(codex_home.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        config.guardian_policy_config.as_deref(),
        Some("Use the user-configured guardian policy.")
    );

    Ok(())
}

#[tokio::test]
async fn requirements_guardian_policy_beats_auto_review() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let config_layer_stack = ConfigLayerStack::new(
        Vec::new(),
        Default::default(),
        codex_config::ConfigRequirementsToml {
            guardian_policy_config: Some("Use the managed guardian policy.".to_string()),
            ..Default::default()
        },
    )
    .map_err(std::io::Error::other)?;
    let cfg = ConfigToml {
        auto_review: Some(AutoReviewToml {
            policy: Some("Use the user-configured guardian policy.".to_string()),
        }),
        ..Default::default()
    };

    let config = Config::load_config_with_layer_stack(
        LOCAL_FS.as_ref(),
        cfg,
        ConfigOverrides {
            cwd: Some(codex_home.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
        config_layer_stack,
    )
    .await?;

    assert_eq!(
        config.guardian_policy_config.as_deref(),
        Some("Use the managed guardian policy.")
    );

    Ok(())
}

#[tokio::test]
async fn load_config_ignores_empty_auto_review_guardian_policy_config() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let cfg = ConfigToml {
        auto_review: Some(AutoReviewToml {
            policy: Some("   ".to_string()),
        }),
        ..Default::default()
    };

    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides {
            cwd: Some(codex_home.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert_eq!(config.guardian_policy_config, None);

    Ok(())
}

#[tokio::test]
async fn load_config_ignores_empty_requirements_guardian_policy_config() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let config_layer_stack = ConfigLayerStack::new(
        Vec::new(),
        Default::default(),
        codex_config::ConfigRequirementsToml {
            guardian_policy_config: Some("   ".to_string()),
            ..Default::default()
        },
    )
    .map_err(std::io::Error::other)?;

    let config = Config::load_config_with_layer_stack(
        LOCAL_FS.as_ref(),
        ConfigToml::default(),
        ConfigOverrides {
            cwd: Some(codex_home.path().to_path_buf()),
            ..Default::default()
        },
        codex_home.abs(),
        config_layer_stack,
    )
    .await?;

    assert_eq!(config.guardian_policy_config, None);

    Ok(())
}
