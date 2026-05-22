#![allow(clippy::expect_used)]

use anyhow::Context as _;
pub(crate) use anyhow::ensure;
pub(crate) use std::collections::HashMap;
pub(crate) use std::ffi::OsStr;
pub(crate) use std::ffi::OsString;
pub(crate) use std::fs;
pub(crate) use std::net::SocketAddr;
pub(crate) use std::net::TcpListener;
pub(crate) use std::path::Path;
pub(crate) use std::path::PathBuf;
pub(crate) use std::process::Command as StdCommand;
pub(crate) use std::sync::Arc;
pub(crate) use std::sync::Mutex;
pub(crate) use std::time::Duration;
pub(crate) use std::time::SystemTime;
pub(crate) use std::time::UNIX_EPOCH;

pub(crate) use codex_config::types::McpServerConfig;
pub(crate) use codex_config::types::McpServerEnvVar;
pub(crate) use codex_config::types::McpServerTransportConfig;
pub(crate) use codex_core::config::Config;
pub(crate) use codex_exec_server::CreateDirectoryOptions;
pub(crate) use codex_exec_server::Environment;
pub(crate) use codex_exec_server::HttpRequestParams;
pub(crate) use codex_login::CodexAuth;
pub(crate) use codex_mcp::MCP_SANDBOX_STATE_META_CAPABILITY;
pub(crate) use codex_models_manager::manager::RefreshStrategy;

pub(crate) use codex_core_test_runtime::assert_regex_match;
pub(crate) use codex_core_test_runtime::remote_env_env_var;
pub(crate) use codex_core_test_runtime::responses;
pub(crate) use codex_core_test_runtime::responses::mount_models_once;
pub(crate) use codex_core_test_runtime::responses::mount_sse_once;
pub(crate) use codex_core_test_runtime::skip_if_no_network;
pub(crate) use codex_core_test_runtime::stdio_server_bin;
pub(crate) use codex_core_test_runtime::test_codex::TestCodex;
pub(crate) use codex_core_test_runtime::test_codex::test_codex;
pub(crate) use codex_core_test_runtime::test_codex::turn_permission_fields;
pub(crate) use codex_core_test_runtime::wait_for_event;
pub(crate) use codex_core_test_runtime::wait_for_event_with_timeout;
pub(crate) use codex_protocol::config_types::ReasoningSummary;
pub(crate) use codex_protocol::models::PermissionProfile;
pub(crate) use codex_protocol::openai_models::ConfigShellToolType;
pub(crate) use codex_protocol::openai_models::InputModality;
pub(crate) use codex_protocol::openai_models::ModelInfo;
pub(crate) use codex_protocol::openai_models::ModelVisibility;
pub(crate) use codex_protocol::openai_models::ModelsResponse;
pub(crate) use codex_protocol::openai_models::ReasoningEffortPreset;
pub(crate) use codex_protocol::openai_models::TruncationPolicyConfig;
pub(crate) use codex_protocol::protocol::AskForApproval;
pub(crate) use codex_protocol::protocol::EventMsg;
pub(crate) use codex_protocol::protocol::McpInvocation;
pub(crate) use codex_protocol::protocol::McpToolCallBeginEvent;
pub(crate) use codex_protocol::protocol::Op;
pub(crate) use codex_protocol::user_input::UserInput;
pub(crate) use codex_utils_cargo_bin::cargo_bin;
pub(crate) use reqwest::Client;
pub(crate) use reqwest::StatusCode;
pub(crate) use serde_json::Value;
pub(crate) use serde_json::json;
pub(crate) use serial_test::serial;
pub(crate) use tempfile::tempdir;
pub(crate) use tokio::process::Child;
pub(crate) use tokio::process::Command;
pub(crate) use tokio::time::Instant;
pub(crate) use tokio::time::sleep;
pub(crate) use wiremock::MockServer;

