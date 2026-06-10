use super::super::*;
use super::*;

#[test]
fn refresh_curated_plugin_cache_replaces_existing_local_version_with_short_sha_version() {
    let tmp = tempfile::tempdir().unwrap();
    let curated_root = curated_plugins_repo_path(tmp.path());
    write_openai_curated_marketplace(&curated_root, &["slack"]);
    write_curated_plugin_sha(tmp.path(), TEST_CURATED_PLUGIN_SHA);
    let plugin_id = PluginId::new(
        "slack".to_string(),
        OPENAI_CURATED_MARKETPLACE_NAME.to_string(),
    )
    .unwrap();
    write_plugin(
        &tmp.path().join("plugins/cache/openai-curated"),
        "slack/local",
        "slack",
    );

    assert!(
        refresh_curated_plugin_cache(tmp.path(), TEST_CURATED_PLUGIN_SHA, &[plugin_id])
            .expect("cache refresh should succeed")
    );

    assert!(
        !tmp.path()
            .join("plugins/cache/openai-curated/slack/local")
            .exists()
    );
    assert!(
        tmp.path()
            .join(format!(
                "plugins/cache/openai-curated/slack/{TEST_CURATED_PLUGIN_CACHE_VERSION}"
            ))
            .is_dir()
    );
}

#[test]
fn refresh_curated_plugin_cache_reinstalls_missing_configured_plugin_with_current_short_version() {
    let tmp = tempfile::tempdir().unwrap();
    let curated_root = curated_plugins_repo_path(tmp.path());
    write_openai_curated_marketplace(&curated_root, &["slack"]);
    write_curated_plugin_sha(tmp.path(), TEST_CURATED_PLUGIN_SHA);
    let plugin_id = PluginId::new(
        "slack".to_string(),
        OPENAI_CURATED_MARKETPLACE_NAME.to_string(),
    )
    .unwrap();

    assert!(
        refresh_curated_plugin_cache(tmp.path(), TEST_CURATED_PLUGIN_SHA, &[plugin_id])
            .expect("cache refresh should recreate missing configured plugin")
    );

    assert!(
        tmp.path()
            .join(format!(
                "plugins/cache/openai-curated/slack/{TEST_CURATED_PLUGIN_CACHE_VERSION}"
            ))
            .is_dir()
    );
}

#[test]
fn curated_plugin_ids_from_config_keys_reads_latest_codex_home_user_config() {
    let tmp = tempfile::tempdir().unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."slack@openai-curated"]
enabled = true

[plugins."sample@debug"]
enabled = true
"#,
    );

    assert_eq!(
        configured_curated_plugin_ids_from_codex_home(tmp.path())
            .into_iter()
            .map(|plugin_id| plugin_id.as_key())
            .collect::<Vec<_>>(),
        vec!["slack@openai-curated".to_string()]
    );

    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
"#,
    );

    assert_eq!(
        configured_curated_plugin_ids_from_codex_home(tmp.path()),
        Vec::<PluginId>::new()
    );
}

#[test]
fn refresh_curated_plugin_cache_returns_false_when_configured_plugins_are_current() {
    let tmp = tempfile::tempdir().unwrap();
    let curated_root = curated_plugins_repo_path(tmp.path());
    write_openai_curated_marketplace(&curated_root, &["slack"]);
    let plugin_id = PluginId::new(
        "slack".to_string(),
        OPENAI_CURATED_MARKETPLACE_NAME.to_string(),
    )
    .unwrap();
    write_plugin(
        &tmp.path().join("plugins/cache/openai-curated"),
        &format!("slack/{TEST_CURATED_PLUGIN_CACHE_VERSION}"),
        "slack",
    );

    assert!(
        !refresh_curated_plugin_cache(tmp.path(), TEST_CURATED_PLUGIN_SHA, &[plugin_id])
            .expect("cache refresh should be a no-op when configured plugins are current")
    );
}

#[test]
fn refresh_curated_plugin_cache_migrates_full_sha_cache_version_to_short_version() {
    let tmp = tempfile::tempdir().unwrap();
    let curated_root = curated_plugins_repo_path(tmp.path());
    write_openai_curated_marketplace(&curated_root, &["slack"]);
    let plugin_id = PluginId::new(
        "slack".to_string(),
        OPENAI_CURATED_MARKETPLACE_NAME.to_string(),
    )
    .unwrap();
    write_plugin(
        &tmp.path().join("plugins/cache/openai-curated"),
        &format!("slack/{TEST_CURATED_PLUGIN_SHA}"),
        "slack",
    );

    assert!(
        refresh_curated_plugin_cache(tmp.path(), TEST_CURATED_PLUGIN_SHA, &[plugin_id])
            .expect("cache refresh should migrate the full sha cache version")
    );
    assert!(
        !tmp.path()
            .join(format!(
                "plugins/cache/openai-curated/slack/{TEST_CURATED_PLUGIN_SHA}"
            ))
            .exists()
    );
    assert!(
        tmp.path()
            .join(format!(
                "plugins/cache/openai-curated/slack/{TEST_CURATED_PLUGIN_CACHE_VERSION}"
            ))
            .is_dir()
    );
}
