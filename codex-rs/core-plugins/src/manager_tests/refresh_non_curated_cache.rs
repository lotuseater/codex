use super::super::*;
use super::*;

#[test]
fn refresh_non_curated_plugin_cache_replaces_existing_local_version_with_manifest_version() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin_with_version(&repo_root, "sample-plugin", "sample-plugin", Some("1.2.3"));
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    }
  ]
}"#,
    );
    write_plugin(
        &tmp.path().join("plugins/cache/debug"),
        "sample-plugin/local",
        "sample-plugin",
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        refresh_non_curated_plugin_cache(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
        )
        .expect("cache refresh should succeed")
    );

    assert!(
        !tmp.path()
            .join("plugins/cache/debug/sample-plugin/local")
            .exists()
    );
    assert!(
        tmp.path()
            .join("plugins/cache/debug/sample-plugin/1.2.3")
            .is_dir()
    );
}

#[test]
fn refresh_non_curated_plugin_cache_reinstalls_missing_configured_plugin_with_manifest_version() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin_with_version(&repo_root, "sample-plugin", "sample-plugin", Some("1.2.3"));
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    }
  ]
}"#,
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        refresh_non_curated_plugin_cache(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
        )
        .expect("cache refresh should reinstall missing configured plugin")
    );

    assert!(
        tmp.path()
            .join("plugins/cache/debug/sample-plugin/1.2.3")
            .is_dir()
    );
}

#[test]
fn refresh_non_curated_plugin_cache_refreshes_configured_git_source() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    let remote_repo = tmp.path().join("remote-plugin-repo");
    let remote_repo_url = url::Url::from_directory_path(&remote_repo)
        .unwrap()
        .to_string();
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    write_plugin_with_version(
        &remote_repo,
        "plugins/sample-plugin",
        "sample-plugin",
        Some("1.2.3"),
    );
    init_git_repo(&remote_repo);
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        &format!(
            r#"{{
  "name": "debug",
  "plugins": [
    {{
      "name": "sample-plugin",
      "source": {{
        "source": "git-subdir",
        "url": "{remote_repo_url}",
        "path": "plugins/sample-plugin"
      }}
    }}
  ]
}}"#
        ),
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        refresh_non_curated_plugin_cache(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
        )
        .expect("cache refresh should materialize configured Git plugin")
    );

    assert!(
        tmp.path()
            .join("plugins/cache/debug/sample-plugin/1.2.3")
            .is_dir()
    );
}

#[test]
fn refresh_non_curated_plugin_cache_returns_false_when_configured_plugins_are_current() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin_with_version(&repo_root, "sample-plugin", "sample-plugin", Some("1.2.3"));
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    }
  ]
}"#,
    );
    write_plugin_with_version(
        &tmp.path().join("plugins/cache/debug"),
        "sample-plugin/1.2.3",
        "sample-plugin",
        Some("1.2.3"),
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        !refresh_non_curated_plugin_cache(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
        )
        .expect("cache refresh should be a no-op when configured plugins are current")
    );
}

#[test]
fn refresh_non_curated_plugin_cache_force_reinstalls_current_local_version() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin(&repo_root, "sample-plugin", "sample-plugin");
    fs::write(repo_root.join("sample-plugin/skills/SKILL.md"), "new skill").unwrap();
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    }
  ]
}"#,
    );
    write_plugin(
        &tmp.path().join("plugins/cache/debug"),
        "sample-plugin/local",
        "sample-plugin",
    );
    fs::write(
        tmp.path()
            .join("plugins/cache/debug/sample-plugin/local/skills/SKILL.md"),
        "old skill",
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        refresh_non_curated_plugin_cache_force_reinstall(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
        )
        .expect("cache refresh should reinstall unchanged local version")
    );

    assert_eq!(
        fs::read_to_string(
            tmp.path()
                .join("plugins/cache/debug/sample-plugin/local/skills/SKILL.md")
        )
        .unwrap(),
        "new skill"
    );
}

#[test]
fn refresh_non_curated_plugin_cache_ignores_invalid_unconfigured_plugin_versions() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin_with_version(&repo_root, "sample-plugin", "sample-plugin", Some("1.2.3"));
    write_plugin_with_version(&repo_root, "broken-plugin", "broken-plugin", Some("   "));
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    },
    {
      "name": "broken-plugin",
      "source": {
        "source": "local",
        "path": "./broken-plugin"
      }
    }
  ]
}"#,
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        refresh_non_curated_plugin_cache(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
        )
        .expect("cache refresh should ignore unrelated invalid plugin manifests")
    );

    assert!(
        tmp.path()
            .join("plugins/cache/debug/sample-plugin/1.2.3")
            .is_dir()
    );
}
