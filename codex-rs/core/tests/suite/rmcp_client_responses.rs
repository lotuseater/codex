#![allow(clippy::expect_used)]

use crate::rmcp_client_support::*;
use anyhow::Context as _;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_test_value)]
async fn stdio_image_responses_round_trip() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;

    let call_id = "img-1";
    let server_name = "rmcp";
    let namespace = format!("mcp__{server_name}__");

    // First stream: model decides to call the image tool.
    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(call_id, &namespace, "image", "{}"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    // Second stream: after tool execution, assistant emits a message and completes.
    let final_mock = mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-1", "rmcp image tool completed successfully."),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    // Build the stdio rmcp server and pass the image as data URL so it can construct ImageContent.
    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(
                    rmcp_test_server_bin,
                    Some(HashMap::from([(
                        "MCP_TEST_IMAGE_DATA_URL".to_string(),
                        OPENAI_PNG.to_string(),
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
    wait_for_mcp_server(&fixture, server_name).await?;

    fixture
        .codex
        .submit(read_only_user_turn(&fixture, "call the rmcp image tool"))
        .await?;

    // Wait for tool begin/end and final completion.
    let begin_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallBegin(_))
    })
    .await;
    let EventMsg::McpToolCallBegin(begin) = begin_event else {
        unreachable!("begin");
    };
    assert_eq!(
        begin,
        McpToolCallBeginEvent {
            call_id: call_id.to_string(),
            invocation: McpInvocation {
                server: server_name.to_string(),
                tool: "image".to_string(),
                arguments: Some(json!({})),
            },
            mcp_app_resource_uri: None,
        },
    );

    let end_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallEnd(_))
    })
    .await;
    let EventMsg::McpToolCallEnd(end) = end_event else {
        unreachable!("end");
    };
    assert_eq!(end.call_id, call_id);
    assert_eq!(
        end.invocation,
        McpInvocation {
            server: server_name.to_string(),
            tool: "image".to_string(),
            arguments: Some(json!({})),
        }
    );
    let result = end.result.expect("rmcp image tool should return success");
    assert_eq!(result.is_error, Some(false));
    assert_eq!(result.content.len(), 1);
    let base64_only = OPENAI_PNG
        .strip_prefix("data:image/png;base64,")
        .expect("data url prefix");
    let entry = result.content[0].as_object().expect("content object");
    assert_eq!(entry.get("type"), Some(&json!("image")));
    assert_eq!(entry.get("mimeType"), Some(&json!("image/png")));
    assert_eq!(entry.get("data"), Some(&json!(base64_only)));

    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let output_item = final_mock.single_request().function_call_output(call_id);
    assert_eq!(output_item["type"], "function_call_output");
    assert_eq!(output_item["call_id"], call_id);
    let output = output_item["output"]
        .as_array()
        .expect("image MCP output should be content items");
    assert_eq!(output.len(), 2);
    assert_wall_time_header(
        output[0]["text"]
            .as_str()
            .expect("first MCP image output item should be wall-time text"),
    );
    assert_eq!(
        output[1],
        json!({
            "type": "input_image",
            "image_url": OPENAI_PNG,
            "detail": "high"
        })
    );
    server.verify().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_test_value)]
async fn stdio_image_responses_preserve_original_detail_metadata() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;

    let call_id = "img-original-detail-1";
    let server_name = "rmcp";
    let namespace = format!("mcp__{server_name}__");

    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(
                call_id,
                &namespace,
                "image_scenario",
                r#"{"scenario":"image_only_original_detail"}"#,
            ),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    let final_mock = mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-1", "rmcp original-detail image completed."),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_model("gpt-5.3-codex")
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
        .codex
        .submit(read_only_user_turn(
            &fixture,
            "call the rmcp image_scenario tool",
        ))
        .await?;

    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let output_item = final_mock.single_request().function_call_output(call_id);
    let output = output_item["output"]
        .as_array()
        .expect("image MCP output should be content items");
    assert_eq!(output.len(), 2);
    assert_wall_time_header(
        output[0]["text"]
            .as_str()
            .expect("first MCP image output item should be wall-time text"),
    );
    assert_eq!(
        output[1],
        json!({
            "type": "input_image",
            "image_url": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
            "detail": "original",
        })
    );

    server.verify().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_test_value)]
async fn stdio_image_responses_are_sanitized_for_text_only_model() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;

    let call_id = "img-text-only-1";
    let server_name = "rmcp";
    let namespace = format!("mcp__{server_name}__");
    let text_only_model_slug = "rmcp-text-only-model";

    let models_mock = mount_models_once(
        &server,
        ModelsResponse {
            models: vec![ModelInfo {
                slug: text_only_model_slug.to_string(),
                display_name: "RMCP Text Only".to_string(),
                description: Some("Test model without image input support".to_string()),
                default_reasoning_level: None,
                supported_reasoning_levels: vec![ReasoningEffortPreset {
                    effort: codex_protocol::openai_models::ReasoningEffort::Medium,
                    description: "Medium".to_string(),
                }],
                shell_type: ConfigShellToolType::Default,
                visibility: ModelVisibility::List,
                supported_in_api: true,
                priority: 1,
                additional_speed_tiers: Vec::new(),
                service_tiers: Vec::new(),
                upgrade: None,
                base_instructions: "base instructions".to_string(),
                model_messages: None,
                supports_reasoning_summaries: false,
                default_reasoning_summary: ReasoningSummary::Auto,
                support_verbosity: false,
                default_verbosity: None,
                availability_nux: None,
                apply_patch_tool_type: None,
                web_search_tool_type: Default::default(),
                truncation_policy: TruncationPolicyConfig::bytes(/*limit*/ 10_000),
                supports_parallel_tool_calls: false,
                supports_image_detail_original: false,
                context_window: Some(272_000),
                max_context_window: None,
                auto_compact_token_limit: None,
                effective_context_window_percent: 95,
                experimental_supported_tools: Vec::new(),
                input_modalities: vec![InputModality::Text],
                used_fallback_model_metadata: false,
                supports_search_tool: false,
            }],
        },
    )
    .await;

    // First stream: model decides to call the image tool.
    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(call_id, &namespace, "image", "{}"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    // Second stream: after tool execution, assistant emits a message and completes.
    let final_mock = mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-1", "rmcp image tool completed successfully."),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(
                    rmcp_test_server_bin,
                    Some(HashMap::from([(
                        "MCP_TEST_IMAGE_DATA_URL".to_string(),
                        OPENAI_PNG.to_string(),
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
        .thread_manager
        .get_models_manager()
        .list_models(RefreshStrategy::Online)
        .await;
    assert_eq!(models_mock.requests().len(), 1);

    fixture
        .codex
        .submit(read_only_user_turn_with_model(
            &fixture,
            "call the rmcp image tool",
            text_only_model_slug.to_string(),
        ))
        .await?;

    wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallBegin(_))
    })
    .await;
    wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallEnd(_))
    })
    .await;
    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let output_item = final_mock.single_request().function_call_output(call_id);
    let output_text = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("function_call_output output should be a JSON string");
    let wrapped_payload = split_wall_time_wrapped_output(output_text);
    let output_json: Value = serde_json::from_str(wrapped_payload)
        .expect("function_call_output output should be valid JSON");
    assert_eq!(
        output_json,
        json!([{
            "type": "text",
            "text": "<image content omitted because you do not support image input>"
        }])
    );
    server.verify().await;
    Ok(())
}