pub(crate) static OPENAI_PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAD0AAAA9CAYAAAAeYmHpAAAE6klEQVR4Aeyau44UVxCGx1fZsmRLlm3Zoe0XcGQ5cUiCCIgJeS9CHgAhMkISQnIuGQgJEkBcxLW+nqnZ6uqqc+nuWRC7q/P3qetf9e+MtOwyX25O4Nep6JPyop++0qev9HrfgZ+F6r2DuB/vHOrt/UIkqdDHYvujOW6fO7h/CNEI+a5jc+pBR8uy0jVFsziYu5HtfSUk+Io34q921hLNctFSX0gwww+S8wce8K1LfCU+cYW4888aov8NxqvQILUPPReLOrm6zyLxa4i+6VZuFbJo8d1MOHZm+7VUtB/aIvhPWc/3SWg49JcwFLlHxuXKjtyloo+YNhuW3VS+WPBuUEMvCFKjEDVgFBQHXrnazpqiSxNZCkQ1kYiozsbm9Oz7l4i2Il7vGccGNWAc3XosDrZe/9P3ZnMmzHNEQw4smf8RQ87XEAMsC7Az0Au+dgXerfH4+sHvEc0SYGic8WBBUGqFH2gN7yDrazy7m2pbRTeRmU3+MjZmr1h6LJgPbGy23SI6GlYT0brQ71IY8Us4PNQCm+zepSbaD2BY9xCaAsD9IIj/IzFmKMSdHHonwdZATbTnYREf6/VZGER98N9yCWIvXQwXDoDdhZJoT8jwLnJXDB9w4Sb3e6nK5ndzlkTLnP3JBu4LKkbrYrU69gCVceV0JvpyuW1xlsUVngzhwMetn/XamtTORF9IO5YnWNiyeF9zCAfqR3fUW+vZZKLtgP+ts8BmQRBREAdRDhH3o8QuRh/YucNFz2BEjxbRN6LGzphfKmvP6v6QhqIQyZ8XNJ0W0X83MR1PEcJBNO2KC2Z1TW/v244scp9FwRViZxIOBF0Lctk7ZVSavdLvRlV1hz/ysUi9sr8CIcB3nvWBwA93ykTz18eAYxQ6N/K2DkPA1lv3iXCwmDUT7YkjIby9siXueIJj9H+pzSqJ9oIuJWTUgSSt4WO7o/9GGg0viR4VinNRUDoIj34xoCd6pxD3aK3zfdbnx5v1J3ZNNEJsE0sBG7N27ReDrJc4sFxz7dI/ZAbOmmiKvHBitQXpAdR6+F7v+/ol/tOouUV01EeMZQF2BoQDn6dP4XNr+j9GZEtEK1/L8pFw7bd3a53tsTa7WD+054jOFmPg1XBKPQgnqFfmFcy32ZRvjmiIIQTYFvyDxQ8nH8WIwwGwlyDjDznnilYyFr6njrlZwsKkBpO59A7OwgdzPEWRm+G+oeb7IfyNuzjEEVLrOVxJsxvxwF8kmCM6I2QYmJunz4u4TrADpfl7mlbRTWQ7VmrBzh3+C9f6Grc3YoGN9dg/SXFthpRsT6vobfXRs2VBlgBHXVMLHjDNbIZv1sZ9+X3hB09cXdH1JKViyG0+W9bWZDa/r2f9zAFR71sTzGpMSWz2iI4YssWjWo3REy1MDGjdwe5e0dFSiAC1JakBvu4/CUS8Eh6dqHdU0Or0ioY3W5ClSqDXAy7/6SRfgw8vt4I+tbvvNtFT2kVDhY5+IGb1rCqYaXNF08vSALsXCPmt0kQNqJT1p5eI1mkIV/BxCY1z85lOzeFbPBQHURkkPTlwTYK9gTVE25l84IbFFN+YJDHjdpn0gq6mrHht0dkcjbM4UL9283O5p77GN+SPW/QwVB4IUYg7Or+Kp7naR6qktP98LNF2UxWo9yObPIT9KYg+hK4i56no4rfnM0qeyFf6AwAAAP//trwR3wAAAAZJREFUAwBZ0sR75itw5gAAAABJRU5ErkJggg==";

pub(crate) fn assert_wall_time_line(line: &str) {
    assert_regex_match(r"^Wall time: [0-9]+(?:\.[0-9]+)? seconds$", line);
}

pub(crate) fn split_wall_time_wrapped_output(output: &str) -> &str {
    let Some((wall_time, rest)) = output.split_once('\n') else {
        panic!("wall-time output should contain an Output section: {output}");
    };
    assert_wall_time_line(wall_time);
    let Some(output) = rest.strip_prefix("Output:\n") else {
        panic!("wall-time output should contain Output marker: {output}");
    };
    output
}

