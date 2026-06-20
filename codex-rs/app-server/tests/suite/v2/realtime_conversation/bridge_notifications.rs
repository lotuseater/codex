use super::support::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn realtime_conversation_streams_v2_notifications() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let responses_server = create_mock_responses_server_sequence_unchecked(vec![
        create_final_assistant_message_sse_response("delegated")?,
    ])
    .await;
    let realtime_server = start_websocket_server(vec![vec![
        vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_backend", "instructions": "backend prompt" }
        })],
        vec![],
        vec![
            json!({
                "type": "response.output_audio.delta",
                "delta": "AQID",
                "sample_rate": 24_000,
                "channels": 1,
                "samples_per_channel": 512
            }),
            json!({
                "type": "conversation.item.added",
                "item": {
                    "type": "message",
                    "role": "assistant",
                    "content": [{ "type": "text", "text": "hi" }]
                }
            }),
            json!({
                "type": "conversation.item.input_audio_transcription.delta",
                "delta": "delegate now"
            }),
            json!({
                "type": "response.output_text.delta",
                "delta": "working"
            }),
            json!({
                "type": "response.output_text.done",
                "text": "working on it"
            }),
            json!({
                "type": "conversation.item.done",
                "item": {
                    "id": "item_assistant_1",
                    "type": "message",
                    "role": "assistant",
                    "content": [{ "type": "output_text", "text": "working on it" }]
                }
            }),
            json!({
                "type": "conversation.item.done",
                "item": {
                    "id": "item_2",
                    "type": "function_call",
                    "name": "background_agent",
                    "call_id": "handoff_1",
                    "arguments": "{\"input_transcript\":\"delegate now\"}"
                }
            }),
            json!({
                "type": "error",
                "message": "upstream boom"
            }),
        ],
        vec![],
    ]])
    .await;

    let codex_home = TempDir::new()?;
    create_config_toml(
        codex_home.path(),
        &responses_server.uri(),
        realtime_server.uri(),
        /*realtime_enabled*/ true,
        StartupContextConfig::Generated,
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;
    login_with_api_key(&mut mcp, "sk-test-key").await?;

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
            prompt: None,
            realtime_session_id: None,
            transport: None,
            voice: Some(RealtimeVoice::Cedar),
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let _: ThreadRealtimeStartResponse = to_response(start_response)?;

    let started =
        read_notification::<ThreadRealtimeStartedNotification>(&mut mcp, "thread/realtime/started")
            .await?;
    assert_eq!(started.thread_id, thread_start.thread.id);
    assert!(started.realtime_session_id.is_some());
    assert_eq!(started.version, RealtimeConversationVersion::V2);

    let startup_context_request = realtime_server
        .wait_for_request(/*connection_index*/ 0, /*request_index*/ 0)
        .await;
    assert_eq!(
        startup_context_request.body_json()["type"].as_str(),
        Some("session.update")
    );
    assert_eq!(
        startup_context_request.body_json()["session"]["audio"]["output"]["voice"],
        "cedar"
    );
    assert_eq!(
        startup_context_request.body_json()["session"]["output_modalities"],
        json!(["audio"])
    );
    let startup_context_instructions =
        startup_context_request.body_json()["session"]["instructions"]
            .as_str()
            .context("expected startup context instructions")?
            .to_string();
    assert!(startup_context_instructions.starts_with("backend prompt"));
    assert!(startup_context_instructions.contains(STARTUP_CONTEXT_HEADER));

    let audio_append_request_id = mcp
        .send_thread_realtime_append_audio_request(ThreadRealtimeAppendAudioParams {
            thread_id: started.thread_id.clone(),
            audio: ThreadRealtimeAudioChunk {
                data: "BQYH".to_string(),
                sample_rate: 24_000,
                num_channels: 1,
                samples_per_channel: Some(480),
                item_id: None,
            },
        })
        .await?;
    let audio_append_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(audio_append_request_id)),
    )
    .await??;
    let _: ThreadRealtimeAppendAudioResponse = to_response(audio_append_response)?;

    let text_append_request_id = mcp
        .send_thread_realtime_append_text_request(ThreadRealtimeAppendTextParams {
            thread_id: started.thread_id.clone(),
            text: "hello".to_string(),
        })
        .await?;
    let text_append_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(text_append_request_id)),
    )
    .await??;
    let _: ThreadRealtimeAppendTextResponse = to_response(text_append_response)?;

    let output_audio = read_notification::<ThreadRealtimeOutputAudioDeltaNotification>(
        &mut mcp,
        "thread/realtime/outputAudio/delta",
    )
    .await?;
    assert_eq!(output_audio.audio.data, "AQID");
    assert_eq!(output_audio.audio.sample_rate, 24_000);
    assert_eq!(output_audio.audio.num_channels, 1);
    assert_eq!(output_audio.audio.samples_per_channel, Some(512));

    let item_added = read_notification::<ThreadRealtimeItemAddedNotification>(
        &mut mcp,
        "thread/realtime/itemAdded",
    )
    .await?;
    assert_eq!(item_added.thread_id, output_audio.thread_id);
    assert_eq!(item_added.item["type"], json!("message"));

    let first_transcript_delta = read_notification::<ThreadRealtimeTranscriptDeltaNotification>(
        &mut mcp,
        "thread/realtime/transcript/delta",
    )
    .await?;
    assert_eq!(first_transcript_delta.thread_id, output_audio.thread_id);
    assert_eq!(first_transcript_delta.role, "user");
    assert_eq!(first_transcript_delta.delta, "delegate now");

    let second_transcript_delta = read_notification::<ThreadRealtimeTranscriptDeltaNotification>(
        &mut mcp,
        "thread/realtime/transcript/delta",
    )
    .await?;
    assert_eq!(second_transcript_delta.thread_id, output_audio.thread_id);
    assert_eq!(second_transcript_delta.role, "assistant");
    assert_eq!(second_transcript_delta.delta, "working");

    let final_transcript_done = read_notification::<ThreadRealtimeTranscriptDoneNotification>(
        &mut mcp,
        "thread/realtime/transcript/done",
    )
    .await?;
    assert_eq!(final_transcript_done.thread_id, output_audio.thread_id);
    assert_eq!(final_transcript_done.role, "assistant");
    assert_eq!(final_transcript_done.text, "working on it");

    let handoff_item_added = read_notification::<ThreadRealtimeItemAddedNotification>(
        &mut mcp,
        "thread/realtime/itemAdded",
    )
    .await?;
    assert_eq!(handoff_item_added.thread_id, output_audio.thread_id);
    assert_eq!(handoff_item_added.item["type"], json!("handoff_request"));
    assert_eq!(handoff_item_added.item["handoff_id"], json!("handoff_1"));
    assert_eq!(handoff_item_added.item["item_id"], json!("item_2"));
    assert_eq!(
        handoff_item_added.item["input_transcript"],
        json!("delegate now")
    );
    assert_eq!(
        handoff_item_added.item["active_transcript"],
        json!([
            {"role": "user", "text": "delegate now"},
            {"role": "assistant", "text": "working on it"}
        ])
    );

    let realtime_error =
        read_notification::<ThreadRealtimeErrorNotification>(&mut mcp, "thread/realtime/error")
            .await?;
    assert_eq!(realtime_error.thread_id, output_audio.thread_id);
    assert_eq!(realtime_error.message, "upstream boom");

    let closed =
        read_notification::<ThreadRealtimeClosedNotification>(&mut mcp, "thread/realtime/closed")
            .await?;
    assert_eq!(closed.thread_id, output_audio.thread_id);
    assert_eq!(closed.reason.as_deref(), Some("error"));

    let connections = realtime_server.connections();
    assert_eq!(connections.len(), 1);
    let connection = &connections[0];
    assert_eq!(connection.len(), 3);
    assert_eq!(
        connection[0].body_json()["type"].as_str(),
        Some("session.update")
    );
    assert_eq!(
        connection[0].body_json()["session"]["instructions"].as_str(),
        Some(startup_context_instructions.as_str()),
    );
    let mut request_types = [
        connection[1].body_json()["type"]
            .as_str()
            .context("expected websocket request type")?
            .to_string(),
        connection[2].body_json()["type"]
            .as_str()
            .context("expected websocket request type")?
            .to_string(),
    ];
    request_types.sort();
    assert_eq!(
        request_types,
        [
            "conversation.item.create".to_string(),
            "input_audio_buffer.append".to_string(),
        ]
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn realtime_conversation_stop_emits_closed_notification() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let responses_server = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    let realtime_server = start_websocket_server(vec![vec![
        vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_backend", "instructions": "backend prompt" }
        })],
        vec![],
    ]])
    .await;

    let codex_home = TempDir::new()?;
    create_config_toml(
        codex_home.path(),
        &responses_server.uri(),
        realtime_server.uri(),
        /*realtime_enabled*/ true,
        StartupContextConfig::Generated,
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;
    login_with_api_key(&mut mcp, "sk-test-key").await?;

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
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let _: ThreadRealtimeStartResponse = to_response(start_response)?;

    let started =
        read_notification::<ThreadRealtimeStartedNotification>(&mut mcp, "thread/realtime/started")
            .await?;

    let stop_request_id = mcp
        .send_thread_realtime_stop_request(ThreadRealtimeStopParams {
            thread_id: started.thread_id.clone(),
        })
        .await?;
    let stop_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(stop_request_id)),
    )
    .await??;
    let _: ThreadRealtimeStopResponse = to_response(stop_response)?;

    let closed =
        read_notification::<ThreadRealtimeClosedNotification>(&mut mcp, "thread/realtime/closed")
            .await?;
    assert_eq!(closed.thread_id, started.thread_id);
    assert!(matches!(
        closed.reason.as_deref(),
        Some("requested" | "transport_closed")
    ));

    realtime_server.shutdown().await;
    Ok(())
}
