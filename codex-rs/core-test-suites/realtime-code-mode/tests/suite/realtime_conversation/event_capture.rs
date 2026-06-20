use crate::realtime_conversation_support::*;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_start_audio_text_close_round_trip() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![
        vec![],
        vec![
            vec![json!({
                "type": "session.updated",
                "session": { "id": "sess_1", "instructions": "backend prompt" }
            })],
            vec![],
            vec![
                json!({
                    "type": "conversation.output_audio.delta",
                    "delta": "AQID",
                    "sample_rate": 24000,
                    "channels": 1
                }),
                json!({
                    "type": "conversation.item.added",
                    "item": {
                        "type": "message",
                        "role": "assistant",
                        "content": [{"type": "text", "text": "hi"}]
                    }
                }),
            ],
        ],
    ])
    .await;

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
    assert_eq!(started.version, RealtimeConversationVersion::V1);

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

    test.codex
        .submit(Op::RealtimeConversationAudio(ConversationAudioParams {
            frame: RealtimeAudioFrame {
                data: "AQID".to_string(),
                sample_rate: 24000,
                num_channels: 1,
                samples_per_channel: Some(480),
                item_id: None,
            },
        }))
        .await?;
    test.codex
        .submit(Op::RealtimeConversationText(ConversationTextParams {
            text: "hello".to_string(),
        }))
        .await?;

    let audio_out = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::AudioOut(frame),
        }) => Some(frame.clone()),
        _ => None,
    })
    .await;
    assert_eq!(audio_out.data, "AQID");

    let connections = server.connections();
    assert_eq!(connections.len(), 2);
    let connection = &connections[1];
    assert_eq!(connection.len(), 3);
    assert_eq!(
        connection[0].body_json()["type"].as_str(),
        Some("session.update")
    );
    assert_eq!(
        connection[0].body_json()["session"]["audio"]["output"]["voice"],
        "cove"
    );
    let initial_instructions = websocket_request_instructions(&connection[0])
        .expect("initial session update instructions");
    assert!(initial_instructions.starts_with("backend prompt"));
    assert_eq!(
        server.handshakes()[1]
            .header("x-session-id")
            .expect("session.update x-session-id header"),
        started
            .realtime_session_id
            .as_deref()
            .expect("started session id should be present")
    );
    assert_eq!(
        server.handshakes()[1].header("authorization").as_deref(),
        Some("Bearer dummy")
    );
    assert_eq!(
        server.handshakes()[1].uri(),
        "/v1/realtime?intent=quicksilver&model=realtime-test-model"
    );
    let mut request_types = [
        connection[1].body_json()["type"]
            .as_str()
            .expect("request type")
            .to_string(),
        connection[2].body_json()["type"]
            .as_str()
            .expect("request type")
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

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_user_text_turn_is_sent_to_realtime_when_active() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let api_server = start_mock_server().await;
    let response_mock = responses::mount_sse_once(
        &api_server,
        responses::sse(vec![
            responses::ev_response_created("resp_user_text"),
            responses::ev_assistant_message("msg_user_text", "ack"),
            responses::ev_completed("resp_user_text"),
        ]),
    )
    .await;

    let realtime_server = start_websocket_server(vec![vec![
        vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_user_text", "instructions": "backend prompt" }
        })],
        vec![],
    ]])
    .await;

    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.experimental_realtime_ws_startup_context = Some(String::new());
        }
    });
    let test = builder.build(&api_server).await?;

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
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
    assert_eq!(session_updated, "sess_user_text");

    let user_text = "typed follow-up for realtime";
    let prefixed_user_text = format!("[USER] {user_text}");
    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: user_text.to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let realtime_text_request = wait_for_matching_websocket_request(
        &realtime_server,
        "normal user turn text mirrored to realtime",
        |request| websocket_request_text(request).as_deref() == Some(prefixed_user_text.as_str()),
    )
    .await;
    let model_user_texts = response_mock.single_request().message_input_texts("user");
    assert_eq!(
        (
            model_user_texts.iter().any(|text| text == user_text),
            websocket_request_text(&realtime_text_request),
        ),
        (true, Some(prefixed_user_text)),
    );
    let realtime_response_create = timeout(Duration::from_millis(200), async {
        wait_for_matching_websocket_request(
            &realtime_server,
            "unexpected realtime response request for mirrored user text",
            |request| request.body_json()["type"].as_str() == Some("response.create"),
        )
        .await
    })
    .await;
    assert!(
        realtime_response_create.is_err(),
        "mirrored user text should not request a realtime response"
    );

    let realtime_request_body = realtime_text_request.body_json();
    let content = &realtime_request_body["item"]["content"][0];
    let snapshot = format!(
        "type: {}\nitem.type: {}\nitem.role: {}\ncontent[0].type: {}\ncontent[0].text: {}\nresponse.create: {}",
        realtime_request_body["type"].as_str().unwrap_or_default(),
        realtime_request_body["item"]["type"]
            .as_str()
            .unwrap_or_default(),
        realtime_request_body["item"]["role"]
            .as_str()
            .unwrap_or_default(),
        content["type"].as_str().unwrap_or_default(),
        content["text"].as_str().unwrap_or_default(),
        realtime_response_create.is_ok(),
    );
    insta::assert_snapshot!(
        "conversation_user_text_turn_is_sent_to_realtime_when_active",
        snapshot
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_user_text_turn_is_capped_when_mirrored_to_realtime() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let api_server = start_mock_server().await;
    let response_mock = responses::mount_sse_once(
        &api_server,
        responses::sse(vec![
            responses::ev_response_created("resp_long_user_text"),
            responses::ev_assistant_message("msg_long_user_text", "ack"),
            responses::ev_completed("resp_long_user_text"),
        ]),
    )
    .await;

    let realtime_server = start_websocket_server(vec![vec![
        vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_long_user_text", "instructions": "backend prompt" }
        })],
        vec![],
    ]])
    .await;

    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.experimental_realtime_ws_startup_context = Some(String::new());
        }
    });
    let test = builder.build(&api_server).await?;

    // Phase 1: start realtime so the next normal user turn mirrors over the
    // active WebSocket session.
    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
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
    assert_eq!(session_updated, "sess_long_user_text");

    // Phase 2: submit one oversized text turn. The model request should keep
    // the exact user text, while the realtime mirror should get the capped copy.
    let user_text = format!(
        "mirror-head {} mirror-middle {} mirror-tail",
        "alpha ".repeat(900),
        "omega ".repeat(900),
    );
    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: user_text.clone(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    // Phase 3: capture the mirrored WebSocket item; the snapshot below records
    // the capped payload shape.
    let realtime_text_request = wait_for_matching_websocket_request(
        &realtime_server,
        "capped normal user turn text mirrored to realtime",
        |request| websocket_request_text(request).is_some_and(|text| text.contains("mirror-head")),
    )
    .await;
    let realtime_text =
        websocket_request_text(&realtime_text_request).expect("realtime request text");
    let model_user_texts = response_mock.single_request().message_input_texts("user");

    let realtime_request_body = realtime_text_request.body_json();
    let content = &realtime_request_body["item"]["content"][0];

    // Snapshot the request envelope and capped text together so reviewers can
    // see the preserved head/tail and truncation marker in one place.
    let snapshot = format!(
        "type: {}\nitem.type: {}\nitem.role: {}\ncontent[0].type: {}\nmodel_has_full_user_text: {}\nrealtime_text_equal_full_user_text: {}\nrealtime_text_approx_tokens: {}\ncontent[0].text: {}",
        realtime_request_body["type"].as_str().unwrap_or_default(),
        realtime_request_body["item"]["type"]
            .as_str()
            .unwrap_or_default(),
        realtime_request_body["item"]["role"]
            .as_str()
            .unwrap_or_default(),
        content["type"].as_str().unwrap_or_default(),
        model_user_texts.iter().any(|text| text == &user_text),
        realtime_text == user_text,
        approx_token_count(&realtime_text),
        realtime_text,
    );
    insta::assert_snapshot!(
        "conversation_user_text_turn_is_capped_when_mirrored_to_realtime",
        snapshot
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbound_conversation_item_does_not_start_turn_and_still_forwards_audio() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let api_server = start_mock_server().await;

    let realtime_server = start_websocket_server(vec![vec![vec![
        json!({
            "type": "session.updated",
            "session": { "id": "sess_ignore_item", "instructions": "backend prompt" }
        }),
        json!({
            "type": "conversation.item.added",
            "item": {
                "type": "message",
                "role": "user",
                "content": [{"type": "text", "text": "echoed local text"}]
            }
        }),
        json!({
            "type": "conversation.output_audio.delta",
            "delta": "AQID",
            "sample_rate": 24000,
            "channels": 1
        }),
    ]]])
    .await;

    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
        }
    });
    let test = builder.build(&api_server).await?;

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
        }))
        .await?;

    let _ = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload:
                RealtimeEvent::SessionUpdated {
                    realtime_session_id: session_id,
                    ..
                },
        }) if session_id == "sess_ignore_item" => Some(()),
        _ => None,
    })
    .await;

    let audio_out = tokio::time::timeout(
        Duration::from_millis(500),
        wait_for_event_match(&test.codex, |msg| match msg {
            EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
                payload: RealtimeEvent::AudioOut(frame),
            }) => Some(frame.clone()),
            _ => None,
        }),
    )
    .await
    .expect("timed out waiting for realtime audio after conversation item");
    assert_eq!(audio_out.data, "AQID");

    let unexpected_turn_started = tokio::time::timeout(
        Duration::from_millis(200),
        wait_for_event_match(&test.codex, |msg| match msg {
            EventMsg::TurnStarted(_) => Some(()),
            _ => None,
        }),
    )
    .await;
    assert!(unexpected_turn_started.is_err());

    realtime_server.shutdown().await;
    Ok(())
}
