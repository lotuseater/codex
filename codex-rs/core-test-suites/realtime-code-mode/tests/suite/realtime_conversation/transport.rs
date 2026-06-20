use crate::realtime_conversation_support::*;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_webrtc_start_posts_generated_session() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let sideband_accept_delay = Duration::from_millis(1000);
    let capture = RealtimeCallRequestCapture::new();
    Mock::given(method("POST"))
        .and(path_regex(".*/realtime/calls$"))
        .and(capture.clone())
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Location", "/v1/realtime/calls/calls/rtc_core_test")
                .set_body_string("v=answer\r\n"),
        )
        .mount(&server)
        .await;
    let realtime_server = start_websocket_server_with_headers(vec![WebSocketConnectionConfig {
        requests: vec![
            vec![json!({
                "type": "session.updated",
                "session": { "id": "sess_webrtc", "instructions": "backend prompt" }
            })],
            vec![],
        ],
        response_headers: Vec::new(),
        accept_delay: Some(sideband_accept_delay),
        close_after_requests: false,
    }])
    .await;

    let realtime_ws_base_url = realtime_server.uri().to_string();
    let mut builder = test_codex().with_config(move |config| {
        config.experimental_realtime_ws_backend_prompt = Some("backend prompt".to_string());
        config.experimental_realtime_ws_model = Some("realtime-test-model".to_string());
        config.experimental_realtime_ws_startup_context = Some("startup context".to_string());
        config.experimental_realtime_ws_base_url = Some(realtime_ws_base_url);
        config.realtime.version = RealtimeWsVersion::V1;
    });
    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: Some(ConversationStartTransport::Webrtc {
                sdp: "v=offer\r\n".to_string(),
            }),
            voice: None,
        }))
        .await?;

    // Phase 1: the client gets the SDP answer that configures its peer connection, and then the
    // normal realtime event stream from the joined sideband WebSocket.
    let created = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationSdp(created) => Some(Ok(created.clone())),
        EventMsg::Error(err) => Some(Err(err.clone())),
        _ => None,
    })
    .await
    .unwrap_or_else(|err: ErrorEvent| panic!("conversation call create failed: {err:?}"));
    assert_eq!(created.sdp, "v=answer\r\n");
    assert!(
        realtime_server.handshakes().is_empty(),
        "SDP should be emitted before the delayed sideband websocket joins"
    );

    test.codex
        .submit(Op::RealtimeConversationText(ConversationTextParams {
            text: "queued before sideband".to_string(),
        }))
        .await?;

    let session_updated = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload:
                RealtimeEvent::SessionUpdated {
                    realtime_session_id: session_id,
                    ..
                },
        }) => Some(session_id.clone()),
        _ => None,
    })
    .await;
    assert_eq!(session_updated, "sess_webrtc");

    // Phase 2: call creation posts the offer and generated session together, so the media leg can
    // begin inference before the sideband WebSocket is ready.
    let request = capture.single_request();
    assert_eq!(request.url.path(), "/v1/realtime/calls");
    assert_eq!(request.url.query(), None);
    assert_eq!(
        request
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer dummy")
    );
    assert_eq!(
        request
            .headers
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("multipart/form-data; boundary=codex-realtime-call-boundary")
    );
    let body = String::from_utf8(request.body).context("multipart body should be utf-8")?;
    let session = r#"{"audio":{"input":{"format":{"type":"audio/pcm","rate":24000}},"output":{"voice":"cove"}},"type":"quicksilver","model":"realtime-test-model","instructions":"backend prompt\n\nstartup context"}"#;
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

    // Phase 3: the server joins that same call over the direct sideband WebSocket, sends the
    // ordinary session.update, and keeps the conversation alive until the client closes it.
    let session_update = wait_for_websocket_request(
        &realtime_server,
        /*connection_index*/ 0,
        /*request_index*/ 0,
    )
    .await?;
    assert_eq!(
        session_update.body_json()["type"].as_str(),
        Some("session.update")
    );
    assert!(
        websocket_request_instructions(&session_update)
            .context("session.update should include instructions")?
            .contains("startup context")
    );
    let queued_text = wait_for_websocket_request(
        &realtime_server,
        /*connection_index*/ 0,
        /*request_index*/ 1,
    )
    .await?;
    assert_eq!(
        websocket_request_text(&queued_text).as_deref(),
        Some("queued before sideband")
    );
    let handshake = realtime_server.single_handshake();
    assert_eq!(
        handshake.uri(),
        "/v1/realtime?intent=quicksilver&call_id=rtc_core_test"
    );
    assert_eq!(
        handshake.header("authorization").as_deref(),
        Some("Bearer dummy")
    );

    test.codex.submit(Op::RealtimeConversationClose).await?;
    let closed = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationClosed(closed) => Some(closed.clone()),
        _ => None,
    })
    .await;
    assert!(matches!(
        closed.reason.as_deref(),
        Some("requested" | "transport_closed")
    ));

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_webrtc_close_while_sideband_connecting_drops_pending_join() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    Mock::given(method("POST"))
        .and(path_regex(".*/realtime/calls$"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Location", "/v1/realtime/calls/calls/rtc_close_pending")
                .set_body_string("v=answer\r\n"),
        )
        .mount(&server)
        .await;
    let realtime_server = start_websocket_server_with_headers(vec![WebSocketConnectionConfig {
        requests: vec![vec![]],
        response_headers: Vec::new(),
        accept_delay: Some(Duration::from_millis(500)),
        close_after_requests: false,
    }])
    .await;

    let realtime_ws_base_url = realtime_server.uri().to_string();
    let mut builder = test_codex().with_config(move |config| {
        config.experimental_realtime_ws_backend_prompt = Some("backend prompt".to_string());
        config.experimental_realtime_ws_model = Some("realtime-test-model".to_string());
        config.experimental_realtime_ws_startup_context = Some(String::new());
        config.experimental_realtime_ws_base_url = Some(realtime_ws_base_url);
        config.realtime.version = RealtimeWsVersion::V1;
    });
    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: Some(ConversationStartTransport::Webrtc {
                sdp: "v=offer\r\n".to_string(),
            }),
            voice: None,
        }))
        .await?;

    let sdp = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationSdp(created) => Some(created.sdp.clone()),
        _ => None,
    })
    .await;
    assert_eq!(sdp, "v=answer\r\n");
    assert!(
        realtime_server.handshakes().is_empty(),
        "sideband websocket should still be pending when SDP is emitted"
    );

    test.codex.submit(Op::RealtimeConversationClose).await?;
    let closed = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationClosed(closed) => Some(closed.clone()),
        _ => None,
    })
    .await;
    assert_eq!(closed.reason.as_deref(), Some("requested"));

    let stale_event = timeout(Duration::from_millis(700), async {
        wait_for_event_match(&test.codex, |msg| match msg {
            EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
                payload: RealtimeEvent::Error(message),
            }) => Some(format!("stale realtime error: {message}")),
            EventMsg::RealtimeConversationClosed(closed) => {
                Some(format!("stale close event: {:?}", closed.reason))
            }
            _ => None,
        })
        .await
    })
    .await;
    assert!(
        stale_event.is_err(),
        "pending sideband task leaked after close: {:?}",
        stale_event.ok()
    );
    assert!(
        realtime_server.handshakes().is_empty(),
        "pending sideband task should abort before websocket handshake completes"
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_webrtc_sideband_connect_failure_closes_with_error() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    Mock::given(method("POST"))
        .and(path_regex(".*/realtime/calls$"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Location", "/v1/realtime/calls/calls/rtc_sideband_failure")
                .set_body_string("v=answer\r\n"),
        )
        .mount(&server)
        .await;
    let mut builder = test_codex().with_config(|config| {
        config.experimental_realtime_ws_backend_prompt = Some("backend prompt".to_string());
        config.experimental_realtime_ws_model = Some("realtime-test-model".to_string());
        config.experimental_realtime_ws_startup_context = Some(String::new());
        config.experimental_realtime_ws_base_url = Some("http://127.0.0.1:1".to_string());
        // Keep the failure-path test inside wait_for_event's timeout on Windows,
        // where refused localhost websocket connects can take around two seconds.
        config.model_provider.request_max_retries = Some(0);
        config.realtime.version = RealtimeWsVersion::V1;
    });
    let test = builder.build(&server).await?;

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: Some(ConversationStartTransport::Webrtc {
                sdp: "v=offer\r\n".to_string(),
            }),
            voice: None,
        }))
        .await?;

    let started = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationStarted(started) => Some(started.clone()),
        _ => None,
    })
    .await;
    assert!(started.realtime_session_id.is_some());

    let sdp = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationSdp(created) => Some(created.sdp.clone()),
        _ => None,
    })
    .await;
    assert_eq!(sdp, "v=answer\r\n");

    let err = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::Error(message),
        }) => Some(message.clone()),
        _ => None,
    })
    .await;
    assert!(!err.is_empty());

    let closed = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationClosed(closed) => Some(closed.clone()),
        _ => None,
    })
    .await;
    assert_eq!(closed.reason.as_deref(), Some("error"));

    test.codex
        .submit(Op::RealtimeConversationText(ConversationTextParams {
            text: "after sideband failure".to_string(),
        }))
        .await?;
    let err = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::Error(err) => Some(err.clone()),
        _ => None,
    })
    .await;
    assert_eq!(err.message, "conversation is not running");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_transport_close_emits_closed_event() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let session_updated = vec![json!({
        "type": "session.updated",
        "session": { "id": "sess_1", "instructions": "backend prompt" }
    })];
    let server = start_websocket_server(vec![vec![], vec![session_updated]]).await;

    let mut builder = test_codex();
    let test = builder.build_with_websocket_server(&server).await?;
    assert!(
        server
            .wait_for_handshakes(/*expected*/ 1, Duration::from_secs(2))
            .await
    );

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
        }))
        .await?;

    let started = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationStarted(started) => Some(Ok(started.clone())),
        EventMsg::Error(err) => Some(Err(err.clone())),
        _ => None,
    })
    .await
    .unwrap_or_else(|err: ErrorEvent| panic!("conversation start failed: {err:?}"));
    assert!(started.realtime_session_id.is_some());

    let session_updated = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload:
                RealtimeEvent::SessionUpdated {
                    realtime_session_id: session_id,
                    ..
                },
        }) => Some(session_id.clone()),
        _ => None,
    })
    .await;
    assert_eq!(session_updated, "sess_1");

    let closed = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationClosed(closed) => Some(closed.clone()),
        _ => None,
    })
    .await;
    assert_eq!(closed.reason.as_deref(), Some("transport_closed"));

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_start_connect_failure_emits_realtime_error_only() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![]).await;
    let mut builder = test_codex().with_config(|config| {
        config.experimental_realtime_ws_base_url = Some("http://127.0.0.1:1".to_string());
        config.realtime.version = RealtimeWsVersion::V1;
    });
    let test = builder.build_with_websocket_server(&server).await?;

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
        }))
        .await?;

    let err = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::Error(message),
        }) => Some(message.clone()),
        _ => None,
    })
    .await;
    assert!(!err.is_empty());

    let closed = timeout(Duration::from_millis(200), async {
        wait_for_event_match(&test.codex, |msg| match msg {
            EventMsg::RealtimeConversationClosed(closed) => Some(closed.clone()),
            _ => None,
        })
        .await
    })
    .await;
    assert!(closed.is_err(), "connect failure should not emit closed");

    server.shutdown().await;
    Ok(())
}
