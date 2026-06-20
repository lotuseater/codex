use super::support::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn realtime_text_output_modality_requests_text_output_and_final_transcript() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let responses_server = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    let realtime_server = start_websocket_server(vec![vec![vec![
        json!({
            "type": "session.updated",
            "session": { "id": "sess_text", "instructions": "backend prompt" }
        }),
        json!({
            "type": "response.output_text.delta",
            "delta": "hello "
        }),
        json!({
            "type": "response.output_text.delta",
            "delta": "world"
        }),
        json!({
            "type": "response.output_audio_transcript.done",
            "transcript": "hello world"
        }),
        json!({
            "type": "conversation.item.done",
            "item": {
                "id": "item_output_1",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "hello world"}]
            }
        }),
    ]]])
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
            output_modality: RealtimeOutputModality::Text,
            prompt: None,
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

    let session_update = realtime_server
        .wait_for_request(/*connection_index*/ 0, /*request_index*/ 0)
        .await;
    assert_eq!(
        session_update.body_json()["session"]["output_modalities"],
        json!(["text"])
    );

    let first_delta = read_notification::<ThreadRealtimeTranscriptDeltaNotification>(
        &mut mcp,
        "thread/realtime/transcript/delta",
    )
    .await?;
    let second_delta = read_notification::<ThreadRealtimeTranscriptDeltaNotification>(
        &mut mcp,
        "thread/realtime/transcript/delta",
    )
    .await?;
    let done = read_notification::<ThreadRealtimeTranscriptDoneNotification>(
        &mut mcp,
        "thread/realtime/transcript/done",
    )
    .await?;
    assert_eq!(
        vec![first_delta, second_delta],
        vec![
            ThreadRealtimeTranscriptDeltaNotification {
                thread_id: thread_start.thread.id.clone(),
                role: "assistant".to_string(),
                delta: "hello ".to_string(),
            },
            ThreadRealtimeTranscriptDeltaNotification {
                thread_id: thread_start.thread.id.clone(),
                role: "assistant".to_string(),
                delta: "world".to_string(),
            },
        ]
    );
    assert_eq!(
        done,
        ThreadRealtimeTranscriptDoneNotification {
            thread_id: thread_start.thread.id,
            role: "assistant".to_string(),
            text: "hello world".to_string(),
        }
    );
    assert!(
        timeout(
            Duration::from_millis(200),
            mcp.read_stream_until_notification_message("thread/realtime/transcript/done"),
        )
        .await
        .is_err(),
        "should not emit duplicate transcript done from audio transcript done"
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn realtime_list_voices_returns_supported_names() -> Result<()> {
    let codex_home = TempDir::new()?;
    create_config_toml(
        codex_home.path(),
        "http://127.0.0.1:1",
        "ws://127.0.0.1:1",
        /*realtime_enabled*/ true,
        StartupContextConfig::Generated,
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_thread_realtime_list_voices_request(ThreadRealtimeListVoicesParams {})
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let response: ThreadRealtimeListVoicesResponse = to_response(response)?;

    assert_eq!(
        response,
        ThreadRealtimeListVoicesResponse {
            voices: RealtimeVoicesList {
                v1: vec![
                    RealtimeVoice::Juniper,
                    RealtimeVoice::Maple,
                    RealtimeVoice::Spruce,
                    RealtimeVoice::Ember,
                    RealtimeVoice::Vale,
                    RealtimeVoice::Breeze,
                    RealtimeVoice::Arbor,
                    RealtimeVoice::Sol,
                    RealtimeVoice::Cove,
                ],
                v2: vec![
                    RealtimeVoice::Alloy,
                    RealtimeVoice::Ash,
                    RealtimeVoice::Ballad,
                    RealtimeVoice::Coral,
                    RealtimeVoice::Echo,
                    RealtimeVoice::Sage,
                    RealtimeVoice::Shimmer,
                    RealtimeVoice::Verse,
                    RealtimeVoice::Marin,
                    RealtimeVoice::Cedar,
                ],
                default_v1: RealtimeVoice::Cove,
                default_v2: RealtimeVoice::Marin,
            },
        }
    );

    Ok(())
}
