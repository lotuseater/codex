use super::super::*;
use super::*;

#[tokio::test]
async fn plugin_telemetry_metadata_uses_default_mcp_config_path() {
    let codex_home = TempDir::new().unwrap();
    let plugin_root = codex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");

    write_file(
        &plugin_root.join(".codex-plugin/plugin.json"),
        r#"{
  "name": "sample"
}"#,
    );
    write_file(
        &plugin_root.join(".mcp.json"),
        r#"{
  "mcpServers": {
    "sample": {
      "type": "http",
      "url": "https://sample.example/mcp"
    }
  }
}"#,
    );

    let metadata = plugin_telemetry_metadata_from_root(
        &PluginId::parse("sample@test").expect("plugin id should parse"),
        &plugin_root.abs(),
    )
    .await;

    assert_eq!(
        metadata.capability_summary,
        Some(PluginCapabilitySummary {
            config_name: "sample@test".to_string(),
            display_name: "sample".to_string(),
            description: None,
            has_skills: false,
            mcp_server_names: vec!["sample".to_string()],
            app_connector_ids: Vec::new(),
        })
    );
}

#[tokio::test]
async fn capability_summary_sanitizes_plugin_descriptions_to_one_line() {
    let codex_home = TempDir::new().unwrap();
    let plugin_root = codex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");

    write_file(
        &plugin_root.join(".codex-plugin/plugin.json"),
        r#"{
  "name": "sample",
  "description": "Plugin that\n includes   the sample\tserver"
}"#,
    );
    write_file(
        &plugin_root.join("skills/sample-search/SKILL.md"),
        "---\nname: sample-search\ndescription: search sample data\n---\n",
    );

    let outcome = load_plugins_from_config(
        &plugin_config_toml(/*enabled*/ true, /*plugins_feature_enabled*/ true),
        codex_home.path(),
    )
    .await;

    assert_eq!(
        outcome.plugins()[0].manifest_description.as_deref(),
        Some("Plugin that\n includes   the sample\tserver")
    );
    assert_eq!(
        outcome.capability_summaries()[0].description.as_deref(),
        Some("Plugin that includes the sample server")
    );
}

#[tokio::test]
async fn capability_summary_truncates_overlong_plugin_descriptions() {
    let codex_home = TempDir::new().unwrap();
    let plugin_root = codex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");
    let too_long = "x".repeat(MAX_CAPABILITY_SUMMARY_DESCRIPTION_LEN + 1);

    write_file(
        &plugin_root.join(".codex-plugin/plugin.json"),
        &format!(
            r#"{{
  "name": "sample",
  "description": "{too_long}"
}}"#
        ),
    );
    write_file(
        &plugin_root.join("skills/sample-search/SKILL.md"),
        "---\nname: sample-search\ndescription: search sample data\n---\n",
    );

    let outcome = load_plugins_from_config(
        &plugin_config_toml(/*enabled*/ true, /*plugins_feature_enabled*/ true),
        codex_home.path(),
    )
    .await;

    assert_eq!(
        outcome.plugins()[0].manifest_description.as_deref(),
        Some(too_long.as_str())
    );
    assert_eq!(
        outcome.capability_summaries()[0].description,
        Some("x".repeat(MAX_CAPABILITY_SUMMARY_DESCRIPTION_LEN))
    );
}

#[test]
fn capability_index_filters_inactive_and_zero_capability_plugins() {
    let codex_home = TempDir::new().unwrap();
    let connector = |id: &str| AppConnectorId(id.to_string());
    let http_server = |url: &str| McpServerConfig {
        transport: McpServerTransportConfig::StreamableHttp {
            url: url.to_string(),
            bearer_token_env_var: None,
            http_headers: None,
            env_http_headers: None,
        },
        environment_id: "local".to_string(),
        enabled: true,
        required: false,
        supports_parallel_tool_calls: false,
        disabled_reason: None,
        startup_timeout_sec: None,
        tool_timeout_sec: None,
        default_tools_approval_mode: None,
        enabled_tools: None,
        disabled_tools: None,
        scopes: None,
        oauth: None,
        oauth_resource: None,
        tools: HashMap::new(),
    };
    let plugin = |config_name: &str, dir_name: &str, manifest_name: &str| LoadedPlugin {
        config_name: config_name.to_string(),
        manifest_name: Some(manifest_name.to_string()),
        manifest_description: None,
        root: AbsolutePathBuf::try_from(codex_home.path().join(dir_name)).unwrap(),
        enabled: true,
        skill_roots: Vec::new(),
        disabled_skill_paths: HashSet::new(),
        has_enabled_skills: false,
        mcp_servers: HashMap::new(),
        apps: Vec::new(),
        hook_sources: Vec::new(),
        hook_load_warnings: Vec::new(),
        error: None,
    };
    let summary = |config_name: &str, display_name: &str| PluginCapabilitySummary {
        config_name: config_name.to_string(),
        display_name: display_name.to_string(),
        description: None,
        ..PluginCapabilitySummary::default()
    };
    let outcome = PluginLoadOutcome::from_plugins(vec![
        LoadedPlugin {
            skill_roots: vec![codex_home.path().join("skills-plugin/skills").abs()],
            has_enabled_skills: true,
            ..plugin("skills@test", "skills-plugin", "skills-plugin")
        },
        LoadedPlugin {
            mcp_servers: HashMap::from([("alpha".to_string(), http_server("https://alpha"))]),
            apps: vec![connector("connector_example")],
            ..plugin("alpha@test", "alpha-plugin", "alpha-plugin")
        },
        LoadedPlugin {
            mcp_servers: HashMap::from([("beta".to_string(), http_server("https://beta"))]),
            apps: vec![connector("connector_example"), connector("connector_gmail")],
            ..plugin("beta@test", "beta-plugin", "beta-plugin")
        },
        plugin("empty@test", "empty-plugin", "empty-plugin"),
        LoadedPlugin {
            enabled: false,
            skill_roots: vec![codex_home.path().join("disabled-plugin/skills").abs()],
            apps: vec![connector("connector_hidden")],
            ..plugin("disabled@test", "disabled-plugin", "disabled-plugin")
        },
        LoadedPlugin {
            apps: vec![connector("connector_broken")],
            error: Some("failed to load".to_string()),
            ..plugin("broken@test", "broken-plugin", "broken-plugin")
        },
    ]);

    assert_eq!(
        outcome.capability_summaries(),
        &[
            PluginCapabilitySummary {
                has_skills: true,
                ..summary("skills@test", "skills-plugin")
            },
            PluginCapabilitySummary {
                mcp_server_names: vec!["alpha".to_string()],
                app_connector_ids: vec![connector("connector_example")],
                ..summary("alpha@test", "alpha-plugin")
            },
            PluginCapabilitySummary {
                mcp_server_names: vec!["beta".to_string()],
                app_connector_ids: vec![
                    connector("connector_example"),
                    connector("connector_gmail"),
                ],
                ..summary("beta@test", "beta-plugin")
            },
        ]
    );
}
