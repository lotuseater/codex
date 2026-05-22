#![allow(clippy::expect_used)]

use anyhow::Context as _;
use crate::rmcp_client_support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn streamable_http_tool_call_round_trip() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    // Phase 1: script the model responses so Codex will call the MCP echo tool
    // and then complete the turn after the tool result is returned.
    let server = responses::start_mock_server().await;

    let call_id = "call-456";
    let server_name = "rmcp_http";
    let namespace = format!("mcp__{server_name}__");

    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(
                call_id,
                &namespace,
                "echo",
                "{\"message\":\"ping\"}",
            ),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message(
                "msg-1",
                "rmcp streamable http echo tool completed successfully.",
            ),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    // Phase 2: start the Streamable HTTP MCP test server in the active
    // placement. In full CI this may be the remote executor container; locally
    // it is a host process.
    let expected_env_value = "propagated-env-http";
    let Some(http_server) =
        start_streamable_http_test_server(expected_env_value, /*expected_token*/ None).await?
    else {
        return Ok(());
    };
    let server_url = http_server.url().to_string();

    // Phase 3: configure Codex with the Streamable HTTP MCP server and build a
    // fixture that selects remote MCP placement only when the remote test
    // environment is active.
    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                McpServerTransportConfig::StreamableHttp {
                    url: server_url,
                    bearer_token_env_var: None,
                    http_headers: None,
                    env_http_headers: None,
                },
                TestMcpServerOptions {
                    experimental_environment: remote_aware_experimental_environment(),
                    ..Default::default()
                },
            );
        })
        .build_remote_aware(&server)
        .await?;
    // Phase 4: submit the user turn that should trigger the MCP tool call.
    fixture
        .codex
        .submit(read_only_user_turn(
            &fixture,
            "call the rmcp streamable http echo tool",
        ))
        .await?;

    // Phase 5: assert Codex begins the expected tool invocation.
    let begin_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallBegin(_))
    })
    .await;

    let EventMsg::McpToolCallBegin(begin) = begin_event else {
        unreachable!("event guard guarantees McpToolCallBegin");
    };
    assert_eq!(begin.invocation.server, server_name);
    assert_eq!(begin.invocation.tool, "echo");

    // Phase 6: assert the tool result proves the server handled the request and
    // propagated the expected environment value.
    let end_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallEnd(_))
    })
    .await;
    let EventMsg::McpToolCallEnd(end) = end_event else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };

    let result = end
        .result
        .as_ref()
        .expect("rmcp echo tool should return success");
    assert_eq!(result.is_error, Some(false));
    assert!(
        result.content.is_empty(),
        "content should default to an empty array"
    );

    let structured = result
        .structured_content
        .as_ref()
        .expect("structured content");
    let Value::Object(map) = structured else {
        panic!("structured content should be an object: {structured:?}");
    };
    let echo_value = map
        .get("echo")
        .and_then(Value::as_str)
        .expect("echo payload present");
    assert_eq!(echo_value, "ECHOING: ping");
    let env_value = map
        .get("env")
        .and_then(Value::as_str)
        .expect("env snapshot inserted");
    assert_eq!(env_value, expected_env_value);

    // Phase 7: verify the scripted model calls were consumed and clean up the
    // placement-aware MCP server.
    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    server.verify().await;

    http_server.shutdown().await;

    Ok(())
}

/// This test writes to a fallback credentials file in CODEX_HOME.
/// Ideally, we wouldn't need to serialize the test but it's much more cumbersome to wire CODEX_HOME through the code.
#[test]
#[serial(codex_home)]
fn streamable_http_with_oauth_round_trip() -> anyhow::Result<()> {
    const TEST_STACK_SIZE_BYTES: usize = 8 * 1024 * 1024;

    let handle = std::thread::Builder::new()
        .name("streamable_http_with_oauth_round_trip".to_string())
        .stack_size(TEST_STACK_SIZE_BYTES)
        .spawn(|| -> anyhow::Result<()> {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()?;
            runtime.block_on(streamable_http_with_oauth_round_trip_impl())
        })?;

    match handle.join() {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!(
            "streamable_http_with_oauth_round_trip thread panicked"
        )),
    }
}

