use super::*;

#[test]
fn multi_agent_v2_default_hints_gate_exploration_with_first_moves() {
    let config = MultiAgentV2Config::default();
    let root_hint = config
        .root_agent_usage_hint_text
        .as_deref()
        .expect("root hint should be configured by default");
    let subagent_hint = config
        .subagent_usage_hint_text
        .as_deref()
        .expect("subagent hint should be configured by default");

    assert!(root_hint.contains("first_moves_predict"));
    assert!(root_hint.contains("Agent ROI Estimate"));
    assert!(root_hint.contains("what to delegate to subagents"));
    assert!(root_hint.contains("up to three persistent high-capability helpers"));
    assert!(root_hint.contains("Only the main/root agent spawns helpers"));
    assert!(root_hint.contains("Compact helpers after bulky reads"));
    assert!(root_hint.contains("short summary or short result only when the main agent needs"));
    assert!(root_hint.contains("net >= 2"));
    assert!(root_hint.contains("reuse_cost=1"));
    assert!(root_hint.contains("loop_followup_gain=0-3"));
    assert!(root_hint.contains("automatic continuation is normally 2"));
    assert!(root_hint.contains("implementation prompt may be accepted automatically"));
    assert!(root_hint.contains("stable `helper` agent task name"));
    assert!(root_hint.contains("Spawn a fresh helper only when reuse is unavailable"));
    assert!(root_hint.contains("git commit/push/tag/rebase/merge"));
    assert!(root_hint.contains("Do not spawn an agent just to do a broad opening survey"));
    assert!(root_hint.contains("SCOUT_EVIDENCE"));
    assert!(root_hint.contains("WHY_AGENT / ROI"));
    assert!(root_hint.contains("do not call first_moves_predict"));
    assert!(root_hint.contains("plan-completion self-review"));
    assert!(root_hint.contains("raw `rg` search"));
    assert!(root_hint.contains("weaker model can be less token-effective"));
    assert!(root_hint.contains("active loop iterations"));
    assert!(subagent_hint.contains("first_moves_predict"));
    assert!(subagent_hint.contains("SCOUT_EVIDENCE"));
    assert!(subagent_hint.contains("WHY_AGENT / ROI"));
    assert!(subagent_hint.contains("skip first_moves_predict"));
    assert!(subagent_hint.contains("If you are a `helper` agent"));
    assert!(subagent_hint.contains("repo_context_scout"));
    assert!(subagent_hint.contains("Root owns finalization"));
    assert!(subagent_hint.contains("Do not spawn more agents"));
    assert!(subagent_hint.contains("A short summary or short result is optional"));
    assert!(subagent_hint.contains("configured tools, skills, MCP/app surfaces"));
}

#[tokio::test]
async fn multi_agent_v2_config_from_feature_table() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
max_concurrent_threads_per_session = 5
min_wait_timeout_ms = 2500
max_wait_timeout_ms = 120000
default_wait_timeout_ms = 30000
usage_hint_enabled = false
usage_hint_text = "Custom delegation guidance."
root_agent_usage_hint_text = "Root guidance."
subagent_usage_hint_text = "Subagent guidance."
tool_namespace = "agents"
hide_spawn_agent_metadata = true
non_code_mode_only = true
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    assert!(config.features.enabled(Feature::MultiAgentV2));
    assert_eq!(config.multi_agent_v2.max_concurrent_threads_per_session, 5);
    assert_eq!(config.multi_agent_v2.min_wait_timeout_ms, 2500);
    assert_eq!(config.multi_agent_v2.max_wait_timeout_ms, 120000);
    assert_eq!(config.multi_agent_v2.default_wait_timeout_ms, 30000);
    assert_eq!(config.agent_max_threads, None);
    assert_eq!(
        config.effective_agent_max_threads(codex_protocol::protocol::MultiAgentVersion::V2)?,
        Some(4)
    );
    assert!(!config.multi_agent_v2.usage_hint_enabled);
    assert_eq!(
        config.multi_agent_v2.usage_hint_text.as_deref(),
        Some("Custom delegation guidance.")
    );
    assert_eq!(
        config.multi_agent_v2.root_agent_usage_hint_text.as_deref(),
        Some("Root guidance.")
    );
    assert_eq!(
        config.multi_agent_v2.subagent_usage_hint_text.as_deref(),
        Some("Subagent guidance.")
    );
    assert_eq!(
        config.multi_agent_v2.tool_namespace.as_deref(),
        Some("agents")
    );
    assert!(config.multi_agent_v2.hide_spawn_agent_metadata);
    assert!(config.multi_agent_v2.non_code_mode_only);

    Ok(())
}

