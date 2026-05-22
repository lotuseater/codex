use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use codex_config::types::McpServerConfig;
use codex_config::types::McpServerTransportConfig;
use codex_features::Feature;
use codex_login::CodexAuth;
use codex_models_manager::bundled_models_response;
use codex_protocol::dynamic_tools::DynamicToolCallOutputContentItem;
use codex_protocol::dynamic_tools::DynamicToolResponse;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use codex_core_test_runtime::apps_test_server::AppsTestServer;
use codex_core_test_runtime::assert_regex_match;
use codex_core_test_runtime::responses;
use codex_core_test_runtime::responses::ResponseMock;
use codex_core_test_runtime::responses::ResponsesRequest;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_custom_tool_call;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::stdio_server_bin;
use codex_core_test_runtime::test_codex::TestCodex;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::test_codex::turn_permission_fields;
use codex_core_test_runtime::wait_for_event;
use codex_core_test_runtime::wait_for_event_match;
use pretty_assertions::assert_eq;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;
use wiremock::MockServer;

fn custom_tool_output_items(req: &ResponsesRequest, call_id: &str) -> Vec<Value> {
    match req.custom_tool_call_output(call_id).get("output") {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::String(text)) => {
            vec![serde_json::json!({ "type": "input_text", "text": text })]
        }
        _ => panic!("custom tool output should be serialized as text or content items"),
    }
}

fn tool_names(body: &Value) -> Vec<String> {
    body.get("tools")
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| {
                    tool.get("name")
                        .or_else(|| tool.get("type"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn function_tool_output_items(req: &ResponsesRequest, call_id: &str) -> Vec<Value> {
    match req.function_call_output(call_id).get("output") {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::String(text)) => {
            vec![serde_json::json!({ "type": "input_text", "text": text })]
        }
        _ => panic!("function tool output should be serialized as text or content items"),
    }
}

fn text_item(items: &[Value], index: usize) -> &str {
    items[index]
        .get("text")
        .and_then(Value::as_str)
        .expect("content item should be input_text")
}

fn extract_running_cell_id(text: &str) -> String {
    text.strip_prefix("Script running with cell ID ")
        .and_then(|rest| rest.split('\n').next())
        .expect("running header should contain a cell ID")
        .to_string()
}

fn wait_for_file_source(path: &Path) -> Result<String> {
    let quoted_path = shlex::try_join([path.to_string_lossy().as_ref()])?;
    let command = format!("if [ -f {quoted_path} ]; then printf ready; fi");
    Ok(format!(
        r#"while ((await tools.exec_command({{ cmd: {command:?} }})).output !== "ready") {{
}}"#
    ))
}

fn custom_tool_output_body_and_success(
    req: &ResponsesRequest,
    call_id: &str,
) -> (String, Option<bool>) {
    let (content, success) = req
        .custom_tool_call_output_content_and_success(call_id)
        .expect("custom tool output should be present");
    let items = custom_tool_output_items(req, call_id);
    let text_items = items
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>();
    let output = match text_items.as_slice() {
        [] => content.unwrap_or_default(),
        [only] => (*only).to_string(),
        [_, rest @ ..] => rest.concat(),
    };
    (output, success)
}

fn custom_tool_output_last_non_empty_text(req: &ResponsesRequest, call_id: &str) -> Option<String> {
    match req.custom_tool_call_output(call_id).get("output") {
        Some(Value::String(text)) if !text.trim().is_empty() => Some(text.clone()),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .rfind(|text| !text.trim().is_empty())
            .map(str::to_string),
        Some(Value::String(_))
        | Some(Value::Object(_))
        | Some(Value::Number(_))
        | Some(Value::Bool(_))
        | Some(Value::Null)
        | None => None,
    }
}

async fn run_code_mode_turn(
    server: &MockServer,
    prompt: &str,
    code: &str,
    include_apply_patch: bool,
) -> Result<(TestCodex, ResponseMock)> {
    let mut builder = test_codex()
        .with_model("test-gpt-5.1-codex")
        .with_config(move |config| {
            let _ = config.features.enable(Feature::CodeMode);
            config.include_apply_patch_tool = include_apply_patch;
        });
    let test = builder.build(server).await?;

    responses::mount_sse_once(
        server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call("call-1", "exec", code),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let second_mock = responses::mount_sse_once(
        server,
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    test.submit_turn(prompt).await?;
    Ok((test, second_mock))
}

async fn run_code_mode_turn_with_rmcp(
    server: &MockServer,
    prompt: &str,
    code: &str,
) -> Result<(TestCodex, ResponseMock)> {
    run_code_mode_turn_with_rmcp_model(server, prompt, code, "test-gpt-5.1-codex").await
}

async fn run_code_mode_turn_with_rmcp_model(
    server: &MockServer,
    prompt: &str,
    code: &str,
    model: &'static str,
) -> Result<(TestCodex, ResponseMock)> {
    run_code_mode_turn_with_rmcp_config(server, prompt, code, model, /*code_mode_only*/ false).await
}

async fn run_code_mode_turn_with_rmcp_mode(
    server: &MockServer,
    prompt: &str,
    code: &str,
    code_mode_only: bool,
) -> Result<(TestCodex, ResponseMock)> {
    run_code_mode_turn_with_rmcp_config(server, prompt, code, "test-gpt-5.1-codex", code_mode_only)
        .await
}

async fn run_code_mode_turn_with_rmcp_config(
    server: &MockServer,
    prompt: &str,
    code: &str,
    model: &'static str,
    code_mode_only: bool,
) -> Result<(TestCodex, ResponseMock)> {
    let rmcp_test_server_bin = stdio_server_bin()?;
    let mut builder = test_codex().with_model(model).with_config(move |config| {
        let _ = if code_mode_only {
            config.features.enable(Feature::CodeModeOnly)
        } else {
            config.features.enable(Feature::CodeMode)
        };

        let mut servers = config.mcp_servers.get().clone();
        servers.insert(
            "rmcp".to_string(),
            McpServerConfig {
                transport: McpServerTransportConfig::Stdio {
                    command: rmcp_test_server_bin,
                    args: Vec::new(),
                    env: Some(HashMap::from([(
                        "MCP_TEST_VALUE".to_string(),
                        "propagated-env".to_string(),
                    )])),
                    env_vars: Vec::new(),
                    cwd: None,
                },
                experimental_environment: None,
                enabled: true,
                required: false,
                supports_parallel_tool_calls: false,
                disabled_reason: None,
                startup_timeout_sec: Some(Duration::from_secs(10)),
                tool_timeout_sec: None,
                default_tools_approval_mode: None,
                enabled_tools: None,
                disabled_tools: None,
                scopes: None,
                oauth_resource: None,
                tools: HashMap::new(),
            },
        );
        config
            .mcp_servers
            .set(servers)
            .expect("test mcp servers should accept any configuration");
    });
    let test = builder.build(server).await?;

    responses::mount_sse_once(
        server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call("call-1", "exec", code),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let second_mock = responses::mount_sse_once(
        server,
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    test.submit_turn(prompt).await?;
    Ok((test, second_mock))
}