pub(crate) fn assert_wall_time_header(output: &str) {
    let Some((wall_time, marker)) = output.split_once('\n') else {
        panic!("wall-time header should contain an Output marker: {output}");
    };
    assert_wall_time_line(wall_time);
    assert_eq!(marker, "Output:");
}

pub(crate) fn read_only_user_turn(fixture: &TestCodex, text: impl Into<String>) -> Op {
    read_only_user_turn_with_model(fixture, text, fixture.session_configured.model.clone())
}

pub(crate) fn read_only_user_turn_with_model(
    fixture: &TestCodex,
    text: impl Into<String>,
    model: String,
) -> Op {
    let cwd = fixture.cwd.path().to_path_buf();
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::read_only(), cwd.as_path());
    Op::UserTurn {
        items: vec![UserInput::Text {
            text: text.into(),
            text_elements: Vec::new(),
        }],
        final_output_json_schema: None,
        cwd,
        approval_policy: AskForApproval::Never,
        approvals_reviewer: None,
        sandbox_policy,
        permission_profile,
        model,
        effort: None,
        summary: None,
        service_tier: None,
        context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
        collaboration_mode: None,
        personality: None,
        environments: None,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum McpCallEvent {
    Begin(String),
    End(String),
}

const REMOTE_MCP_ENVIRONMENT: &str = "remote";

pub(crate) fn remote_aware_experimental_environment() -> Option<String> {
    // These tests run locally in normal CI and against the Docker-backed
    // executor in full-ci. Match that shared test environment instead of
    // parameterizing each stdio MCP test with its own local/remote cases.
    std::env::var_os(remote_env_env_var()).map(|_| REMOTE_MCP_ENVIRONMENT.to_string())
}

/// Returns the stdio MCP test server command path for the active test placement.
///
/// Local test runs can execute the host-built test binary directly. Remote-aware
/// runs start MCP stdio through the executor inside Docker, so the host path
/// would be meaningless to the process that actually launches the server. When
/// the remote test environment is active, copy the binary into the executor
/// container and return that in-container path instead.
pub(crate) fn remote_aware_stdio_server_bin() -> anyhow::Result<String> {
    let bin = stdio_server_bin()?;
    let Some(container_name) = remote_env_container_name()? else {
        return Ok(bin);
    };

    // Keep the Docker path rewrite scoped to tests that use `build_remote_aware`.
    // Other MCP tests still start their stdio server from the orchestrator test
    // process, even when the full-ci remote env is present.
    //
    // Remote-aware MCP tests run the executor inside Docker. The stdio test
    // server is built on the host, so hand the executor a copied in-container
    // path instead of the host build artifact path.
    // Several remote-aware MCP tests can run in parallel; give each copied
    // binary its own path so one test cannot replace another test's executable.
    copy_binary_to_remote_env(&container_name, Path::new(&bin), "test_stdio_server")
}

/// Returns the Docker container used by remote-aware MCP tests, when active.
pub(crate) fn remote_env_container_name() -> anyhow::Result<Option<String>> {
    let Some(container_name) = std::env::var_os(remote_env_env_var()) else {
        return Ok(None);
    };
    Ok(Some(container_name.into_string().map_err(|value| {
        anyhow::anyhow!("remote env container name must be utf-8: {value:?}")
    })?))
}

/// Builds a collision-resistant in-container path for copied test binaries.
pub(crate) fn unique_remote_path(binary_name: &str) -> anyhow::Result<String> {
    let unique_suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(format!(
        "/tmp/codex-remote-env/{binary_name}-{}-{unique_suffix}",
        std::process::id()
    ))
}

/// Copies a host-built helper binary into the remote test container.
pub(crate) fn copy_binary_to_remote_env(
    container_name: &str,
    host_path: &Path,
    binary_name: &str,
) -> anyhow::Result<String> {
    let remote_path = unique_remote_path(binary_name)?;
    let mkdir_output = StdCommand::new("docker")
        .args([
            "exec",
            container_name,
            "mkdir",
            "-p",
            "/tmp/codex-remote-env",
        ])
        .output()
        .context("create remote MCP test binary directory")?;
    ensure!(
        mkdir_output.status.success(),
        "docker mkdir remote MCP test binary directory failed: stdout={} stderr={}",
        String::from_utf8_lossy(&mkdir_output.stdout).trim(),
        String::from_utf8_lossy(&mkdir_output.stderr).trim()
    );

    let container_target = format!("{container_name}:{remote_path}");
    let copy_output = StdCommand::new("docker")
        .arg("cp")
        .arg(host_path)
        .arg(&container_target)
        .output()
        .with_context(|| {
            format!(
                "copy {} to remote MCP test env",
                host_path.to_string_lossy()
            )
        })?;
    ensure!(
        copy_output.status.success(),
        "docker cp {binary_name} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&copy_output.stdout).trim(),
        String::from_utf8_lossy(&copy_output.stderr).trim()
    );

    let chmod_output = StdCommand::new("docker")
        .args(["exec", container_name, "chmod", "+x", remote_path.as_str()])
        .output()
        .with_context(|| format!("mark remote {binary_name} executable"))?;
    ensure!(
        chmod_output.status.success(),
        "docker chmod {binary_name} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&chmod_output.stdout).trim(),
        String::from_utf8_lossy(&chmod_output.stderr).trim()
    );

    Ok(remote_path)
}

pub(crate) async fn wait_for_mcp_server(
    fixture: &TestCodex,
    server_name: &str,
) -> anyhow::Result<()> {
    let startup_event = wait_for_event_with_timeout(
        &fixture.codex,
        |ev| match ev {
            EventMsg::McpStartupComplete(summary) => {
                summary.ready.iter().any(|server| server == server_name)
                    || summary
                        .failed
                        .iter()
                        .any(|failure| failure.server == server_name)
                    || summary.cancelled.iter().any(|server| server == server_name)
            }
            _ => false,
        },
        Duration::from_secs(70),
    )
    .await;
    let EventMsg::McpStartupComplete(summary) = startup_event else {
        unreachable!("event guard guarantees McpStartupComplete");
    };
    if let Some(failure) = summary
        .failed
        .iter()
        .find(|failure| failure.server == server_name)
    {
        let error = &failure.error;
        anyhow::bail!("MCP server {server_name} failed to start: {error}");
    }
    if summary.cancelled.iter().any(|server| server == server_name) {
        anyhow::bail!("MCP server {server_name} startup was cancelled");
    }
    ensure!(
        summary.ready.iter().any(|server| server == server_name),
        "expected MCP server {server_name} to be ready; startup summary: {summary:?}"
    );
    Ok(())
}

#[derive(Default)]
pub(crate) struct TestMcpServerOptions {
    pub(crate) experimental_environment: Option<String>,
    pub(crate) supports_parallel_tool_calls: bool,
    pub(crate) tool_timeout_sec: Option<Duration>,
}

pub(crate) fn stdio_transport(
    command: String,
    env: Option<HashMap<String, String>>,
    env_vars: Vec<McpServerEnvVar>,
) -> McpServerTransportConfig {
    stdio_transport_with_cwd(command, env, env_vars, /*cwd*/ None)
}

pub(crate) fn stdio_transport_with_cwd(
    command: String,
    env: Option<HashMap<String, String>>,
    env_vars: Vec<McpServerEnvVar>,
    cwd: Option<PathBuf>,
) -> McpServerTransportConfig {
    McpServerTransportConfig::Stdio {
        command,
        args: Vec::new(),
        env,
        env_vars,
        cwd,
    }
}

pub(crate) fn insert_mcp_server(
    config: &mut Config,
    server_name: &str,
    transport: McpServerTransportConfig,
    options: TestMcpServerOptions,
) {
    let mut servers = config.mcp_servers.get().clone();
    servers.insert(
        server_name.to_string(),
        McpServerConfig {
            transport,
            experimental_environment: options.experimental_environment,
            enabled: true,
            required: false,
            supports_parallel_tool_calls: options.supports_parallel_tool_calls,
            disabled_reason: None,
            startup_timeout_sec: Some(Duration::from_secs(10)),
            tool_timeout_sec: options.tool_timeout_sec,
            default_tools_approval_mode: None,
            enabled_tools: None,
            disabled_tools: None,
            scopes: None,
            oauth_resource: None,
            tools: HashMap::new(),
        },
    );
    if let Err(err) = config.mcp_servers.set(servers) {
        panic!("test mcp servers should accept any configuration: {err}");
    }
}

pub(crate) async fn call_cwd_tool(
    server: &MockServer,
    fixture: &TestCodex,
    server_name: &str,
    call_id: &str,
) -> anyhow::Result<Value> {
    let namespace = format!("mcp__{server_name}__");
    mount_sse_once(
        server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(call_id, &namespace, "cwd", r#"{}"#),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    mount_sse_once(
        server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-1", "rmcp cwd tool completed successfully."),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    fixture
        .codex
        .submit(read_only_user_turn(fixture, "call the rmcp cwd tool"))
        .await?;

    wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallBegin(_))
    })
    .await;
    let end_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallEnd(_))
    })
    .await;
    let EventMsg::McpToolCallEnd(end) = end_event else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };
    let structured_content = end
        .result
        .as_ref()
        .expect("rmcp cwd tool should return success")
        .structured_content
        .as_ref()
        .expect("structured content")
        .clone();

    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
    Ok(structured_content)
}

