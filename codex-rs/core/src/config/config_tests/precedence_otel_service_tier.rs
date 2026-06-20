use super::common::*;
use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn legacy_profile_selection_is_rejected() -> std::io::Result<()> {
    let mut fixture = create_test_fixture()?;
    fixture.cfg.profile = Some("gpt3".to_string());

    let err = Config::load_from_base_config_with_overrides(
        fixture.cfg.clone(),
        ConfigOverrides {
            cwd: Some(fixture.cwd_path()),
            ..Default::default()
        },
        fixture.codex_home(),
    )
    .await
    .expect_err("legacy profile selection should be rejected");

    assert_eq!(err.kind(), ErrorKind::InvalidData);
    assert!(
        err.to_string()
            .contains("legacy `profile = \"gpt3\"` config is no longer supported"),
        "unexpected error: {err}"
    );
    Ok(())
}

#[tokio::test]
async fn metrics_exporter_defaults_to_statsig_when_missing() -> std::io::Result<()> {
    let fixture = create_test_fixture()?;

    let config = Config::load_from_base_config_with_overrides(
        fixture.cfg.clone(),
        ConfigOverrides {
            cwd: Some(fixture.cwd_path()),
            ..Default::default()
        },
        fixture.codex_home(),
    )
    .await?;

    assert_eq!(config.otel.metrics_exporter, OtelExporterKind::Statsig);
    Ok(())
}

#[tokio::test]
async fn trace_exporter_defaults_to_none_when_log_exporter_is_set() -> std::io::Result<()> {
    let fixture = create_test_fixture()?;
    let mut cfg = fixture.cfg.clone();
    cfg.otel = Some(OtelConfigToml {
        exporter: Some(OtelExporterKind::OtlpHttp {
            endpoint: "http://localhost:14318/v1/logs".to_string(),
            headers: HashMap::new(),
            protocol: codex_config::types::OtelHttpProtocol::Binary,
            tls: None,
        }),
        metrics_exporter: Some(OtelExporterKind::None),
        ..Default::default()
    });

    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides {
            cwd: Some(fixture.cwd_path()),
            ..Default::default()
        },
        fixture.codex_home(),
    )
    .await?;

    assert!(matches!(
        config.otel.exporter,
        OtelExporterKind::OtlpHttp { .. }
    ));
    assert_eq!(config.otel.trace_exporter, OtelExporterKind::None);
    Ok(())
}

#[tokio::test]
async fn load_config_applies_otel_trace_metadata() -> std::io::Result<()> {
    let mut fixture = create_test_fixture()?;
    fixture.cfg = toml::from_str(
        r#"
[otel.span_attributes]
"example.trace_attr" = "enabled"

[otel.tracestate.example]
alpha = "one"
beta = "two"
"#,
    )
    .expect("TOML deserialization should succeed");

    let config = Config::load_from_base_config_with_overrides(
        fixture.cfg.clone(),
        ConfigOverrides {
            cwd: Some(fixture.cwd_path()),
            ..Default::default()
        },
        fixture.codex_home(),
    )
    .await?;

    assert_eq!(
        config.otel.span_attributes,
        BTreeMap::from([("example.trace_attr".to_string(), "enabled".to_string())])
    );
    assert_eq!(
        config.otel.tracestate,
        BTreeMap::from([(
            "example".to_string(),
            BTreeMap::from([
                ("alpha".to_string(), "one".to_string()),
                ("beta".to_string(), "two".to_string()),
            ]),
        )])
    );
    Ok(())
}

#[tokio::test]
async fn load_config_drops_invalid_otel_trace_metadata_entries() -> std::io::Result<()> {
    let mut fixture = create_test_fixture()?;
    fixture.cfg = toml::from_str(
        r#"
[otel]
environment = "test"

[otel.span_attributes]
"" = "missing-key"
"example.trace_attr" = "enabled"

[otel.tracestate.example]
alpha = "one"
beta = "two\ntoo"

[otel.tracestate.bad]
alpha = "one\ntwo"
"#,
    )
    .expect("TOML deserialization should succeed");

    let config = Config::load_from_base_config_with_overrides(
        fixture.cfg.clone(),
        ConfigOverrides {
            cwd: Some(fixture.cwd_path()),
            ..Default::default()
        },
        fixture.codex_home(),
    )
    .await?;

    assert_eq!(config.otel.environment, "test");
    assert_eq!(
        config.otel.span_attributes,
        BTreeMap::from([("example.trace_attr".to_string(), "enabled".to_string())])
    );
    assert_eq!(
        config.otel.tracestate,
        BTreeMap::from([(
            "example".to_string(),
            BTreeMap::from([("alpha".to_string(), "one".to_string())]),
        )])
    );
    assert!(
        config.startup_warnings.iter().any(|warning| {
            warning.contains("Ignoring invalid `otel.span_attributes` config")
                && warning.contains("configured span attribute key must not be empty")
        }),
        "{:?}",
        config.startup_warnings
    );
    assert!(
        config.startup_warnings.iter().any(|warning| {
            warning.contains("Ignoring invalid `otel.tracestate` config")
                && warning.contains("invalid configured tracestate value for example.beta")
        }),
        "{:?}",
        config.startup_warnings
    );
    assert!(
        config.startup_warnings.iter().any(|warning| {
            warning.contains("Ignoring invalid `otel.tracestate` config")
                && warning.contains("invalid configured tracestate value for bad.alpha")
        }),
        "{:?}",
        config.startup_warnings
    );
    Ok(())
}

