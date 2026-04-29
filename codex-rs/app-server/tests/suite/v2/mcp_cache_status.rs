use anyhow::Result;
use app_test_support::McpProcess;
use app_test_support::to_response;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::McpCacheEntryState;
use codex_app_server_protocol::McpCacheStatusResponse;
use codex_app_server_protocol::RequestId;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[tokio::test]
async fn mcp_cache_status_reports_codex_apps_tools_cache_miss() -> Result<()> {
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path())?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_raw_request("mcp/cache/status", /*params*/ None)
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let response: McpCacheStatusResponse = to_response::<McpCacheStatusResponse>(response)?;

    assert!(response.deferred_mcp_loading_enabled);
    assert_eq!(response.codex_apps_tools.state, McpCacheEntryState::Missing);
    assert!(
        response
            .codex_apps_tools
            .path
            .contains("cache\\codex_apps_tools")
            || response
                .codex_apps_tools
                .path
                .contains("cache/codex_apps_tools")
    );
    assert_eq!(response.codex_apps_tools.schema_version, None);
    assert_eq!(response.codex_apps_tools.byte_size, None);
    assert_eq!(response.codex_apps_tools.modified_at, None);
    assert_eq!(response.codex_apps_tools.tool_count, None);

    Ok(())
}

fn create_config_toml(codex_home: &std::path::Path) -> std::io::Result<()> {
    std::fs::write(
        codex_home.join("config.toml"),
        r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "mock_provider"
suppress_unstable_features_warning = true

[features]
tool_search_always_defer_mcp_tools = true

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "http://127.0.0.1:9/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#,
    )
}