pub(crate) fn assert_cwd_tool_output(structured: &Value, expected_cwd: &Path) {
    let actual_cwd = structured
        .get("cwd")
        .and_then(Value::as_str)
        .expect("cwd tool should return a string cwd");

    if std::env::var_os(remote_env_env_var()).is_some() {
        assert_eq!(
            structured,
            &json!({
                "cwd": expected_cwd.to_string_lossy(),
            })
        );
        return;
    }

    // Local Windows can report the same absolute directory through an 8.3 path.
    // Canonical paths keep the assertion focused on cwd precedence.
    assert_eq!(
        Path::new(actual_cwd)
            .canonicalize()
            .expect("cwd tool path should exist"),
        expected_cwd
            .canonicalize()
            .expect("expected cwd should exist"),
    );
}

pub(crate) struct StreamableHttpTestServer {
    server_url: String,
    process: StreamableHttpTestServerProcess,
}

/// Tracks whether the Streamable HTTP test server runs on the host or remotely.
pub(crate) enum StreamableHttpTestServerProcess {
    Local(Child),
    Remote(RemoteStreamableHttpServer),
}

/// Remote Streamable HTTP server process and copied files to remove on drop.
pub(crate) struct RemoteStreamableHttpServer {
    container_name: String,
    pid: String,
    paths_to_remove: Vec<String>,
}

