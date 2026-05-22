#![cfg(not(target_os = "windows"))]
#![allow(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]

use crate::search_tool_support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_indexes_only_enabled_non_app_mcp_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let echo_call_id = "tool-search-echo";
    let image_call_id = "tool-search-image";
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_tool_search_call(
                    echo_call_id,
                    &json!({
                        "query": "Echo back the provided message and include environment data.",
                        "limit": 8,
                    }),
                ),
                ev_tool_search_call(
                    image_call_id,
                    &json!({
                        "query": "Return a single image content block.",
                        "limit": 8,
                    }),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let rmcp_test_server_bin = stdio_server_bin()?;
    let mut builder =
        configured_builder(apps_server.chatgpt_base_url.clone()).with_config(move |config| {
            let mut servers = config.mcp_servers.get().clone();
            servers.insert(
                "rmcp".to_string(),
                McpServerConfig {
                    transport: McpServerTransportConfig::Stdio {
                        command: rmcp_test_server_bin,
                        args: Vec::new(),
                        env: None,
                        env_vars: Vec::new(),
                        cwd: None,
                    },
                    experimental_environment: None,
                    enabled: true,
                    required: false,
                    disabled_reason: None,
                    startup_timeout_sec: Some(Duration::from_secs(10)),
                    tool_timeout_sec: None,
                    default_tools_approval_mode: None,
                    enabled_tools: Some(vec!["echo".to_string(), "image".to_string()]),
                    disabled_tools: Some(vec!["image".to_string()]),
                    scopes: None,
                    oauth: None,
                    oauth_resource: None,
                    supports_parallel_tool_calls: false,
                    tools: HashMap::new(),
                },
            );
            config
                .mcp_servers
                .set(servers)
                .expect("test mcp servers should accept any configuration");
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "Find the rmcp echo and image tools.",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = mock.requests();
    assert_eq!(requests.len(), 2);

    let first_request_tools = tool_names(&requests[0].body_json());
    assert!(
        first_request_tools
            .iter()
            .any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "first request should advertise tool_search: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools
            .iter()
            .any(|name| name == "mcp__rmcp__echo"),
        "non-app MCP tools should be hidden before search in large-search mode: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools.iter().any(|name| name == "mcp__rmcp__"),
        "non-app MCP namespace should be hidden before search in large-search mode: {first_request_tools:?}"
    );

    let echo_tools = tool_search_output_tools(&requests[1], echo_call_id);
    let echo_output = json!({ "tools": echo_tools });
    let rmcp_echo_tool = namespace_child_tool(&echo_output, "mcp__rmcp__", "echo")
        .expect("tool_search should return rmcp echo as a namespace child tool");
    assert_eq!(
        rmcp_echo_tool.get("type").and_then(Value::as_str),
        Some("function")
    );

    let image_tools = tool_search_output_tools(&requests[1], image_call_id);
    let found_rmcp_image_tool = image_tools
        .iter()
        .filter(|tool| tool.get("name").and_then(Value::as_str) == Some("mcp__rmcp__"))
        .flat_map(|namespace| namespace.get("tools").and_then(Value::as_array))
        .flatten()
        .any(|tool| tool.get("name").and_then(Value::as_str).is_some());
    assert!(
        !found_rmcp_image_tool,
        "disabled non-app MCP tools should not be searchable: {image_tools:?}"
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_surfaced_mcp_tool_errors_are_returned_to_model() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let search_call_id = "tool-search-rmcp-echo";
    let tool_call_id = "rmcp-echo-error";
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_tool_search_call(
                    search_call_id,
                    &json!({
                        "query": "Echo back the provided message and include environment data.",
                        "limit": 8,
                    }),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_function_call_with_namespace(tool_call_id, "mcp__rmcp__", "echo", "{}"),
                ev_completed("resp-2"),
            ]),
            sse(vec![
                ev_response_created("resp-3"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-3"),
            ]),
        ],
    )
    .await;

    let rmcp_test_server_bin = stdio_server_bin()?;
    let mut builder =
        configured_builder(apps_server.chatgpt_base_url.clone()).with_config(move |config| {
            config
                .features
                .enable(Feature::ToolSearchAlwaysDeferMcpTools)
                .expect("test config should allow feature update");
            let mut servers = config.mcp_servers.get().clone();
            servers.insert(
                "rmcp".to_string(),
                McpServerConfig {
                    transport: McpServerTransportConfig::Stdio {
                        command: rmcp_test_server_bin,
                        args: Vec::new(),
                        env: None,
                        env_vars: Vec::new(),
                        cwd: None,
                    },
                    experimental_environment: None,
                    enabled: true,
                    required: false,
                    disabled_reason: None,
                    startup_timeout_sec: Some(Duration::from_secs(10)),
                    tool_timeout_sec: None,
                    default_tools_approval_mode: None,
                    enabled_tools: Some(vec!["echo".to_string()]),
                    disabled_tools: None,
                    scopes: None,
                    oauth: None,
                    oauth_resource: None,
                    supports_parallel_tool_calls: false,
                    tools: HashMap::new(),
                },
            );
            config
                .mcp_servers
                .set(servers)
                .expect("test mcp servers should accept any configuration");
        });
    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "Find the rmcp echo tool and call it.".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let EventMsg::McpToolCallEnd(end) = wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::McpToolCallEnd(_))
    })
    .await
    else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };
    assert_eq!(end.call_id, tool_call_id);
    assert!(!end.is_success());
    let tool_error = end
        .result
        .as_ref()
        .expect_err("rmcp echo error should stay in the MCP result");
    assert!(
        tool_error.contains("tool call error:")
            && tool_error.contains("missing field")
            && tool_error.contains("message"),
        "MCP invocation should report the execution failure: {tool_error}"
    );

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = mock.requests();
    assert_eq!(requests.len(), 3);

    let first_request_tools = tool_names(&requests[0].body_json());
    assert!(
        first_request_tools
            .iter()
            .any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "first request should advertise tool_search: {first_request_tools:?}"
    );
    assert!(
        !first_request_tools.iter().any(|name| name == "mcp__rmcp__"),
        "deferred rmcp namespace should not be directly exposed before search: {first_request_tools:?}"
    );

    assert!(
        tool_search_output_has_namespace_child(&requests[1], search_call_id, "mcp__rmcp__", "echo"),
        "tool_search should return the rmcp echo tool"
    );

    let output = requests[2].function_call_output(tool_call_id);
    let output_text = match output.get("output") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        other => panic!("unexpected MCP error output payload: {other:?}"),
    };
    assert!(
        output_text.contains("missing field") && output_text.contains("message"),
        "MCP error output should be model visible: {output_text}"
    );
    assert!(
        !output_text.contains("unsupported call"),
        "search-surfaced MCP calls should not fall through to unsupported call: {output_text}"
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_search_uses_non_app_mcp_server_instructions_as_namespace_description() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let search_call_id = "tool-search-echo";
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_tool_search_call(
                    search_call_id,
                    &json!({
                        "query": "Echo back the provided message and include environment data.",
                        "limit": 8,
                    }),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let rmcp_test_server_bin = stdio_server_bin()?;
    let mut builder =
        configured_builder(apps_server.chatgpt_base_url.clone()).with_config(move |config| {
            let mut servers = config.mcp_servers.get().clone();
            servers.insert(
                "rmcp".to_string(),
                McpServerConfig {
                    transport: McpServerTransportConfig::Stdio {
                        command: rmcp_test_server_bin,
                        args: Vec::new(),
                        env: None,
                        env_vars: Vec::new(),
                        cwd: None,
                    },
                    experimental_environment: None,
                    enabled: true,
                    required: false,
                    disabled_reason: None,
                    startup_timeout_sec: Some(Duration::from_secs(10)),
                    tool_timeout_sec: None,
                    default_tools_approval_mode: None,
                    enabled_tools: Some(vec!["echo".to_string()]),
                    disabled_tools: None,
                    scopes: None,
                    oauth: None,
                    oauth_resource: None,
                    supports_parallel_tool_calls: false,
                    tools: HashMap::new(),
                },
            );
            config
                .mcp_servers
                .set(servers)
                .expect("test mcp servers should accept any configuration");
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "Find the rmcp echo tool.",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = mock.requests();
    assert_eq!(requests.len(), 2);

    let tools = tool_search_output_tools(&requests[1], search_call_id);
    let rmcp_namespace = tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some("mcp__rmcp__"))
        .expect("tool_search should return the rmcp namespace");
    assert_eq!(
        rmcp_namespace.get("description").and_then(Value::as_str),
        Some("Use these tools to exercise the rmcp test server.")
    );

    Ok(())
}