#[tokio::test]
async fn multi_agent_v2_default_session_thread_cap_counts_root() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    assert_eq!(config.multi_agent_v2.max_concurrent_threads_per_session, 4);
    assert_eq!(config.multi_agent_v2.min_wait_timeout_ms, 10_000);
    assert_eq!(config.multi_agent_v2.max_wait_timeout_ms, 3_600_000);
    assert_eq!(config.multi_agent_v2.default_wait_timeout_ms, 30_000);
    assert_eq!(config.agent_max_threads, None);
    assert_eq!(
        config.effective_agent_max_threads(codex_protocol::protocol::MultiAgentVersion::V2)?,
        Some(3)
    );
    assert!(!config.multi_agent_v2.non_code_mode_only);

    Ok(())
}

#[tokio::test]
async fn multi_agent_v2_rejects_agents_max_threads() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true

[agents]
max_threads = 3
"#,
    )?;

    let err = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await
        .expect_err("agents.max_threads should conflict with multi_agent_v2");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "agents.max_threads cannot be set when multi_agent_v2 is enabled"
    );

    Ok(())
}

#[tokio::test]
async fn runtime_selected_multi_agent_v2_ignores_legacy_agent_max_threads() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    let mut config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    config.agent_max_threads = Some(7);
    config.multi_agent_v2.max_concurrent_threads_per_session = 6;

    assert_eq!(
        config.effective_agent_max_threads(codex_protocol::protocol::MultiAgentVersion::V2)?,
        Some(5)
    );
    assert_eq!(
        config.effective_agent_max_threads(codex_protocol::protocol::MultiAgentVersion::V1)?,
        Some(7)
    );

    Ok(())
}

#[tokio::test]
async fn multi_agent_v2_rejects_invalid_wait_timeouts() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
min_wait_timeout_ms = 0
max_wait_timeout_ms = 0
default_wait_timeout_ms = 0
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    assert_eq!(config.multi_agent_v2.min_wait_timeout_ms, 0);
    assert_eq!(config.multi_agent_v2.max_wait_timeout_ms, 0);
    assert_eq!(config.multi_agent_v2.default_wait_timeout_ms, 0);

    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
min_wait_timeout_ms = -1
"#,
    )?;

    let err = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await
        .expect_err("negative min_wait_timeout_ms should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "features.multi_agent_v2.min_wait_timeout_ms must be at least 0"
    );

    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
min_wait_timeout_ms = 3600001
"#,
    )?;

    let err = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await
        .expect_err("too large min_wait_timeout_ms should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "features.multi_agent_v2.min_wait_timeout_ms must be at most 3600000"
    );

    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
max_wait_timeout_ms = -1
"#,
    )?;

    let err = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await
        .expect_err("negative max_wait_timeout_ms should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "features.multi_agent_v2.max_wait_timeout_ms must be at least 0"
    );

    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
max_wait_timeout_ms = 3600001
"#,
    )?;

    let err = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await
        .expect_err("too large max_wait_timeout_ms should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "features.multi_agent_v2.max_wait_timeout_ms must be at most 3600000"
    );

    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
default_wait_timeout_ms = -1
"#,
    )?;

    let err = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await
        .expect_err("negative default_wait_timeout_ms should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "features.multi_agent_v2.default_wait_timeout_ms must be at least 0"
    );

    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
min_wait_timeout_ms = 1000
max_wait_timeout_ms = 500
"#,
    )?;

    let err = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await
        .expect_err("min greater than max should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "features.multi_agent_v2.min_wait_timeout_ms must be at most features.multi_agent_v2.max_wait_timeout_ms"
    );

    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
min_wait_timeout_ms = 1000
max_wait_timeout_ms = 2000
default_wait_timeout_ms = 500
"#,
    )?;

    let err = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await
        .expect_err("default less than min should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "features.multi_agent_v2.default_wait_timeout_ms must be at least features.multi_agent_v2.min_wait_timeout_ms"
    );

    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
