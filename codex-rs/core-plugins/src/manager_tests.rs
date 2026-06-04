use super::*;
use crate::LoadedPlugin;
use crate::PluginLoadOutcome;
use crate::installed_marketplaces::marketplace_install_root;
use crate::loader::load_plugins_from_layer_stack;
use crate::loader::refresh_non_curated_plugin_cache;
use crate::loader::refresh_non_curated_plugin_cache_force_reinstall;
use crate::marketplace::MarketplacePluginInstallPolicy;
use crate::remote::RemoteInstalledPlugin;
use crate::remote::RemotePluginScope;
use crate::startup_sync::curated_plugins_repo_path;
use crate::test_support::TEST_CURATED_PLUGIN_CACHE_VERSION;
use crate::test_support::TEST_CURATED_PLUGIN_SHA;
use crate::test_support::load_plugins_config as load_plugins_config_input;
use crate::test_support::write_curated_plugin_sha_with as write_curated_plugin_sha;
use crate::test_support::write_file;
use crate::test_support::write_openai_curated_marketplace;
use codex_config_types::ConfigLayerSource;
use codex_config::AppToolApproval;
use codex_config::CONFIG_TOML_FILE;
use codex_config::ConfigLayerEntry;
use codex_config::ConfigLayerStack;
use codex_config::ConfigRequirements;
use codex_config::ConfigRequirementsToml;
use codex_config::McpServerConfig;
use codex_config::McpServerOAuthConfig;
use codex_config::McpServerToolConfig;
use codex_config::types::McpServerTransportConfig;
use codex_login::CodexAuth;
use codex_protocol::protocol::HookEventName;
use codex_protocol::protocol::Product;
use codex_utils_absolute_path::test_support::PathBufExt;
use pretty_assertions::assert_eq;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use toml::Value;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;
use wiremock::matchers::query_param;

mod capability_summary;
mod featured;
mod install;
mod list_marketplaces;
mod list_marketplaces_installed;
mod load_plugins;
mod load_plugins_component_paths;
mod read_plugin;
mod refresh_curated_cache;
mod refresh_non_curated_cache;
mod remote_installed_cache;
mod sync_remote;
mod sync_remote_edge;

const MAX_CAPABILITY_SUMMARY_DESCRIPTION_LEN: usize = 1024;

