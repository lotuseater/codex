use super::support::*;

#[tokio::test]
async fn realtime_conversation_requires_feature_flag() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let responses_server = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    let realtime_server = start_websocket_server(vec![vec![]]).await;

    let codex_home = TempDir::new()?;
    create_config_toml(
        codex_home.path(),
        &responses_server.uri(),
        realtime_server.uri(),
        /*realtime_enabled*/ false,
        StartupContextConfig::Generated,
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let thread_start_request_id = mcp
        .send_thread_start_request(ThreadStartParams::default())
        .await?;
    let thread_start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(thread_start_request_id)),
    )
    .await??;
    let thread_start: ThreadStartResponse = to_response(thread_start_response)?;

    let start_request_id = mcp
        .send_thread_realtime_start_request(ThreadRealtimeStartParams {
            thread_id: thread_start.thread.id.clone(),
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
        })
        .await?;
    let error = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    assert_invalid_request(
        error,
        format!(
            "thread {} does not support realtime conversation",
            thread_start.thread.id
        ),
    );

    realtime_server.shutdown().await;
    Ok(())
}