#[allow(clippy::expect_used)]
async fn streamable_http_with_oauth_round_trip_impl() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    // Phase 1: script the model responses so Codex will call the OAuth-backed
    // MCP echo tool and then finish the turn after receiving the result.
    let server = responses::start_mock_server().await;

    let call_id = "call-789";
    let server_name = "rmcp_http_oauth";
    let namespace = format!("mcp__{server_name}__");

    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(
                call_id,
                &namespace,
                "echo",
                "{\"message\":\"ping\"}",
            ),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message(
                "msg-1",
                "rmcp streamable http oauth echo tool completed successfully.",
            ),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    // Phase 2: start the Streamable HTTP MCP test server with bearer-token
    // enforcement enabled so the client must use stored OAuth credentials.
    let expected_env_value = "propagated-env-http-oauth";
    let expected_token = "initial-access-token";
    let client_id = "test-client-id";
    let refresh_token = "initial-refresh-token";
    let Some(http_server) =
        start_streamable_http_test_server(expected_env_value, Some(expected_token)).await?
    else {
        return Ok(());
    };
    let server_url = http_server.url().to_string();

    // Phase 3: seed an isolated CODEX_HOME with fallback OAuth tokens for this
    // server so the test does not share credentials with other suite cases.
    let temp_home = Arc::new(tempdir()?);
    let _codex_home_guard = EnvVarGuard::set("CODEX_HOME", temp_home.path().as_os_str());
    write_fallback_oauth_tokens(
        temp_home.path(),
        server_name,
        &server_url,
        client_id,
        expected_token,
        refresh_token,
    )?;

    // Phase 4: configure Codex with the OAuth-backed Streamable HTTP MCP
    // server and build the fixture in the active local or remote-aware mode.
    let fixture = test_codex()
        .with_home(temp_home.clone())
        .with_config(move |config| {
            // Keep OAuth credentials isolated to this test home because Bazel
            // runs the full core suite in one process.
            config.mcp_oauth_credentials_store_mode = serde_json::from_value(json!("file"))
                .expect("`file` should deserialize as OAuthCredentialsStoreMode");
            insert_mcp_server(
                config,
                server_name,
                McpServerTransportConfig::StreamableHttp {
                    url: server_url,
                    bearer_token_env_var: None,
                    http_headers: None,
                    env_http_headers: None,
                },
                TestMcpServerOptions {
                    experimental_environment: remote_aware_experimental_environment(),
                    ..Default::default()
                },
            );
        })
        .build_remote_aware(&server)
        .await?;
    // Phase 5: wait for MCP startup before the turn is submitted, which keeps
    // failures tied to server startup/discovery.
    wait_for_mcp_server(&fixture, server_name).await?;

    // Phase 6: submit the user turn that should invoke the OAuth-backed tool.
    fixture
        .codex
        .submit(read_only_user_turn(
            &fixture,
            "call the rmcp streamable http oauth echo tool",
        ))
        .await?;

    // Phase 7: assert Codex begins the expected tool invocation.
    let begin_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallBegin(_))
    })
    .await;

    let EventMsg::McpToolCallBegin(begin) = begin_event else {
        unreachable!("event guard guarantees McpToolCallBegin");
    };
    assert_eq!(begin.invocation.server, server_name);
    assert_eq!(begin.invocation.tool, "echo");

    // Phase 8: assert the tool result proves the authenticated request reached
    // the server and preserved the expected environment value.
    let end_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallEnd(_))
    })
    .await;
    let EventMsg::McpToolCallEnd(end) = end_event else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };

    let result = end
        .result
        .as_ref()
        .expect("rmcp echo tool should return success");
    assert_eq!(result.is_error, Some(false));
    assert!(
        result.content.is_empty(),
        "content should default to an empty array"
    );

    let structured = result
        .structured_content
        .as_ref()
        .expect("structured content");
    let Value::Object(map) = structured else {
        panic!("structured content should be an object: {structured:?}");
    };
    let echo_value = map
        .get("echo")
        .and_then(Value::as_str)
        .expect("echo payload present");
    assert_eq!(echo_value, "ECHOING: ping");
    let env_value = map
        .get("env")
        .and_then(Value::as_str)
        .expect("env snapshot inserted");
    assert_eq!(env_value, expected_env_value);

    // Phase 9: verify the scripted model calls were consumed and clean up the
    // placement-aware MCP server.
    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    server.verify().await;

    http_server.shutdown().await;

    Ok(())
}

/// Starts the Streamable HTTP MCP test server in the active test placement.
