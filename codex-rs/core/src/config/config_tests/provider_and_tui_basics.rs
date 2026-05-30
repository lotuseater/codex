use super::*;

#[test]
fn rejects_provider_auth_with_env_key() {
    let err = toml::from_str::<ConfigToml>(
        r#"
[model_providers.corp]
name = "Corp"
env_key = "CORP_TOKEN"

[model_providers.corp.auth]
command = "print-token"
"#,
    )
    .unwrap_err();

    assert!(
        err.to_string()
            .contains("model_providers.corp: provider auth cannot be combined with env_key")
    );
}

#[test]
fn rejects_provider_aws_for_custom_provider() {
    let err = toml::from_str::<ConfigToml>(
        r#"
[model_providers.custom]
name = "Custom Provider"

[model_providers.custom.aws]
profile = "codex-bedrock"
"#,
    )
    .unwrap_err();

    assert!(
        err.to_string().contains(
            "model_providers.custom: provider aws is only supported for `amazon-bedrock`"
        )
    );
}

#[test]
fn accepts_amazon_bedrock_aws_profile_override() {
    let cfg = toml::from_str::<ConfigToml>(
        r#"
[model_providers.amazon-bedrock.aws]
profile = "codex-bedrock"
region = "us-west-2"
"#,
    )
    .expect("Amazon Bedrock AWS overrides should deserialize");

    assert_eq!(
        cfg.model_providers
            .get("amazon-bedrock")
            .and_then(|provider| provider.aws.as_ref())
            .and_then(|aws| aws.profile.as_deref()),
        Some("codex-bedrock")
    );
    assert_eq!(
        cfg.model_providers
            .get("amazon-bedrock")
            .and_then(|provider| provider.aws.as_ref())
            .and_then(|aws| aws.region.as_deref()),
        Some("us-west-2")
    );
}

#[tokio::test]
async fn load_config_applies_amazon_bedrock_aws_profile_override() {
    let cfg = toml::from_str::<ConfigToml>(
        r#"
model_provider = "amazon-bedrock"

[model_providers.amazon-bedrock.aws]
profile = "codex-bedrock"
region = "us-west-2"
"#,
    )
    .expect("Amazon Bedrock AWS overrides should deserialize");

    let config = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides::default(),
        tempdir().expect("tempdir").abs(),
    )
    .await
    .expect("load config");

    assert_eq!(config.model_provider_id, "amazon-bedrock");
    assert_eq!(
        config
            .model_provider
            .aws
            .as_ref()
            .and_then(|aws| aws.profile.as_deref()),
        Some("codex-bedrock")
    );
    assert_eq!(
        config
            .model_provider
            .aws
            .as_ref()
            .and_then(|aws| aws.region.as_deref()),
        Some("us-west-2")
    );
}

#[tokio::test]
async fn load_config_rejects_unsupported_amazon_bedrock_overrides() {
    let cfg = toml::from_str::<ConfigToml>(
        r#"
model_provider = "amazon-bedrock"

[model_providers.amazon-bedrock]
name = "Custom Bedrock"
base_url = "https://bedrock.example.com/v1"
requires_openai_auth = true
supports_websockets = true

[model_providers.amazon-bedrock.aws]
profile = "codex-bedrock"
region = "us-west-2"
"#,
    )
    .expect("Amazon Bedrock unsupported overrides should deserialize");

    let err = Config::load_from_base_config_with_overrides(
        cfg,
        ConfigOverrides::default(),
        tempdir().expect("tempdir").abs(),
    )
    .await
    .unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains(
        "model_providers.amazon-bedrock only supports changing `aws.profile` and `aws.region`; other non-default provider fields are not supported"
    ));
}