impl Drop for RemoteStreamableHttpServer {
    /// Stops the remote process and removes copied test artifacts best-effort.
    fn drop(&mut self) {
        self.kill();
        if self.paths_to_remove.is_empty() {
            return;
        }
        let script = format!("rm -f {}", self.paths_to_remove.join(" "));
        let _ = StdCommand::new("docker")
            .args(["exec", &self.container_name, "sh", "-lc", &script])
            .output();
    }
}

impl RemoteStreamableHttpServer {
    /// Stops the remote Streamable HTTP test server process.
    pub(crate) fn kill(&self) {
        let _ = StdCommand::new("docker")
            .args(["exec", &self.container_name, "kill", &self.pid])
            .output();
    }
}

impl StreamableHttpTestServer {
    /// Returns the MCP endpoint URL that Codex should connect to.
    pub(crate) fn url(&self) -> &str {
        &self.server_url
    }

    /// Stops the local or remote test server and waits for local process exit.
    pub(crate) async fn shutdown(mut self) {
        match &mut self.process {
            StreamableHttpTestServerProcess::Local(child) => match child.try_wait() {
                Ok(Some(_)) => {}
                Ok(None) => {
                    let _ = child.kill().await;
                }
                Err(error) => {
                    eprintln!("failed to check streamable http server status: {error}");
                    let _ = child.kill().await;
                }
            },
            StreamableHttpTestServerProcess::Remote(server) => {
                server.kill();
            }
        }
        if let StreamableHttpTestServerProcess::Local(child) = &mut self.process
            && let Err(error) = child.wait().await
        {
            eprintln!("failed to await streamable http server shutdown: {error}");
        }
    }
}