#[tokio::test]
async fn explicit_null_service_tier_override_maps_to_default_service_tier() -> std::io::Result<()> {
    let fixture = create_test_fixture()?;

    let config = Config::load_from_base_config_with_overrides(
        fixture.cfg.clone(),
        ConfigOverrides {
            cwd: Some(fixture.cwd_path()),
            service_tier: Some(None),
            ..Default::default()
        },
        fixture.codex_home(),
    )
    .await?;

    assert_eq!(
        config.service_tier,
        Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string())
    );
    assert_eq!(config.notices.fast_default_opt_out, None);
    Ok(())
}

#[tokio::test]
async fn default_service_tier_override_uses_default_request_value() -> std::io::Result<()> {
    let fixture = create_test_fixture()?;

    let config = Config::load_from_base_config_with_overrides(
        fixture.cfg.clone(),
        ConfigOverrides {
            cwd: Some(fixture.cwd_path()),
            service_tier: Some(Some("default".to_string())),
            ..Default::default()
        },
        fixture.codex_home(),
    )
    .await?;

    assert_eq!(
        config.service_tier,
        Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string())
    );
    Ok(())
}

#[tokio::test]
async fn legacy_fast_service_tier_override_uses_priority_request_value() -> std::io::Result<()> {
    let fixture = create_test_fixture()?;

    let config = Config::load_from_base_config_with_overrides(
        fixture.cfg.clone(),
        ConfigOverrides {
            cwd: Some(fixture.cwd_path()),
            service_tier: Some(Some("fast".to_string())),
            ..Default::default()
        },
        fixture.codex_home(),
    )
    .await?;

    assert_eq!(
        config.service_tier,
        Some(ServiceTier::Fast.request_value().to_string())
    );
    Ok(())
}

#[test]
fn context_budget_mode_deserializes_top_level_and_profile() {
    let cfg: ConfigToml = toml::from_str(
        r#"
context_budget_mode = "slow"

[profiles.lean]
context_budget_mode = "standard"
"#,
    )
    .expect("deserialize context budget mode");

    assert_eq!(cfg.context_budget_mode, Some(ContextBudgetMode::Slow));
    assert_eq!(
        cfg.profiles
            .get("lean")
            .and_then(|profile| profile.context_budget_mode),
        Some(ContextBudgetMode::Standard)
    );
}

#[tokio::test]
async fn context_budget_mode_profile_overrides_top_level() -> std::io::Result<()> {
    let fixture = create_test_fixture()?;
    let mut cfg = fixture.cfg.clone();
    cfg.context_budget_mode = Some(ContextBudgetMode::Slow);
    cfg.profiles.insert(
        "standard".to_string(),
        ConfigProfile {
            context_budget_mode: Some(ContextBudgetMode::Standard),
            ..Default::default()
        },
    );

    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides {
            cwd: Some(fixture.cwd_path()),
            config_profile: Some("standard".to_string()),
            ..Default::default()
        },
        fixture.codex_home(),
    )
    .await?;

    assert_eq!(config.context_budget_mode, ContextBudgetMode::Standard);
    Ok(())
}

#[tokio::test]
async fn config_toml_priority_service_tier_uses_priority_request_value() -> std::io::Result<()> {
    let mut fixture = create_test_fixture()?;
    fixture.cfg.service_tier = Some(ServiceTier::Fast.request_value().to_string());
    let cwd = fixture.cwd_path();
    let codex_home = fixture.codex_home();

    let config = Config::load_from_base_config_with_overrides(
        fixture.cfg,
        ConfigOverrides {
            cwd: Some(cwd),
            ..Default::default()
        },
        codex_home,
    )
    .await?;

    assert_eq!(
        config.service_tier,
        Some(ServiceTier::Fast.request_value().to_string())
    );
    Ok(())
}

#[tokio::test]
async fn config_toml_service_tier_accepts_arbitrary_string() -> std::io::Result<()> {
    let mut fixture = create_test_fixture()?;
    fixture.cfg.service_tier = Some("experimental-tier-id".to_string());
    let cwd = fixture.cwd_path();
    let codex_home = fixture.codex_home();

    let config = Config::load_from_base_config_with_overrides(
        fixture.cfg,
        ConfigOverrides {
            cwd: Some(cwd),
            ..Default::default()
        },
        codex_home,
    )
    .await?;

    assert_eq!(
        config.service_tier,
        Some("experimental-tier-id".to_string())
    );
    Ok(())
}

#[tokio::test]
async fn config_toml_legacy_fast_service_tier_uses_priority_request_value() -> std::io::Result<()> {
    let mut fixture = create_test_fixture()?;
    fixture.cfg.service_tier = Some("fast".to_string());
    let cwd = fixture.cwd_path();
    let codex_home = fixture.codex_home();

    let config = Config::load_from_base_config_with_overrides(
        fixture.cfg,
        ConfigOverrides {
            cwd: Some(cwd),
            ..Default::default()
        },
        codex_home,
    )
    .await?;

    assert_eq!(
        config.service_tier,
        Some(ServiceTier::Fast.request_value().to_string())
    );
    Ok(())
}

#[tokio::test]
async fn fast_default_opt_out_notice_config_is_respected() -> std::io::Result<()> {
    let fixture = create_test_fixture()?;
    let mut cfg = fixture.cfg.clone();
    cfg.notice = Some(Notice {
        fast_default_opt_out: Some(true),
        ..Default::default()
    });

    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides {
            cwd: Some(fixture.cwd_path()),
            ..Default::default()
        },
        fixture.codex_home(),
    )
    .await?;

    assert_eq!(config.service_tier, None);
    assert_eq!(config.notices.fast_default_opt_out, Some(true));
    Ok(())
}