#[test]
fn config_toml_deserializes_model_availability_nux() {
    let toml = r#"
[tui.model_availability_nux]
"gpt-foo" = 2
"gpt-bar" = 4
"#;
    let cfg: ConfigToml =
        toml::from_str(toml).expect("TOML deserialization should succeed for TUI NUX");

    assert_eq!(
        cfg.tui.expect("tui config should deserialize"),
        Tui {
            notification_settings: TuiNotificationSettings::default(),
            animations: true,
            show_tooltips: true,
            vim_mode_default: false,
            raw_output_mode: false,
            alternate_screen: AltScreenMode::default(),
            status_line: None,
            status_line_use_colors: true,
            terminal_title: None,
            theme: None,
            pet: None,
            pet_anchor: TuiPetAnchor::Composer,
            session_picker_view: None,
            keymap: TuiKeymap::default(),
            model_availability_nux: ModelAvailabilityNuxConfig {
                shown_count: HashMap::from([
                    ("gpt-bar".to_string(), 4),
                    ("gpt-foo".to_string(), 2),
                ]),
            },
            terminal_resize_reflow_max_rows: None,
        }
    );
}

#[test]
fn config_toml_status_line_use_colors_defaults_to_enabled() {
    let toml = r#"
[tui]
"#;
    let cfg: ConfigToml =
        toml::from_str(toml).expect("TOML deserialization should succeed for TUI config");

    assert!(
        cfg.tui
            .expect("tui config should deserialize")
            .status_line_use_colors
    );
}

#[test]
fn config_toml_deserializes_status_line_use_colors_disabled() {
    let toml = r#"
[tui]
status_line_use_colors = false
"#;
    let cfg: ConfigToml =
        toml::from_str(toml).expect("TOML deserialization should succeed for TUI config");

    assert!(
        !cfg.tui
            .expect("tui config should deserialize")
            .status_line_use_colors
    );
}

#[test]
fn config_toml_deserializes_terminal_resize_reflow_config() {
    let toml = r#"
[tui]
terminal_resize_reflow_max_rows = 9000
"#;
    let cfg: ConfigToml =
        toml::from_str(toml).expect("TOML deserialization should succeed for resize reflow config");

    assert_eq!(
        cfg.tui
            .expect("tui config should deserialize")
            .terminal_resize_reflow_max_rows,
        Some(9000)
    );
}

#[tokio::test]
async fn runtime_config_defaults_model_availability_nux() {
    let cfg = Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        tempdir().expect("tempdir").abs(),
    )
    .await
    .expect("load config");

    assert_eq!(
        cfg.model_availability_nux,
        ModelAvailabilityNuxConfig::default()
    );
}

#[test]
fn test_tui_vim_mode_default_defaults_to_false() {
    let toml = r#"
        [tui]
    "#;
    let parsed: ConfigToml = toml::from_str(toml).expect("deserialize empty [tui] table");
    assert!(
        !parsed
            .tui
            .expect("config should include tui section")
            .vim_mode_default
    );
}

#[test]
fn test_tui_vim_mode_default_true() {
    let toml = r#"
        [tui]
        vim_mode_default = true
    "#;
    let parsed: ConfigToml = toml::from_str(toml).expect("deserialize vim_mode_default=true");
    assert!(
        parsed
            .tui
            .expect("config should include tui section")
            .vim_mode_default
    );
}

#[test]
fn test_tui_raw_output_mode_defaults_to_false() {
    let toml = r#"
        [tui]
    "#;
    let parsed: ConfigToml = toml::from_str(toml).expect("deserialize empty [tui] table");
    assert!(
        !parsed
            .tui
            .expect("config should include tui section")
            .raw_output_mode
    );
}

#[test]
fn test_tui_raw_output_mode_true() {
    let toml = r#"
        [tui]
        raw_output_mode = true
    "#;
    let parsed: ConfigToml = toml::from_str(toml).expect("deserialize raw_output_mode=true");
    assert!(
        parsed
            .tui
            .expect("config should include tui section")
            .raw_output_mode
    );
}

#[tokio::test]
async fn runtime_config_uses_tui_raw_output_mode() {
    let toml = r#"
        [tui]
        raw_output_mode = true
    "#;
    let cfg_toml: ConfigToml = toml::from_str(toml).expect("deserialize raw_output_mode=true");
    let cfg = Config::load_from_base_config_with_overrides(
        cfg_toml,
        ConfigOverrides::default(),
        tempdir().expect("tempdir").abs(),
    )
    .await
    .expect("load config");

    assert!(cfg.tui_raw_output_mode);
}