fn write_plugin_with_version(
    root: &Path,
    dir_name: &str,
    manifest_name: &str,
    manifest_version: Option<&str>,
) {
    let plugin_root = root.join(dir_name);
    fs::create_dir_all(plugin_root.join(".codex-plugin")).unwrap();
    fs::create_dir_all(plugin_root.join("skills")).unwrap();
    let version = manifest_version
        .map(|manifest_version| format!(r#","version":"{manifest_version}""#))
        .unwrap_or_default();
    fs::write(
        plugin_root.join(".codex-plugin/plugin.json"),
        format!(r#"{{"name":"{manifest_name}"{version}}}"#),
    )
    .unwrap();
    fs::write(plugin_root.join("skills/SKILL.md"), "skill").unwrap();
    fs::write(plugin_root.join(".mcp.json"), r#"{"mcpServers":{}}"#).unwrap();
}

fn write_plugin(root: &Path, dir_name: &str, manifest_name: &str) {
    write_plugin_with_version(
        root,
        dir_name,
        manifest_name,
        /*manifest_version*/ None,
    );
}

fn init_git_repo(repo: &Path) {
    run_git(repo, &["init"]);
    run_git(repo, &["config", "user.email", "codex-test@example.com"]);
    run_git(repo, &["config", "user.name", "Codex Test"]);
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-m", "initial"]);
}

fn run_git(repo: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("git should run: {err}"));
    assert!(
        output.status.success(),
        "git -C {} {} failed\nstdout:\n{}\nstderr:\n{}",
        repo.display(),
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn plugin_config_toml(enabled: bool, plugins_feature_enabled: bool) -> String {
    let mut root = toml::map::Map::new();

    let mut features = toml::map::Map::new();
    features.insert(
        "plugins".to_string(),
        Value::Boolean(plugins_feature_enabled),
    );
    root.insert("features".to_string(), Value::Table(features));

    let mut plugin = toml::map::Map::new();
    plugin.insert("enabled".to_string(), Value::Boolean(enabled));

    let mut plugins = toml::map::Map::new();
    plugins.insert("sample@test".to_string(), Value::Table(plugin));
    root.insert("plugins".to_string(), Value::Table(plugins));

    toml::to_string(&Value::Table(root)).expect("plugin test config should serialize")
}

async fn load_plugins_from_config(config_toml: &str, codex_home: &Path) -> PluginLoadOutcome {
    write_file(&codex_home.join(CONFIG_TOML_FILE), config_toml);
    let config = load_config(codex_home, codex_home).await;
    PluginsManager::new(codex_home.to_path_buf())
        .plugins_for_config(&config)
        .await
}

async fn load_config(codex_home: &Path, cwd: &Path) -> PluginsConfigInput {
    load_plugins_config_input(codex_home, cwd).await
}

fn remote_installed_linear_plugin() -> RemoteInstalledPlugin {
    remote_installed_plugin("linear")
}

fn remote_installed_plugin(name: &str) -> RemoteInstalledPlugin {
    RemoteInstalledPlugin {
        marketplace_name: "openai-curated-remote".to_string(),
        id: format!("plugins~Plugin_{name}"),
        name: name.to_string(),
        enabled: true,
        install_policy: codex_protocol::plugin::PluginInstallPolicy::Available,
        auth_policy: codex_protocol::plugin::PluginAuthPolicy::OnUse,
        availability: codex_protocol::plugin::PluginAvailability::Available,
        interface: None,
        keywords: Vec::new(),
    }
}
// fork-local: this test family was split into `manager_tests/*.rs` submodules by the fork's
// modularization pass. The four tests below (plus the `write_cached_plugin` helper) are NEW in
// upstream and not yet present in the fork's split submodules, so they are kept inline here to
// preserve coverage. A later test-repair wave can relocate them to their topical submodules
// (`remote_installed_cache.rs`, `load_plugins.rs`).
fn write_cached_plugin(codex_home: &Path, marketplace_name: &str, plugin_name: &str) {
    write_plugin_with_version(
        &codex_home
            .join("plugins/cache")
            .join(marketplace_name)
            .join(plugin_name),
        "local",
        plugin_name,
        /*manifest_version*/ Some("local"),
    );
}

#[tokio::test]
async fn remote_installed_cache_prefers_local_curated_conflicts_when_remote_plugin_disabled() {
    let codex_home = TempDir::new().unwrap();
    write_file(
        &codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
remote_plugin = false

[plugins."linear@openai-curated"]
enabled = true

[plugins."calendar@openai-curated"]
enabled = true
"#,
    );
    write_cached_plugin(codex_home.path(), "openai-curated", "linear");
    write_cached_plugin(codex_home.path(), "openai-curated", "calendar");
    write_cached_plugin(codex_home.path(), "openai-curated-remote", "linear");
    write_cached_plugin(codex_home.path(), "openai-curated-remote", "remote-only");

    let config = load_config(codex_home.path(), codex_home.path()).await;
    let manager = PluginsManager::new(codex_home.path().to_path_buf());
    manager.write_remote_installed_plugins_cache(vec![
        remote_installed_plugin("linear"),
        remote_installed_plugin("remote-only"),
    ]);

    let outcome = manager.plugins_for_config(&config).await;
    assert_eq!(
        outcome
            .plugins()
            .iter()
            .map(|plugin| plugin.config_name.clone())
            .collect::<Vec<_>>(),
        vec![
            "calendar@openai-curated".to_string(),
            "linear@openai-curated".to_string(),
            "remote-only@openai-curated-remote".to_string(),
        ]
    );
}

#[tokio::test]
async fn remote_installed_cache_prefers_remote_curated_conflicts_when_remote_plugin_enabled() {
    let codex_home = TempDir::new().unwrap();
    write_file(
        &codex_home.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
remote_plugin = true

[plugins."linear@openai-curated"]
enabled = true

[plugins."calendar@openai-curated"]
enabled = true
"#,
    );
    write_cached_plugin(codex_home.path(), "openai-curated", "linear");
    write_cached_plugin(codex_home.path(), "openai-curated", "calendar");
    write_cached_plugin(codex_home.path(), "openai-curated-remote", "linear");
    write_cached_plugin(codex_home.path(), "openai-curated-remote", "remote-only");

    let config = load_config(codex_home.path(), codex_home.path()).await;
    let manager = PluginsManager::new(codex_home.path().to_path_buf());
    manager.write_remote_installed_plugins_cache(vec![
        remote_installed_plugin("linear"),
        remote_installed_plugin("remote-only"),
    ]);

    let outcome = manager.plugins_for_config(&config).await;
    assert_eq!(
        outcome
            .plugins()
            .iter()
            .map(|plugin| plugin.config_name.clone())
            .collect::<Vec<_>>(),
        vec![
            "calendar@openai-curated".to_string(),
            "linear@openai-curated-remote".to_string(),
            "remote-only@openai-curated-remote".to_string(),
        ]
    );
}

#[tokio::test]
async fn load_plugins_ignores_invalid_manifest_skills_shape() {
    let codex_home = TempDir::new().unwrap();
    let plugin_root = codex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");

    write_file(
        &plugin_root.join(".codex-plugin/plugin.json"),
        r#"{
  "name": "sample",
  "skills": ["./custom-skills/"]
}"#,
    );
    write_file(
        &plugin_root.join("skills/default-skill/SKILL.md"),
        "---\nname: default-skill\ndescription: default skill\n---\n",
    );
    write_file(
        &plugin_root.join("custom-skills/custom-skill/SKILL.md"),
        "---\nname: custom-skill\ndescription: custom skill\n---\n",
    );

    let outcome = load_plugins_from_config(
        &plugin_config_toml(/*enabled*/ true, /*plugins_feature_enabled*/ true),
        codex_home.path(),
    )
    .await;

    assert_eq!(outcome.plugins()[0].error, None);
    assert_eq!(
        outcome.plugins()[0].skill_roots,
        vec![plugin_root.join("skills").abs()]
    );
}

#[tokio::test]
async fn effective_apps_preserves_app_config_order() {
    let codex_home = TempDir::new().unwrap();
    let plugin_root = codex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");

    write_file(
        &plugin_root.join(".codex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    );
    write_file(
        &plugin_root.join(".app.json"),
        r#"{
  "apps": {
    "slack": {
      "id": "connector_slack"
    },
    "github": {
      "id": "connector_github"
    },
    "slack-copy": {
      "id": "connector_slack"
    }
  }
}"#,
    );

    let outcome = load_plugins_from_config(
        &plugin_config_toml(/*enabled*/ true, /*plugins_feature_enabled*/ true),
        codex_home.path(),
    )
    .await;

    assert_eq!(
        outcome.effective_apps(),
        vec![
            AppConnectorId("connector_slack".to_string()),
            AppConnectorId("connector_github".to_string()),
        ]
    );
}
