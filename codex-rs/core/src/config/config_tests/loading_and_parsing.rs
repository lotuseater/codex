use super::*;

#[tokio::test]
async fn load_config_normalizes_relative_cwd_override() -> std::io::Result<()> {
    let expected_cwd = AbsolutePathBuf::relative_to_current_dir("nested")?;
    let codex_home = tempdir()?;
    let config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides {
            cwd: Some(PathBuf::from("nested")),
            ..Default::default()
        },
        codex_home.abs(),
    )
    .await?;

    assert_eq!(config.cwd, expected_cwd);
    Ok(())
}

#[tokio::test]
async fn load_config_resolves_batch_mini_programming_instructions() -> std::io::Result<()> {
    let codex_home = tempdir()?;
    let default_config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        default_config.batch_mini_programming_instructions.mode,
        BatchMiniProgrammingInstructionsMode::Off
    );

    let enabled_config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            batch_mini_programming_instructions: Some(BatchMiniProgrammingInstructionsToml {
                mode: Some(BatchMiniProgrammingInstructionsModeToml::Always),
            }),
            ..Default::default()
        },
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        enabled_config.batch_mini_programming_instructions.mode,
        BatchMiniProgrammingInstructionsMode::Always
    );

    Ok(())
}

#[tokio::test]
async fn load_config_resolves_model_compact_percentage() -> std::io::Result<()> {
    let codex_home = tempdir()?;
    let default_config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        default_config.model_compact_percentage,
        DEFAULT_TRIGGER_CONTEXT_PERCENT
    );

    let codex_home = tempdir()?;
    let configured = Config::load_from_base_config_with_overrides(
        ConfigToml {
            model_compact_percentage: Some(35),
            ..Default::default()
        },
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;

    assert_eq!(configured.model_compact_percentage, 35);
    Ok(())
}

#[tokio::test]
async fn load_config_warns_and_defaults_invalid_model_compact_percentage() -> std::io::Result<()> {
    let codex_home = tempdir()?;
    let config = Config::load_from_base_config_with_overrides(
        ConfigToml {
            model_compact_percentage: Some(101),
            ..Default::default()
        },
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        config.model_compact_percentage,
        DEFAULT_TRIGGER_CONTEXT_PERCENT
    );
    assert!(
        config
            .startup_warnings
            .iter()
            .any(|warning| warning.contains("model_compact_percentage"))
    );
    Ok(())
}

#[tokio::test]
async fn load_config_loads_global_agents_instructions() -> std::io::Result<()> {
    let codex_home = tempdir()?;
    std::fs::write(
        codex_home.path().join(DEFAULT_AGENTS_MD_FILENAME),
        "\n  global instructions  \n",
    )?;

    let mut config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;
    let _ = config.features.enable(Feature::MemoryTool);

    assert_eq!(
        config.user_instructions.as_deref(),
        Some("global instructions")
    );
    Ok(())
}

#[tokio::test]
async fn load_config_prefers_global_agents_override_instructions() -> std::io::Result<()> {
    let codex_home = tempdir()?;
    std::fs::write(
        codex_home.path().join(DEFAULT_AGENTS_MD_FILENAME),
        "global instructions",
    )?;
    let global_agents_override_path = codex_home.path().join(LOCAL_AGENTS_MD_FILENAME);
    std::fs::write(&global_agents_override_path, "local override instructions")?;

    let config = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        codex_home.abs(),
    )
    .await?;

    assert_eq!(
        config.user_instructions.as_deref(),
        Some("local override instructions")
    );
    Ok(())
}

#[tokio::test]
async fn test_toml_parsing() {
    let history_with_persistence = r#"
[history]
persistence = "save-all"
"#;
    let history_with_persistence_cfg = toml::from_str::<ConfigToml>(history_with_persistence)
        .expect("TOML deserialization should succeed");
    assert_eq!(
        Some(History {
            persistence: HistoryPersistence::SaveAll,
            max_bytes: None,
        }),
        history_with_persistence_cfg.history
    );

    let history_no_persistence = r#"
[history]
persistence = "none"
"#;

    let history_no_persistence_cfg = toml::from_str::<ConfigToml>(history_no_persistence)
        .expect("TOML deserialization should succeed");
    assert_eq!(
        Some(History {
            persistence: HistoryPersistence::None,
            max_bytes: None,
        }),
        history_no_persistence_cfg.history
    );

    let memories = r#"
[memories]
disable_on_external_context = true
generate_memories = false
use_memories = false
max_raw_memories_for_consolidation = 512
max_unused_days = 21
max_rollout_age_days = 42
max_rollouts_per_startup = 9
min_rollout_idle_hours = 24
min_rate_limit_remaining_percent = 12
extract_model = "gpt-5-mini"
consolidation_model = "gpt-5.2"
"#;
    let memories_cfg =
        toml::from_str::<ConfigToml>(memories).expect("TOML deserialization should succeed");
    assert_eq!(
        Some(MemoriesToml {
            disable_on_external_context: Some(true),
            generate_memories: Some(false),
            use_memories: Some(false),
            max_raw_memories_for_consolidation: Some(512),
            max_unused_days: Some(21),
            max_rollout_age_days: Some(42),
            max_rollouts_per_startup: Some(9),
            min_rollout_idle_hours: Some(24),
            min_rate_limit_remaining_percent: Some(12),
            extract_model: Some("gpt-5-mini".to_string()),
            consolidation_model: Some("gpt-5.2".to_string()),
            project_problem_index: None,
            project_problem_context: None,
            project_problem_max_matches: None,
        }),
        memories_cfg.memories
    );

    let config = Config::load_from_base_config_with_overrides(
        memories_cfg,
        ConfigOverrides::default(),
        tempdir().expect("tempdir").abs(),
    )
    .await
    .expect("load config from memories settings");
    assert_eq!(
        config.memories,
        MemoriesConfig {
            disable_on_external_context: true,
            generate_memories: false,
            use_memories: false,
            max_raw_memories_for_consolidation: 512,
            max_unused_days: 21,
            max_rollout_age_days: 42,
            max_rollouts_per_startup: 9,
            min_rollout_idle_hours: 24,
            min_rate_limit_remaining_percent: 12,
            extract_model: Some("gpt-5-mini".to_string()),
            consolidation_model: Some("gpt-5.2".to_string()),
            project_problem_index: true,
            project_problem_context: true,
            project_problem_max_matches: 3,
        }
    );

    let legacy_memories_cfg =
        toml::from_str::<ConfigToml>("[memories]\nno_memories_if_mcp_or_web_search = true\n")
            .expect("legacy memories TOML should deserialize");
    assert!(
        MemoriesConfig::from(
            legacy_memories_cfg
                .memories
                .expect("legacy memories config")
        )
        .disable_on_external_context
    );
}