/// What this tests: Codex can discover and call a Streamable HTTP MCP tool in
/// both local and remote-aware placements, and the tool observes the expected
/// environment value from the server process that actually handled the request.

pub(crate) async fn start_streamable_http_test_server(
    expected_env_value: &str,
    expected_token: Option<&str>,
) -> anyhow::Result<Option<StreamableHttpTestServer>> {
    let rmcp_http_server_bin = match cargo_bin("test_streamable_http_server") {
        Ok(path) => path,
        Err(err) => {
            eprintln!("test_streamable_http_server binary not available, skipping test: {err}");
            return Ok(None);
        }
    };

    if let Some(container_name) = remote_env_container_name()? {
        return Ok(Some(
            start_remote_streamable_http_test_server(
                &container_name,
                &rmcp_http_server_bin,
                expected_env_value,
                expected_token,
            )
            .await?,
        ));
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    let bind_addr = format!("127.0.0.1:{port}");
    let server_url = format!("http://{bind_addr}/mcp");

    let mut command = Command::new(&rmcp_http_server_bin);
    command
        .kill_on_drop(true)
        .env("MCP_STREAMABLE_HTTP_BIND_ADDR", &bind_addr)
        .env("MCP_TEST_VALUE", expected_env_value);
    if let Some(expected_token) = expected_token {
        command.env("MCP_EXPECT_BEARER", expected_token);
    }
    let mut child = command.spawn()?;

    wait_for_local_streamable_http_server(&mut child, &server_url, Duration::from_secs(5)).await?;
    Ok(Some(StreamableHttpTestServer {
        server_url,
        process: StreamableHttpTestServerProcess::Local(child),
    }))
}

/// Starts the Streamable HTTP MCP test server inside the remote test container.
pub(crate) async fn start_remote_streamable_http_test_server(
    container_name: &str,
    rmcp_http_server_bin: &Path,
    expected_env_value: &str,
    expected_token: Option<&str>,
) -> anyhow::Result<StreamableHttpTestServer> {
    let remote_path = copy_binary_to_remote_env(
        container_name,
        rmcp_http_server_bin,
        "test_streamable_http_server",
    )?;
    let bound_addr_file = format!("{remote_path}.addr");
    let log_file = format!("{remote_path}.log");
    let mut env_assignments = vec![
        format!(
            "MCP_STREAMABLE_HTTP_BIND_ADDR={}",
            sh_single_quote("0.0.0.0:0")
        ),
        format!(
            "MCP_STREAMABLE_HTTP_BOUND_ADDR_FILE={}",
            sh_single_quote(&bound_addr_file)
        ),
        format!("MCP_TEST_VALUE={}", sh_single_quote(expected_env_value)),
    ];
    if let Some(expected_token) = expected_token {
        env_assignments.push(format!(
            "MCP_EXPECT_BEARER={}",
            sh_single_quote(expected_token)
        ));
    }

    let script = format!(
        "{} nohup {} > {} 2>&1 < /dev/null & echo $!",
        env_assignments.join(" "),
        sh_single_quote(&remote_path),
        sh_single_quote(&log_file)
    );
    let start_output = StdCommand::new("docker")
        .args(["exec", container_name, "sh", "-lc", &script])
        .output()
        .context("start remote streamable HTTP MCP test server")?;
    ensure!(
        start_output.status.success(),
        "docker start streamable HTTP MCP test server failed: stdout={} stderr={}",
        String::from_utf8_lossy(&start_output.stdout).trim(),
        String::from_utf8_lossy(&start_output.stderr).trim()
    );
    let pid = String::from_utf8(start_output.stdout)
        .context("remote streamable HTTP server pid must be utf-8")?
        .trim()
        .to_string();
    ensure!(
        !pid.is_empty(),
        "remote streamable HTTP server pid is empty"
    );

    let remote_bind_addr =
        wait_for_remote_bound_addr(container_name, &bound_addr_file, Duration::from_secs(5))
            .await?;
    let container_ip = remote_container_ip(container_name)?;
    let server_url = format!("http://{}:{}/mcp", container_ip, remote_bind_addr.port());
    // The orchestrator can see the Docker container IP, but the behavior under
    // test is whether the remote-side MCP client can reach it. Probe through
    // remote HTTP before handing the URL to the Codex fixture.
    wait_for_remote_streamable_http_server(&server_url, Duration::from_secs(5)).await?;
    if expected_token.is_some() {
        wait_for_streamable_http_metadata(&server_url, Duration::from_secs(5)).await?;
    }

    Ok(StreamableHttpTestServer {
        server_url,
        process: StreamableHttpTestServerProcess::Remote(RemoteStreamableHttpServer {
            container_name: container_name.to_string(),
            pid,
            paths_to_remove: vec![remote_path, bound_addr_file, log_file],
        }),
    })
}

/// Single-quotes a value for the small shell snippets sent through Docker.
pub(crate) fn sh_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Waits until the remote test server writes the socket address it bound to.
pub(crate) async fn wait_for_remote_bound_addr(
    container_name: &str,
    bound_addr_file: &str,
    timeout: Duration,
) -> anyhow::Result<SocketAddr> {
    let deadline = Instant::now() + timeout;
    loop {
        let output = StdCommand::new("docker")
            .args(["exec", container_name, "cat", bound_addr_file])
            .output()
            .context("read remote streamable HTTP server bound address")?;
        if output.status.success() {
            let bound_addr = String::from_utf8(output.stdout)
                .context("remote streamable HTTP bound address must be utf-8")?;
            return bound_addr
                .trim()
                .parse()
                .context("parse remote streamable HTTP bound address");
        }
        if Instant::now() >= deadline {
            return Err(anyhow::anyhow!(
                "timed out waiting for remote streamable HTTP bound address: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        sleep(Duration::from_millis(50)).await;
    }
}

/// Reads the container IP that the host-side test process can use.
pub(crate) fn remote_container_ip(container_name: &str) -> anyhow::Result<String> {
    let output = StdCommand::new("docker")
        .args([
            "inspect",
            "-f",
            "{{range .NetworkSettings.Networks}}{{println .IPAddress}}{{end}}",
            container_name,
        ])
        .output()
        .context("inspect remote MCP test container IP")?;
    ensure!(
        output.status.success(),
        "docker inspect remote MCP test container IP failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let inspect_output =
        String::from_utf8(output.stdout).context("remote MCP test container IP must be utf-8")?;
    let ip = inspect_output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_string();
    if ip.is_empty() {
        Ok("127.0.0.1".to_string())
    } else {
        Ok(ip)
    }
}

/// Waits for the local Streamable HTTP test server to publish OAuth metadata.
pub(crate) async fn wait_for_local_streamable_http_server(
    server_child: &mut Child,
    server_url: &str,
    timeout: Duration,
) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    let metadata_url = streamable_http_metadata_url(server_url);
    let client = Client::builder().no_proxy().build()?;
    loop {
        if let Some(status) = server_child.try_wait()? {
            return Err(anyhow::anyhow!(
                "streamable HTTP server exited early with status {status}"
            ));
        }

        let remaining = deadline.saturating_duration_since(Instant::now());

        if remaining.is_zero() {
            return Err(anyhow::anyhow!(
                "timed out waiting for streamable HTTP server metadata at {metadata_url}: deadline reached"
            ));
        }

        match tokio::time::timeout(remaining, client.get(&metadata_url).send()).await {
            Ok(Ok(response)) if response.status() == StatusCode::OK => return Ok(()),
            Ok(Ok(response)) => {
                if Instant::now() >= deadline {
                    return Err(anyhow::anyhow!(
                        "timed out waiting for streamable HTTP server metadata at {metadata_url}: HTTP {}",
                        response.status()
                    ));
                }
            }
            Ok(Err(error)) => {
                if Instant::now() >= deadline {
                    return Err(anyhow::anyhow!(
                        "timed out waiting for streamable HTTP server metadata at {metadata_url}: {error}"
                    ));
                }
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "timed out waiting for streamable HTTP server metadata at {metadata_url}: request timed out"
                ));
            }
        }

        sleep(Duration::from_millis(50)).await;
    }
}

/// Waits for the remote Streamable HTTP test server via remote HTTP.
pub(crate) async fn wait_for_remote_streamable_http_server(
    server_url: &str,
    timeout: Duration,
) -> anyhow::Result<()> {
    let websocket_url = std::env::var(REMOTE_EXEC_SERVER_URL_ENV_VAR).with_context(|| {
        format!("{REMOTE_EXEC_SERVER_URL_ENV_VAR} must be set for remote streamable HTTP MCP tests")
    })?;
    let environment = Environment::create_for_tests(Some(websocket_url))?;
    let http_client = environment.get_http_client();
    let metadata_url = streamable_http_metadata_url(server_url);
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(anyhow::anyhow!(
                "timed out waiting for remote streamable HTTP server metadata at {metadata_url}: deadline reached"
            ));
        }

        let request = HttpRequestParams {
            method: "GET".to_string(),
            url: metadata_url.clone(),
            headers: Vec::new(),
            body: None,
            timeout_ms: Some(remaining.as_millis().clamp(1, 1_000) as u64),
            request_id: "buffered-request".to_string(),
            stream_response: false,
        };
        match http_client.http_request(request).await {
            Ok(response) if response.status == StatusCode::OK.as_u16() => return Ok(()),
            Ok(response) => {
                if Instant::now() >= deadline {
                    return Err(anyhow::anyhow!(
                        "timed out waiting for remote streamable HTTP server metadata at {metadata_url}: HTTP {}",
                        response.status
                    ));
                }
            }
            Err(error) => {
                if Instant::now() >= deadline {
                    return Err(anyhow::anyhow!(
                        "timed out waiting for remote streamable HTTP server metadata at {metadata_url}: {error}"
                    ));
                }
            }
        }

        sleep(Duration::from_millis(50)).await;
    }
}

