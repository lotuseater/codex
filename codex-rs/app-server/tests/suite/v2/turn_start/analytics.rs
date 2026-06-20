use super::*;
use pretty_assertions::assert_eq;

use crate::analytics::mount_analytics_capture;
use crate::analytics::wait_for_analytics_event;

#[tokio::test]
async fn turn_start_tracks_turn_event_analytics() -> Result<()> {
    let responses = vec![create_final_assistant_message_sse_response("Done")?];
    let server = create_mock_responses_server_sequence_unchecked(responses).await;

    let codex_home = TempDir::new()?;
    write_mock_responses_config_toml_with_chatgpt_base_url(
        codex_home.path(),
        &server.uri(),
        &server.uri(),
    )?;
    mount_analytics_capture(&server, codex_home.path()).await?;

    let mut mcp = McpProcess::new_without_managed_config(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let thread_req = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
            thread_source: Some(ThreadSource::User),
            ..Default::default()
        })
        .await?;
    let thread_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(thread_req)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(thread_resp)?;

    let turn_req = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread.id.clone(),
            input: vec![V2UserInput::Image {
                url: "https://example.com/a.png".to_string(),
                detail: None,
            }],
            ..Default::default()
        })
        .await?;
    let turn_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_req)),
    )
    .await??;
    let TurnStartResponse { turn } = to_response::<TurnStartResponse>(turn_resp)?;

    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;

    let event = wait_for_analytics_event(&server, DEFAULT_READ_TIMEOUT, "codex_turn_event").await?;
    assert_eq!(event["event_params"]["thread_id"], thread.id);
    assert_eq!(event["event_params"]["turn_id"], turn.id);
    assert_eq!(
        event["event_params"]["app_server_client"]["product_client_id"],
        DEFAULT_CLIENT_NAME
    );
    assert_eq!(event["event_params"]["model"], "mock-model");
    assert_eq!(event["event_params"]["model_provider"], "mock_provider");
    assert_eq!(event["event_params"]["sandbox_policy"], "read_only");
    assert_eq!(event["event_params"]["ephemeral"], false);
    assert_eq!(event["event_params"]["thread_source"], "user");
    assert_eq!(event["event_params"]["initialization_mode"], "new");
    assert_eq!(
        event["event_params"]["subagent_source"],
        serde_json::Value::Null
    );
    assert_eq!(
        event["event_params"]["parent_thread_id"],
        serde_json::Value::Null
    );
    assert_eq!(event["event_params"]["num_input_images"], 1);
    assert_eq!(event["event_params"]["status"], "completed");
    assert!(event["event_params"]["started_at"].as_u64().is_some());
    assert!(event["event_params"]["completed_at"].as_u64().is_some());
    assert!(event["event_params"]["duration_ms"].as_u64().is_some());
    assert_eq!(event["event_params"]["input_tokens"], 0);
    assert_eq!(event["event_params"]["cached_input_tokens"], 0);
    assert_eq!(event["event_params"]["output_tokens"], 0);
    assert_eq!(event["event_params"]["reasoning_output_tokens"], 0);
    assert_eq!(event["event_params"]["total_tokens"], 0);

    Ok(())
}
