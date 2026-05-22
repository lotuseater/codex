#![allow(clippy::expect_used)]

use crate::rmcp_client_support::*;
use anyhow::Context as _;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_test_value)]
async fn stdio_server_round_trip() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;

    let call_id = "call-123";
    let server_name = "rmcp";
    let namespace = format!("mcp__{server_name}__");

    let call_mock = mount_sse_once(
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
    let final_mock = mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-1", "rmcp echo tool completed successfully."),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    let expected_env_value = "propagated-env";
    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(
                    rmcp_test_server_bin,
                    Some(HashMap::from([(
                        "MCP_TEST_VALUE".to_string(),
                        expected_env_value.to_string(),
                    )])),
                    Vec::new(),
                ),
                TestMcpServerOptions {
                    experimental_environment: remote_aware_experimental_environment(),
                    ..Default::default()
                },
            );
        })
        .build_remote_aware(&server)
        .await?;
    fixture
        .codex
        .submit(read_only_user_turn(&fixture, "call the rmcp echo tool"))
        .await?;

    let begin_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallBegin(_))
    })
    .await;

    let EventMsg::McpToolCallBegin(begin) = begin_event else {
        unreachable!("event guard guarantees McpToolCallBegin");
    };
    assert_eq!(begin.invocation.server, server_name);
    assert_eq!(begin.invocation.tool, "echo");

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

    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let output_item = final_mock.single_request().function_call_output(call_id);
    let request = call_mock.single_request();
    assert!(
        request.tool_by_name(&namespace, "echo").is_some(),
        "direct MCP tool should be sent as a namespace child tool: {:?}",
        request.body_json()
    );

    let output_text = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("function_call_output output should be a string");
    let wrapped_payload = split_wall_time_wrapped_output(output_text);
    let output_json: Value = serde_json::from_str(wrapped_payload)
        .expect("wrapped MCP output should preserve structured JSON");
    assert_eq!(output_json["echo"], "ECHOING: ping");
    assert_eq!(output_json["env"], expected_env_value);

    server.verify().await;

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn stdio_mcp_tool_call_includes_sandbox_state_meta() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;

    let call_id = "sandbox-meta-call";
    let server_name = "rmcp";
    let namespace = format!("mcp__{server_name}__");

    let call_mock = mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(call_id, &namespace, "sandbox_meta", "{}"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    let final_mock = mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-1", "rmcp sandbox meta completed successfully."),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;
    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(rmcp_test_server_bin, /*env*/ None, Vec::new()),
                TestMcpServerOptions {
                    experimental_environment: remote_aware_experimental_environment(),
                    ..Default::default()
                },
            );
        })
        .build_remote_aware(&server)
        .await?;

    wait_for_mcp_server(&fixture, server_name).await?;

    fixture
        .submit_turn_with_permission_profile(
            "call the rmcp sandbox_meta tool",
            PermissionProfile::read_only(),
        )
        .await?;

    let request = call_mock.single_request();
    assert!(
        request.tool_by_name(&namespace, "sandbox_meta").is_some(),
        "direct MCP tool should be sent as a namespace child tool: {:?}",
        request.body_json()
    );

    let output_item = final_mock.single_request().function_call_output(call_id);
    let output_text = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("function_call_output output should be a string");
    let wrapped_payload = split_wall_time_wrapped_output(output_text);
    let output_json: Value = serde_json::from_str(wrapped_payload)
        .expect("wrapped MCP output should preserve sandbox metadata JSON");
    let Value::Object(meta) = output_json else {
        panic!("sandbox_meta should return metadata object: {output_json:?}");
    };

    let sandbox_meta = meta
        .get(MCP_SANDBOX_STATE_META_CAPABILITY)
        .expect("sandbox state metadata should be present");
    let (sandbox_policy, _) =
        turn_permission_fields(PermissionProfile::read_only(), fixture.config.cwd.as_path());
    let expected_sandbox_policy = serde_json::to_value(&sandbox_policy)?;
    assert_eq!(
        sandbox_meta.get("sandboxPolicy"),
        Some(&expected_sandbox_policy)
    );
    assert_eq!(
        sandbox_meta.get("sandboxCwd").and_then(Value::as_str),
        fixture.config.cwd.as_path().to_str()
    );
    assert_eq!(sandbox_meta.get("useLegacyLandlock"), Some(&json!(false)));

    server.verify().await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stdio_mcp_parallel_tool_calls_default_false_runs_serially() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;

    let first_call_id = "sync-serial-1";
    let second_call_id = "sync-serial-2";
    let server_name = "rmcp";
    let namespace = format!("mcp__{server_name}__");
    let args = json!({ "sleep_after_ms": 100 }).to_string();

    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(first_call_id, &namespace, "sync", &args),
            responses::ev_function_call_with_namespace(second_call_id, &namespace, "sync", &args),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    let final_mock = mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-1", "rmcp sync tools completed successfully."),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(rmcp_test_server_bin, /*env*/ None, Vec::new()),
                TestMcpServerOptions {
                    experimental_environment: remote_aware_experimental_environment(),
                    tool_timeout_sec: Some(Duration::from_secs(2)),
                    ..Default::default()
                },
            );
        })
        .build_remote_aware(&server)
        .await?;
    fixture
        .codex
        .submit(read_only_user_turn(
            &fixture,
            "call the rmcp sync tool twice",
        ))
        .await?;

    let mut call_events = Vec::new();
    while call_events.len() < 4 {
        let event = wait_for_event(&fixture.codex, |ev| {
            matches!(
                ev,
                EventMsg::McpToolCallBegin(_) | EventMsg::McpToolCallEnd(_)
            )
        })
        .await;
        match event {
            EventMsg::McpToolCallBegin(begin) => {
                call_events.push(McpCallEvent::Begin(begin.call_id));
            }
            EventMsg::McpToolCallEnd(end) => {
                call_events.push(McpCallEvent::End(end.call_id));
            }
            _ => unreachable!("event guard guarantees MCP call events"),
        }
    }

    let event_index = |needle: McpCallEvent| {
        call_events
            .iter()
            .position(|event| event == &needle)
            .expect("expected MCP call event")
    };
    let first_begin = event_index(McpCallEvent::Begin(first_call_id.to_string()));
    let first_end = event_index(McpCallEvent::End(first_call_id.to_string()));
    let second_begin = event_index(McpCallEvent::Begin(second_call_id.to_string()));
    let second_end = event_index(McpCallEvent::End(second_call_id.to_string()));
    assert!(
        first_end < second_begin || second_end < first_begin,
        "default MCP tool calls should run serially; saw events: {call_events:?}"
    );

    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let request = final_mock.single_request();
    for call_id in [first_call_id, second_call_id] {
        let output_text = request
            .function_call_output_text(call_id)
            .expect("function_call_output present for rmcp sync call");
        let wrapped_payload = split_wall_time_wrapped_output(&output_text);
        let output_json: Value = serde_json::from_str(wrapped_payload)
            .expect("wrapped MCP output should preserve structured JSON");
        assert_eq!(output_json, json!({ "result": "ok" }));
    }

    server.verify().await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stdio_mcp_parallel_tool_calls_opt_in_runs_concurrently() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;

    let first_call_id = "sync-1";
    let second_call_id = "sync-2";
    let server_name = "rmcp";
    let namespace = format!("mcp__{server_name}__");
    let args = json!({
        "sleep_after_ms": 100,
        "barrier": {
            "id": "stdio-mcp-parallel-tool-calls",
            "participants": 2,
            "timeout_ms": 1_000
        }
    })
    .to_string();

    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(first_call_id, &namespace, "sync", &args),
            responses::ev_function_call_with_namespace(second_call_id, &namespace, "sync", &args),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    let final_mock = mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-1", "rmcp sync tools completed successfully."),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(rmcp_test_server_bin, /*env*/ None, Vec::new()),
                TestMcpServerOptions {
                    experimental_environment: remote_aware_experimental_environment(),
                    supports_parallel_tool_calls: true,
                    tool_timeout_sec: Some(Duration::from_secs(2)),
                },
            );
        })
        .build_remote_aware(&server)
        .await?;
    fixture
        .codex
        .submit(read_only_user_turn(
            &fixture,
            "call the rmcp sync tool twice",
        ))
        .await?;

    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let request = final_mock.single_request();
    for call_id in [first_call_id, second_call_id] {
        let output_text = request
            .function_call_output_text(call_id)
            .expect("function_call_output present for rmcp sync call");
        let wrapped_payload = split_wall_time_wrapped_output(&output_text);
        let output_json: Value = serde_json::from_str(wrapped_payload)
            .expect("wrapped MCP output should preserve structured JSON");
        assert_eq!(output_json, json!({ "result": "ok" }));
    }

    server.verify().await;

    Ok(())
}