/// Waits for OAuth metadata from the host-side test process.
pub(crate) async fn wait_for_streamable_http_metadata(
    server_url: &str,
    timeout: Duration,
) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    let metadata_url = streamable_http_metadata_url(server_url);
    let client = Client::builder().no_proxy().build()?;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(anyhow::anyhow!(
                "timed out waiting for streamable HTTP server metadata at {metadata_url}: deadline reached"
            ));
        }

        match tokio::time::timeout(remaining, client.get(&metadata_url).send()).await {
            Ok(Ok(response)) if response.status() == StatusCode::OK => return Ok(()),
            Ok(Ok(response)) => {
                if Instant::now() >= deadline {
                    return Err(anyhow::anyhow!(
                        "timed out waiting for streamable HTTP server metadata at {metadata_url}: HTTP {}",
                        response.status()
                    ));
                }
            }
            Ok(Err(error)) => {
                if Instant::now() >= deadline {
                    return Err(anyhow::anyhow!(
                        "timed out waiting for streamable HTTP server metadata at {metadata_url}: {error}"
                    ));
                }
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "timed out waiting for streamable HTTP server metadata at {metadata_url}: request timed out"
                ));
            }
        }

        sleep(Duration::from_millis(50)).await;
    }
}

/// Builds the OAuth metadata URL for the test Streamable HTTP MCP endpoint.
pub(crate) fn streamable_http_metadata_url(server_url: &str) -> String {
    let base_url = server_url.strip_suffix("/mcp").unwrap_or(server_url);
    format!("{base_url}{STREAMABLE_HTTP_METADATA_PATH}")
}

pub(crate) fn write_fallback_oauth_tokens(
    home: &Path,
    server_name: &str,
    server_url: &str,
    client_id: &str,
    access_token: &str,
    refresh_token: &str,
) -> anyhow::Result<()> {
    let expires_at = SystemTime::now()
        .checked_add(Duration::from_secs(3600))
        .ok_or_else(|| anyhow::anyhow!("failed to compute expiry time"))?
        .duration_since(UNIX_EPOCH)?
        .as_millis() as u64;

    let store = serde_json::json!({
        "stub": {
            "server_name": server_name,
            "server_url": server_url,
            "client_id": client_id,
            "access_token": access_token,
            "expires_at": expires_at,
            "refresh_token": refresh_token,
            "scopes": ["profile"],
        }
    });

    let file_path = home.join(".credentials.json");
    fs::write(&file_path, serde_json::to_vec(&store)?)?;
    Ok(())
}

pub(crate) struct EnvVarGuard {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvVarGuard {
    pub(crate) fn set(key: &'static str, value: &std::ffi::OsStr) -> Self {
        let original = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.original {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