min_wait_timeout_ms = 1000
max_wait_timeout_ms = 2000
default_wait_timeout_ms = 2500
"#,
    )?;

    let err = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await
        .expect_err("default greater than max should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        err.to_string(),
        "features.multi_agent_v2.default_wait_timeout_ms must be at most features.multi_agent_v2.max_wait_timeout_ms"
    );

    Ok(())
}

#[tokio::test]
async fn multi_agent_v2_rejects_invalid_tool_namespace() -> std::io::Result<()> {
    for (namespace, expected_message) in [
        (
            "bad namespace",
            "features.multi_agent_v2.tool_namespace must match ^[a-zA-Z0-9_-]+$",
        ),
        (
            "functions",
            "features.multi_agent_v2.tool_namespace uses a reserved namespace: functions",
        ),
    ] {
        let codex_home = TempDir::new()?;
        std::fs::write(
            codex_home.path().join(CONFIG_TOML_FILE),
            format!(
                r#"[features.multi_agent_v2]
enabled = true
tool_namespace = "{namespace}"
"#
            ),
        )?;

        let err = ConfigBuilder::without_managed_config_for_tests()
            .codex_home(codex_home.path().to_path_buf())
            .fallback_cwd(Some(codex_home.path().to_path_buf()))
            .build()
            .await
            .expect_err("invalid multi_agent_v2 tool namespace should fail");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(err.to_string(), expected_message);
    }

    Ok(())
}

#[tokio::test]
async fn multi_agent_v2_session_thread_cap_one_disallows_subagents() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features.multi_agent_v2]
enabled = true
max_concurrent_threads_per_session = 1
"#,
    )?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    assert_eq!(config.multi_agent_v2.max_concurrent_threads_per_session, 1);
    assert_eq!(config.agent_max_threads, None);
    assert_eq!(
        config.effective_agent_max_threads(codex_protocol::protocol::MultiAgentVersion::V2)?,
        Some(0)
    );

    Ok(())
}

#[tokio::test]
async fn feature_requirements_normalize_runtime_feature_mutations() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;

    let mut config = ConfigBuilder::default()
        .codex_home(codex_home.path().to_path_buf())
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

    let mut requested = config.features.get().clone();
    requested
        .disable(Feature::Personality)
        .enable(Feature::ShellTool);
    assert!(config.features.can_set(&requested).is_ok());
    config
        .features
        .set(requested)
        .expect("managed feature mutations should normalize successfully");

    assert!(config.features.enabled(Feature::Personality));
    assert!(!config.features.enabled(Feature::ShellTool));

    Ok(())
}

#[tokio::test]
async fn feature_requirements_warn_on_collab_legacy_alias() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .cloud_requirements(CloudRequirementsLoader::new(async {
            Ok(Some(codex_config::ConfigRequirementsToml {
                feature_requirements: Some(codex_config::FeatureRequirementsToml {
                    entries: BTreeMap::from([("collab".to_string(), true)]),
                }),
                ..Default::default()
            }))
        }))
        .build()
        .await?;

    assert!(config.features.enabled(Feature::Collab));
    assert!(
        config.startup_warnings.iter().any(|warning| {
            warning.contains("Using legacy `features` requirement `collab`")
                && warning.contains("prefer canonical feature key `multi_agent`")
        }),
        "{:?}",
        config.startup_warnings
    );

    Ok(())
}

#[tokio::test]
async fn feature_requirements_warn_and_ignore_unknown_feature() -> std::io::Result<()> {
    let codex_home = TempDir::new()?;

    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .cloud_requirements(CloudRequirementsLoader::new(async {
            Ok(Some(codex_config::ConfigRequirementsToml {
                feature_requirements: Some(codex_config::FeatureRequirementsToml {
                    entries: BTreeMap::from([("made_up_feature".to_string(), true)]),
                }),
                ..Default::default()
            }))
        }))
        .build()
        .await?;

    assert!(
        config
            .startup_warnings
            .iter()
            .any(|warning| warning
                .contains("Ignoring unknown `features` requirement `made_up_feature`")),
        "{:?}",
        config.startup_warnings
    );

    Ok(())
}