#[test]
fn parses_bundled_skills_config() {
    let cfg: ConfigToml = toml::from_str(
        r#"
[skills]
include_instructions = false

[skills.bundled]
enabled = false
"#,
    )
    .expect("TOML deserialization should succeed");

    assert_eq!(
        cfg.skills,
        Some(SkillsConfig {
            bundled: Some(BundledSkillsConfig { enabled: false }),
            include_instructions: Some(false),
            config: Vec::new(),
        })
    );
}

#[test]
fn tools_web_search_true_deserializes_to_none() {
    let cfg: ConfigToml = toml::from_str(
        r#"
[tools]
web_search = true
"#,
    )
    .expect("TOML deserialization should succeed");

    assert_eq!(cfg.tools, Some(ToolsToml { web_search: None }));
}

#[test]
fn tools_web_search_false_deserializes_to_none() {
    let cfg: ConfigToml = toml::from_str(
        r#"
[tools]
web_search = false
"#,
    )
    .expect("TOML deserialization should succeed");

    assert_eq!(cfg.tools, Some(ToolsToml { web_search: None }));
}

#[test]
fn desktop_automation_config_deserializes() {
    let cfg: ConfigToml = toml::from_str(
        r#"
[desktop_automation]
enabled = false
proactive = false
allow_input = false
prefer_app_harness = true
"#,
    )
    .expect("TOML deserialization should succeed");

    let config = cfg.desktop_automation.expect("desktop automation config");
    assert_eq!(config.enabled, Some(false));
    assert_eq!(config.proactive, Some(false));
    assert_eq!(config.allow_input, Some(false));
    assert_eq!(config.prefer_app_harness, Some(true));
}

#[test]
fn first_moves_config_deserializes() {
    let cfg: ConfigToml = toml::from_str(
        r#"
[first_moves]
enabled = true
mode = "suggest_only"
inject_context = false
prewarm = "off"
max_candidates = 9
max_context_moves = 4
max_prewarm_files = 1
min_context_score = 0.6
min_prewarm_score = 0.9
max_scan_files = 120
max_scan_depth = 3
max_read_bytes = 2048
"#,
    )
    .expect("TOML deserialization should succeed");

    let config = cfg.first_moves.expect("first moves config");
    assert_eq!(config.enabled, Some(true));
    assert_eq!(config.mode, Some(FirstMovesModeToml::SuggestOnly));
    assert_eq!(config.inject_context, Some(false));
    assert_eq!(config.prewarm, Some(FirstMovesPrewarmToml::Off));
    assert_eq!(config.max_candidates, Some(9));
    assert_eq!(config.max_context_moves, Some(4));
    assert_eq!(config.max_prewarm_files, Some(1));
    assert_eq!(config.min_context_score, Some(0.6));
    assert_eq!(config.min_prewarm_score, Some(0.9));
    assert_eq!(config.max_scan_files, Some(120));
    assert_eq!(config.max_scan_depth, Some(3));
    assert_eq!(config.max_read_bytes, Some(2048));
}

#[test]
fn repo_context_scout_config_deserializes() {
    let cfg: ConfigToml = toml::from_str(
        r#"
[repo_context_scout]
mode = "tool"
max_files = 123
max_file_bytes = 4096
max_anchors_per_file = 7
max_output_tokens = 900
max_candidates = 6
"#,
    )
    .expect("TOML deserialization should succeed");

    let config = cfg.repo_context_scout.expect("repo context scout config");
    assert_eq!(config.mode, Some(RepoContextScoutModeToml::Tool));
    assert_eq!(config.max_files, Some(123));
    assert_eq!(config.max_file_bytes, Some(4096));
    assert_eq!(config.max_anchors_per_file, Some(7));
    assert_eq!(config.max_output_tokens, Some(900));
    assert_eq!(config.max_candidates, Some(6));
}
