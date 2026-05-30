use super::*;

fn config_error_from_io(err: &std::io::Error) -> &ConfigError {
    err.get_ref()
        .and_then(|err| err.downcast_ref::<ConfigLoadError>())
        .map(ConfigLoadError::config_error)
        .expect("expected ConfigLoadError")
}

#[tokio::test]
async fn returns_config_error_for_invalid_user_config_toml() {
    let tmp = tempdir().expect("tempdir");
    let contents = r#"model = "gpt-4"
invalid = ["#;
    let config_path = tmp.path().join(CONFIG_TOML_FILE);
    std::fs::write(&config_path, contents).expect("write config");

    let cwd = AbsolutePathBuf::try_from(tmp.path()).expect("cwd");
    let err = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(cwd),
        &[] as &[(String, TomlValue)],
        LoaderOverrides::default(),
        CloudRequirementsLoader::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await
    .expect_err("expected error");

    let config_error = config_error_from_io(&err);
    let expected_toml_error = toml::from_str::<TomlValue>(contents).expect_err("parse error");
    let expected_config_error = config_error_from_toml(&config_path, contents, expected_toml_error);
    assert_eq!(config_error, &expected_config_error);
}

#[tokio::test]
async fn returns_config_error_for_invalid_managed_config_toml() {
    let tmp = tempdir().expect("tempdir");
    let managed_path = tmp.path().join("managed_config.toml");
    let contents = r#"model = "gpt-4"
invalid = ["#;
    std::fs::write(&managed_path, contents).expect("write managed config");

    let overrides = LoaderOverrides::with_managed_config_path_for_tests(managed_path.clone());

    let cwd = AbsolutePathBuf::try_from(tmp.path()).expect("cwd");
    let err = load_config_layers_state(
        LOCAL_FS.as_ref(),
        tmp.path(),
        Some(cwd),
        &[] as &[(String, TomlValue)],
        overrides,
        CloudRequirementsLoader::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await
    .expect_err("expected error");

    let config_error = config_error_from_io(&err);
    let expected_toml_error = toml::from_str::<TomlValue>(contents).expect_err("parse error");
    let expected_config_error =
        config_error_from_toml(&managed_path, contents, expected_toml_error);
    assert_eq!(config_error, &expected_config_error);
}

#[tokio::test]
async fn returns_config_error_for_schema_error_in_user_config() {
    let tmp = tempdir().expect("tempdir");
    let contents = "model_context_window = \"not_a_number\"";
    let config_path = tmp.path().join(CONFIG_TOML_FILE);
    std::fs::write(&config_path, contents).expect("write config");

    let err = ConfigBuilder::default()
        .codex_home(tmp.path().to_path_buf())
        .fallback_cwd(Some(tmp.path().to_path_buf()))
        .build()
        .await
        .expect_err("expected error");

    let config_error = config_error_from_io(&err);
    let _guard = codex_utils_absolute_path::AbsolutePathBufGuard::new(tmp.path());
    let expected_config_error =
        codex_config::config_error_from_typed_toml::<ConfigToml>(&config_path, contents)
            .expect("schema error");
    assert_eq!(config_error, &expected_config_error);
}

#[tokio::test]
async fn strict_config_rejects_unknown_user_config_key() {
    let tmp = tempdir().expect("tempdir");
    let contents = r#"model = "gpt-5"
unknown_key = true"#;
    let config_path = tmp.path().join(CONFIG_TOML_FILE);
    std::fs::write(&config_path, contents).expect("write config");

    let err = ConfigBuilder::default()
        .codex_home(tmp.path().to_path_buf())
        .fallback_cwd(Some(tmp.path().to_path_buf()))
        .loader_overrides(LoaderOverrides::without_managed_config_for_tests())
        .strict_config(/*strict_config*/ true)
        .build()
        .await
        .expect_err("expected error");

    let config_error = config_error_from_io(&err);
    let expected_config_error =
        config_error_from_ignored_toml_fields::<ConfigToml>(&config_path, contents)
            .expect("unknown field error");
    assert_eq!(config_error, &expected_config_error);
}

#[tokio::test]
async fn strict_config_rejects_unknown_cli_override_key() {
    let tmp = tempdir().expect("tempdir");

    let err = ConfigBuilder::default()
        .codex_home(tmp.path().to_path_buf())
        .fallback_cwd(Some(tmp.path().to_path_buf()))
        .loader_overrides(LoaderOverrides::without_managed_config_for_tests())
        .cli_overrides(vec![(
            "foo".to_string(),
            TomlValue::String("bar".to_string()),
        )])
        .strict_config(/*strict_config*/ true)
        .build()
        .await
        .expect_err("expected error");

    assert_eq!(
        err.to_string(),
        "unknown configuration field `foo` in -c/--config override"
    );
}

#[tokio::test]
async fn strict_config_rejects_unknown_cli_override_key_with_relative_path_override() {
    let tmp = tempdir().expect("tempdir");
    let instructions_path = tmp.path().join("instructions.md");
    std::fs::write(&instructions_path, "instructions").expect("write instructions");

    let err = ConfigBuilder::default()
        .codex_home(tmp.path().to_path_buf())
        .fallback_cwd(Some(tmp.path().to_path_buf()))
        .loader_overrides(LoaderOverrides::without_managed_config_for_tests())
        .cli_overrides(vec![
            (
                "model_instructions_file".to_string(),
                TomlValue::String("instructions.md".to_string()),
            ),
            ("foo".to_string(), TomlValue::String("bar".to_string())),
        ])
        .strict_config(/*strict_config*/ true)
        .build()
        .await
        .expect_err("expected error");

    assert_eq!(
        err.to_string(),
        "unknown configuration field `foo` in -c/--config override"
    );
}

#[tokio::test]
async fn strict_config_rejects_unknown_feature_cli_override_key() {
    let tmp = tempdir().expect("tempdir");

    let err = ConfigBuilder::default()
        .codex_home(tmp.path().to_path_buf())
        .fallback_cwd(Some(tmp.path().to_path_buf()))
        .loader_overrides(LoaderOverrides::without_managed_config_for_tests())
        .cli_overrides(vec![("features.foo".to_string(), TomlValue::Boolean(true))])
        .strict_config(/*strict_config*/ true)
        .build()
        .await
        .expect_err("expected error");

    assert_eq!(
        err.to_string(),
        "unknown configuration field `features.foo` in -c/--config override"
    );
}

#[tokio::test]
async fn strict_config_rejects_unknown_feature_user_config_key() {
    let tmp = tempdir().expect("tempdir");
    let contents = r#"[features]
foo = true"#;
    let config_path = tmp.path().join(CONFIG_TOML_FILE);
    std::fs::write(&config_path, contents).expect("write config");

    let err = ConfigBuilder::default()
        .codex_home(tmp.path().to_path_buf())
        .fallback_cwd(Some(tmp.path().to_path_buf()))
        .loader_overrides(LoaderOverrides::without_managed_config_for_tests())
        .strict_config(/*strict_config*/ true)
        .build()
        .await
        .expect_err("expected error");

    let config_error = config_error_from_io(&err);
    assert_eq!(
        config_error.message,
        "unknown configuration field `features.foo`"
    );
    assert_eq!(config_error.range.start.line, 2);
    assert_eq!(config_error.range.start.column, 1);
}

#[test]
fn strict_config_points_to_unknown_nested_key() {
    let tmp = tempdir().expect("tempdir");
    let contents = r#"[mcp_servers.local]
command = "echo"
unknown_key = true"#;
    let config_path = tmp.path().join(CONFIG_TOML_FILE);
    std::fs::write(&config_path, contents).expect("write config");

    let error = config_error_from_ignored_toml_fields::<ConfigToml>(&config_path, contents)
        .expect("unknown field error");

    assert_eq!(
        error.message,
        "unknown configuration field `mcp_servers.local.unknown_key`"
    );
    assert_eq!(error.range.start.line, 3);
    assert_eq!(error.range.start.column, 1);
}
#[test]
fn schema_error_points_to_feature_value() {
    let tmp = tempdir().expect("tempdir");
    let contents = r#"[features]
collaboration_modes = "true""#;
    let config_path = tmp.path().join(CONFIG_TOML_FILE);
    std::fs::write(&config_path, contents).expect("write config");

    let _guard = codex_utils_absolute_path::AbsolutePathBufGuard::new(tmp.path());
    let error = codex_config::config_error_from_typed_toml::<ConfigToml>(&config_path, contents)
        .expect("schema error");

    let value_line = contents.lines().nth(1).expect("value line");
    let value_column = value_line.find("\"true\"").expect("value") + 1;
    assert_eq!(error.range.start.line, 2);
    assert_eq!(error.range.start.column, value_column);
}
