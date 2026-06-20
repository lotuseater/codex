use super::support::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn realtime_webrtc_start_emits_sdp_notification() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let responses_server = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    let call_capture = RealtimeCallRequestCapture::new();
    Mock::given(method("POST"))
        .and(path("/v1/realtime/calls"))
        .and(call_capture.clone())
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Location", "/v1/realtime/calls/rtc_app_test")
                .set_body_string("v=answer\r\n"),
        )
        .mount(&responses_server)
        .await;
    let realtime_server = start_websocket_server_with_headers(vec![WebSocketConnectionConfig {
        requests: vec![vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_webrtc", "instructions": "backend prompt" }
        })]],
        response_headers: Vec::new(),
        accept_delay: None,
        close_after_requests: false,
    }])
    .await;

    let codex_home = TempDir::new()?;
    create_config_toml(
        codex_home.path(),
        &responses_server.uri(),
        realtime_server.uri(),
        /*realtime_enabled*/ true,
        StartupContextConfig::Override("startup context"),
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

    let thread_id = thread_start.thread.id;
    let start_request_id = mcp
        .send_thread_realtime_start_request(ThreadRealtimeStartParams {
            thread_id: thread_id.clone(),
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: Some(ThreadRealtimeStartTransport::Webrtc {
                sdp: "v=offer\r\n".to_string(),
            }),
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
    assert_eq!(started.thread_id, thread_id);
    assert_eq!(started.version, RealtimeConversationVersion::V2);

    let sdp_notification =
        read_notification::<ThreadRealtimeSdpNotification>(&mut mcp, "thread/realtime/sdp").await?;
    assert_eq!(
        sdp_notification,
        ThreadRealtimeSdpNotification {
            thread_id: thread_id.clone(),
            sdp: "v=answer\r\n".to_string()
        }
    );

    let session_update = realtime_server
        .wait_for_request(/*connection_index*/ 0, /*request_index*/ 0)
        .await;
    assert_eq!(
        session_update.body_json()["type"].as_str(),
        Some("session.update")
    );
    assert!(
        session_update.body_json()["session"]["instructions"]
            .as_str()
            .context("expected session.update instructions")?
            .contains("startup context")
    );
    assert_eq!(
        realtime_server.single_handshake().uri(),
        "/v1/realtime?call_id=rtc_app_test"
    );

    let stop_request_id = mcp
        .send_thread_realtime_stop_request(ThreadRealtimeStopParams {
            thread_id: thread_id.clone(),
        })
        .await?;
    let stop_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(stop_request_id)),
    )
    .await??;
    let _: ThreadRealtimeStopResponse = to_response(stop_response)?;

    let closed_notification =
        read_notification::<ThreadRealtimeClosedNotification>(&mut mcp, "thread/realtime/closed")
            .await?;
    assert_eq!(closed_notification.thread_id, thread_id);
    assert!(
        matches!(
            closed_notification.reason.as_deref(),
            Some("requested" | "transport_closed")
        ),
        "unexpected close reason: {closed_notification:?}"
    );

    let request = call_capture.single_request();
    assert_eq!(request.url.path(), "/v1/realtime/calls");
    assert_eq!(request.url.query(), None);
    assert_eq!(
        request
            .headers
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("multipart/form-data; boundary=codex-realtime-call-boundary")
    );
    let body = String::from_utf8(request.body).context("multipart body should be utf-8")?;
    let session = r#"{"tool_choice":"auto","type":"realtime","model":"gpt-realtime-1.5","instructions":"backend prompt\n\nstartup context","output_modalities":["audio"],"audio":{"input":{"format":{"type":"audio/pcm","rate":24000},"noise_reduction":{"type":"near_field"},"transcription":{"model":"gpt-4o-mini-transcribe"},"turn_detection":{"type":"server_vad","interrupt_response":true,"create_response":true,"silence_duration_ms":500}},"output":{"format":{"type":"audio/pcm","rate":24000},"voice":"marin"}},"tools":[{"type":"function","name":"background_agent","description":"Send a user request to the background agent. Use this as the default action. Do not rephrase the user's ask or rewrite it in your own words; pass along the user's own words. If the background agent is idle, this starts a new task and returns the final result to the user. If the background agent is already working on a task, this sends the request as guidance to steer that previous task. If the user asks to do something next, later, after this, or once current work finishes, call this tool so the work is actually queued instead of merely promising to do it later.","parameters":{"type":"object","properties":{"prompt":{"type":"string","description":"The user request to delegate to the background agent."}},"required":["prompt"],"additionalProperties":false}},{"type":"function","name":"remain_silent","description":"Call this when the best response is to say nothing. Use it instead of speaking after hidden system/control messages, after background agent updates in silent modes, or whenever acknowledging aloud would be distracting. This tool has no user-visible effect.","parameters":{"type":"object","properties":{},"additionalProperties":false}}]}"#;
    let session = normalized_json_string(session)?;
    assert_eq!(
        body,
        format!(
            "--codex-realtime-call-boundary\r\n\
             Content-Disposition: form-data; name=\"sdp\"\r\n\
             Content-Type: application/sdp\r\n\
             \r\n\
             v=offer\r\n\
             \r\n\
             --codex-realtime-call-boundary\r\n\
             Content-Disposition: form-data; name=\"session\"\r\n\
             Content-Type: application/json\r\n\
             \r\n\
             {session}\r\n\
             --codex-realtime-call-boundary--\r\n"
        )
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn webrtc_v1_start_posts_offer_returns_sdp_and_joins_sideband() -> Result<()> {
    skip_if_no_network!(Ok(()));

    // Phase 1: build a v1 realtime thread with a mocked call-create response and a sideband socket
    // that immediately proves the joined connection can receive server events.
    let mut harness = RealtimeE2eHarness::new(
        RealtimeTestVersion::V1,
        no_main_loop_responses(),
        realtime_sideband(vec![open_realtime_sideband_connection(vec![vec![
            session_updated("sess_v1_webrtc"),
        ]])]),
    )
    .await?;

    // Phase 2: start through app-server and assert the app receives both the started notification
    // and the answer SDP.
    let started = harness.start_webrtc_realtime("v=offer\r\n").await?;
    assert_eq!(
        started,
        StartedWebrtcRealtime {
            started: ThreadRealtimeStartedNotification {
                thread_id: harness.thread_id.clone(),
                realtime_session_id: Some(harness.thread_id.clone()),
                version: RealtimeConversationVersion::V1,
            },
            sdp: ThreadRealtimeSdpNotification {
                thread_id: harness.thread_id.clone(),
                sdp: "v=answer\r\n".to_string(),
            },
        }
    );

    // Phase 3: verify the HTTP call-create leg, the direct sideband join, and the normal v1
    // session.update; the WebRTC transport should remain alive instead of closing after SDP.
    assert_call_create_multipart(
        harness.call_capture.single_request(),
        "v=offer\r\n",
        v1_session_create_json(),
    )?;

    let session_update = harness.sideband_outbound_request(/*request_index*/ 0).await;
    assert_v1_session_update(&session_update)?;
    assert_eq!(
        harness.realtime_server.single_handshake().uri(),
        "/v1/realtime?intent=quicksilver&call_id=rtc_e2e"
    );

    let closed = timeout(
        Duration::from_millis(100),
        harness
            .mcp
            .read_stream_until_notification_message("thread/realtime/closed"),
    )
    .await;
    assert!(closed.is_err(), "WebRTC start should not close immediately");

    harness.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn webrtc_v1_handoff_request_delegates_and_appends_result() -> Result<()> {
    skip_if_no_network!(Ok(()));

    // Phase 1: script one v1 handoff request on the sideband and one delegated Responses turn.
    let mut harness = RealtimeE2eHarness::new(
        RealtimeTestVersion::V1,
        main_loop_responses(vec![create_final_assistant_message_sse_response(
            "delegated from v1",
        )?]),
        realtime_sideband(vec![realtime_sideband_connection(vec![
            vec![
                session_updated("sess_v1_handoff"),
                json!({
                    "type": "conversation.item.input_audio_transcription.completed",
                    "transcript": "delegate from v1"
                }),
                json!({
                    "type": "response.output_audio_transcript.delta",
                    "delta": "the secret word is "
                }),
                json!({
                    "type": "response.output_audio_transcript.delta",
                    "delta": "kumquat"
                }),
                json!({
                    "type": "conversation.handoff.requested",
                    "handoff_id": "handoff_v1",
                    "item_id": "item_v1",
                    "input_transcript": "delegate from v1"
                }),
            ],
            vec![],
        ])]),
    )
    .await?;

    let started = harness.start_webrtc_realtime("v=offer\r\n").await?;
    assert_eq!(started.started.version, RealtimeConversationVersion::V1);
    assert_call_create_multipart(
        harness.call_capture.single_request(),
        "v=offer\r\n",
        v1_session_create_json(),
    )?;
    assert_v1_session_update(&harness.sideband_outbound_request(/*request_index*/ 0).await)?;

    // Phase 2: wait for the delegated background agent turn that is launched by the handoff request.
    let turn_started = harness
        .read_notification::<TurnStartedNotification>("turn/started")
        .await?;
    assert_eq!(turn_started.thread_id, harness.thread_id);
    let turn_completed = harness
        .read_notification::<TurnCompletedNotification>("turn/completed")
        .await?;
    assert_eq!(turn_completed.thread_id, harness.thread_id);

    // Phase 3: assert the delegated prompt went to Responses, then the v1 handoff append went back
    // over the existing sideband connection.
    let requests = harness.main_loop_responses_requests().await?;
    assert_eq!(requests.len(), 1);
    assert!(
        response_request_contains_text(
            &requests[0],
            "<realtime_delegation>\n  <input>delegate from v1</input>\n  <transcript_delta>user: delegate from v1\nassistant: the secret word is kumquat</transcript_delta>\n</realtime_delegation>",
        ),
        "delegated Responses request should contain realtime delegation envelope: {}",
        requests[0]
    );
    let handoff_append = harness.sideband_outbound_request(/*request_index*/ 1).await;
    assert_eq!(
        handoff_append,
        json!({
            "type": "conversation.handoff.append",
            "handoff_id": "handoff_v1",
            "output_text": "\"Agent Final Message\":\n\ndelegated from v1",
        })
    );

    harness.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn webrtc_v2_forwards_audio_and_text_between_client_and_sideband() -> Result<()> {
    skip_if_no_network!(Ok(()));

    // Phase 1: create a v2 WebRTC conversation whose sideband sends transcript + output audio
    // after the client has had a chance to append input.
    let mut harness = RealtimeE2eHarness::new(
        RealtimeTestVersion::V2,
        no_main_loop_responses(),
        realtime_sideband(vec![realtime_sideband_connection(vec![
            vec![session_updated("sess_v2_stream")],
            vec![],
            vec![
                json!({
                    "type": "conversation.item.input_audio_transcription.delta",
                    "delta": "transcribed audio"
                }),
                json!({
                    "type": "response.output_audio.delta",
                    "delta": "AQID",
                    "sample_rate": 24_000,
                    "channels": 1,
                    "samples_per_channel": 512
                }),
            ],
        ])]),
    )
    .await?;

    let started = harness.start_webrtc_realtime("v=offer\r\n").await?;
    assert_eq!(started.started.version, RealtimeConversationVersion::V2);
    assert_v2_session_update(&harness.sideband_outbound_request(/*request_index*/ 0).await)?;

    // Phase 2: drive app-server as the client would: append audio, append text, then receive
    // transcript/audio notifications that came from the sideband socket.
    let thread_id = started.started.thread_id.clone();
    harness.append_audio(thread_id.clone()).await?;
    harness.append_text(thread_id, "hello").await?;

    let transcript = harness
        .read_notification::<ThreadRealtimeTranscriptDeltaNotification>(
            "thread/realtime/transcript/delta",
        )
        .await?;
    assert_eq!(transcript.delta, "transcribed audio");
    let output_audio = harness
        .read_notification::<ThreadRealtimeOutputAudioDeltaNotification>(
            "thread/realtime/outputAudio/delta",
        )
        .await?;
    assert_eq!(output_audio.audio.data, "AQID");

    // Phase 3: prove the client inputs were translated into the v2 realtime sideband events.
    let requests = [
        harness.sideband_outbound_request(/*request_index*/ 1).await,
        harness.sideband_outbound_request(/*request_index*/ 2).await,
    ];
    assert!(
        requests
            .iter()
            .any(|request| request["type"] == "input_audio_buffer.append"
                && request["audio"] == "BQYH"),
        "sideband requests should include audio append: {requests:?}"
    );
    assert!(
        requests.iter().any(|request| {
            request["type"] == "conversation.item.create"
                && request["item"]["type"] == "message"
                && request["item"]["role"] == "user"
                && request["item"]["content"][0]["type"] == "input_text"
                && request["item"]["content"][0]["text"] == "[USER] hello"
        }),
        "sideband requests should include user text item: {requests:?}"
    );

    harness.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn webrtc_v2_text_input_is_append_only_while_response_is_active() -> Result<()> {
    skip_if_no_network!(Ok(()));

    // Phase 1: script a server-side response that becomes active after the first
    // user text turn, then finishes only after a later audio input.
    let mut harness = RealtimeE2eHarness::new(
        RealtimeTestVersion::V2,
        no_main_loop_responses(),
        realtime_sideband(vec![realtime_sideband_connection(vec![
            vec![session_updated("sess_v2_response_queue")],
            vec![
                json!({
                    "type": "response.created",
                    "response": { "id": "resp_active" }
                }),
                json!({
                    "type": "response.output_text.delta",
                    "delta": "active response started"
                }),
            ],
            vec![],
            vec![json!({
                "type": "response.done",
                "response": { "id": "resp_active" }
            })],
        ])]),
    )
    .await?;

    let started = harness.start_webrtc_realtime("v=offer\r\n").await?;
    assert_eq!(started.started.version, RealtimeConversationVersion::V2);

    // From here on, `sideband_outbound_request(n)` reads outbound messages to
    // the fake Realtime API sideband websocket. These are not client-facing
    // notifications; they are the protocol frames app-server sends upstream.
    assert_v2_session_update(&harness.sideband_outbound_request(/*request_index*/ 0).await)?;

    // Phase 2: send the first text turn. Text input is append-only, so this
    // sends only the user text item.
    let thread_id = started.started.thread_id.clone();
    harness.append_text(thread_id.clone(), "first").await?;
    assert_v2_user_text_item(
        &harness.sideband_outbound_request(/*request_index*/ 1).await,
        "first",
    );
    let transcript = harness
        .read_notification::<ThreadRealtimeTranscriptDeltaNotification>(
            "thread/realtime/transcript/delta",
        )
        .await?;
    assert_eq!(transcript.delta, "active response started");

    // Phase 3: send a second text turn while `resp_active` is still open. The
    // user message must reach realtime without requesting another response.
    harness.append_text(thread_id.clone(), "second").await?;
    assert_v2_user_text_item(
        &harness.sideband_outbound_request(/*request_index*/ 2).await,
        "second",
    );

    // Phase 4: audio still forwards normally after text input.
    harness.append_audio(thread_id).await?;

    let audio = harness.sideband_outbound_request(/*request_index*/ 3).await;
    assert_eq!(audio["type"], "input_audio_buffer.append");
    assert_eq!(audio["audio"], "BQYH");

    harness.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn webrtc_v2_text_input_is_append_only_when_response_is_cancelled() -> Result<()> {
    skip_if_no_network!(Ok(()));

    // Phase 1: script a server-side response that becomes active after the first
    // text turn, then is cancelled only after a later audio input.
    let mut harness = RealtimeE2eHarness::new(
        RealtimeTestVersion::V2,
        no_main_loop_responses(),
        realtime_sideband(vec![realtime_sideband_connection(vec![
            vec![session_updated("sess_v2_response_cancel_queue")],
            vec![json!({
                "type": "response.created",
                "response": { "id": "resp_cancelled" }
            })],
            vec![],
            vec![json!({
                "type": "response.cancelled",
                "response": { "id": "resp_cancelled" }
            })],
        ])]),
    )
    .await?;

    let started = harness.start_webrtc_realtime("v=offer\r\n").await?;
    assert_eq!(started.started.version, RealtimeConversationVersion::V2);
    assert_v2_session_update(&harness.sideband_outbound_request(/*request_index*/ 0).await)?;

    // Phase 2: send the first text turn. Text input is append-only, so this
    // sends only the user text item.
    let thread_id = started.started.thread_id.clone();
    harness.append_text(thread_id.clone(), "first").await?;
    assert_v2_user_text_item(
        &harness.sideband_outbound_request(/*request_index*/ 1).await,
        "first",
    );

    // Phase 3: send a second text turn while `resp_cancelled` is still open.
    // The user message must reach realtime without requesting another response.
    harness.append_text(thread_id.clone(), "second").await?;
    assert_v2_user_text_item(
        &harness.sideband_outbound_request(/*request_index*/ 2).await,
        "second",
    );

    // Phase 4: audio still forwards normally after text input.
    harness.append_audio(thread_id).await?;

    let audio = harness.sideband_outbound_request(/*request_index*/ 3).await;
    assert_eq!(audio["type"], "input_audio_buffer.append");
    assert_eq!(audio["audio"], "BQYH");

    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn webrtc_v2_background_agent_steering_ack_requests_response_create() -> Result<()> {
    skip_if_no_network!(Ok(()));

    // Phase 1: gate the delegated Responses turn from the first tool call so
    // the background-agent handoff stays active while realtime sends a second
    // tool call that should steer the active task.
    let main_loop_responses_server = responses::start_mock_server().await;
    let (gate_completed_tx, gate_completed_rx) = mpsc::channel();
    let gated_response = responses::sse(vec![
        responses::ev_response_created("resp-1"),
        responses::ev_assistant_message("msg-1", "first task finished"),
        responses::ev_completed("resp-1"),
    ]);
    Mock::given(method("POST"))
        .and(path_regex(".*/responses$"))
        .respond_with(GatedSseResponse {
            gate_rx: Mutex::new(Some(gate_completed_rx)),
            response: gated_response,
        })
        .expect(2)
        .mount(&main_loop_responses_server)
        .await;

    let mut harness = RealtimeE2eHarness::new_with_main_loop_responses_server(
        RealtimeTestVersion::V2,
        main_loop_responses_server,
        realtime_sideband(vec![realtime_sideband_connection(vec![
            vec![
                session_updated("sess_v2_steering_ack"),
                v2_background_agent_tool_call("call_active", "start a task"),
                v2_background_agent_tool_call("call_steer", "steer the active task"),
            ],
            vec![],
            vec![],
            vec![],
            vec![],
        ])]),
    )
    .await?;

    let started = harness.start_webrtc_realtime("v=offer\r\n").await?;
    assert_eq!(started.started.version, RealtimeConversationVersion::V2);
    assert_v2_session_update(&harness.sideband_outbound_request(/*request_index*/ 0).await)?;
    let turn_started = harness
        .read_notification::<TurnStartedNotification>("turn/started")
        .await?;
    assert_eq!(turn_started.thread_id, harness.thread_id);

    // Phase 2: the second tool call happens while `call_active` is still
    // running, so app-server sends a steering acknowledgement as a function-call
    // output for the second call.
    assert_v2_function_call_output(
        &harness.sideband_outbound_request(/*request_index*/ 1).await,
        "call_steer",
        V2_STEERING_ACKNOWLEDGEMENT,
    );

    // Phase 3: realtime needs a `response.create` after the steering
    // acknowledgement so it can surface that acknowledgement to the user.
    assert_v2_response_create(&harness.sideband_outbound_request(/*request_index*/ 2).await);

    // Phase 4: release the gated delegated turn. Codex should then continue
    // the same run with the steering text included in the follow-up Responses
    // request, proving realtime did not merely acknowledge and drop it.
    let _ = gate_completed_tx.send(());
    let turn_completed = harness
        .read_notification::<TurnCompletedNotification>("turn/completed")
        .await?;
    assert_eq!(turn_completed.thread_id, harness.thread_id);

    let requests = harness.main_loop_responses_requests().await?;
    assert_eq!(requests.len(), 2);
    assert!(
        response_request_contains_text(&requests[1], "steer the active task"),
        "follow-up Responses request should contain steering prompt: {}",
        requests[1]
    );

    harness.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn webrtc_v2_background_agent_progress_is_sent_before_function_output() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let mut harness = RealtimeE2eHarness::new(
        RealtimeTestVersion::V2,
        main_loop_responses(vec![create_final_assistant_message_sse_response(
            "progress before final",
        )?]),
        realtime_sideband(vec![realtime_sideband_connection(vec![
            vec![
                session_updated("sess_v2_progress_before_final"),
                v2_background_agent_tool_call("call_progress_order", "stream progress"),
            ],
            vec![],
            vec![],
        ])]),
    )
    .await?;

    let started = harness.start_webrtc_realtime("v=offer\r\n").await?;
    assert_eq!(started.started.version, RealtimeConversationVersion::V2);

    let turn_completed = harness
        .read_notification::<TurnCompletedNotification>("turn/completed")
        .await?;
    assert_eq!(turn_completed.thread_id, harness.thread_id);

    let progress = harness.sideband_outbound_request(/*request_index*/ 1).await;
    assert_v2_progress_update(&progress, "progress before final");

    let tool_output = harness.sideband_outbound_request(/*request_index*/ 2).await;
    assert_v2_function_call_output(
        &tool_output,
        "call_progress_order",
        V2_HANDOFF_COMPLETE_ACKNOWLEDGEMENT,
    );

    harness.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn realtime_webrtc_start_surfaces_backend_error() -> Result<()> {
    skip_if_no_network!(Ok(()));

    // Phase 1: make call creation fail before any sideband connection can matter.
    let responses_server = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    Mock::given(method("POST"))
        .and(path("/v1/realtime/calls"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&responses_server)
        .await;
    let realtime_server = start_websocket_server(vec![vec![]]).await;

    let codex_home = TempDir::new()?;
    create_config_toml(
        codex_home.path(),
        &responses_server.uri(),
        realtime_server.uri(),
        /*realtime_enabled*/ true,
        StartupContextConfig::Override("startup context"),
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;
    login_with_api_key(&mut mcp, "sk-test-key").await?;

    // Phase 2: start a normal app-server thread and request realtime over WebRTC.
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
            thread_id: thread_start.thread.id,
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: Some(ThreadRealtimeStartTransport::Webrtc {
                sdp: "v=offer\r\n".to_string(),
            }),
            voice: None,
        })
        .await?;
    let start_response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_request_id)),
    )
    .await??;
    let _: ThreadRealtimeStartResponse = to_response(start_response)?;

    // Phase 3: the JSON-RPC start request returns, and the realtime failure is delivered as the
    // typed realtime error notification.
    let error =
        read_notification::<ThreadRealtimeErrorNotification>(&mut mcp, "thread/realtime/error")
            .await?;
    assert!(error.message.contains("currently experiencing high demand"));

    realtime_server.shutdown().await;
    Ok(())
}
