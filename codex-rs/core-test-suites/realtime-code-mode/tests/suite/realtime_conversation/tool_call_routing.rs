use crate::realtime_conversation_support::*;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn realtime_v2_noop_tool_call_returns_empty_function_output_without_response() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let api_server = start_mock_server().await;
    let realtime_server = start_websocket_server(vec![vec![
        vec![
            json!({
                "type": "session.updated",
                "session": { "id": "sess_silent", "instructions": "backend prompt" }
            }),
            json!({
                "type": "conversation.item.done",
                "item": {
                    "id": "item_silent",
                    "type": "function_call",
                    "name": "remain_silent",
                    "call_id": "call_silent",
                    "arguments": "{}"
                }
            }),
        ],
        vec![],
    ]])
    .await;

    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V2;
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
            payload: RealtimeEvent::NoopRequested(RealtimeNoopRequested { call_id, .. }),
        }) if call_id == "call_silent" => Some(()),
        _ => None,
    })
    .await;

    let function_output = realtime_server
        .wait_for_request(/*connection_index*/ 0, /*request_index*/ 1)
        .await;
    assert_eq!(
        function_output.body_json(),
        json!({
            "type": "conversation.item.create",
            "item": {
                "type": "function_call_output",
                "call_id": "call_silent",
                "output": ""
            }
        })
    );

    let realtime_response_create = timeout(Duration::from_millis(200), async {
        wait_for_matching_websocket_request(
            &realtime_server,
            "unexpected realtime response request for noop tool call",
            |request| request.body_json()["type"].as_str() == Some("response.create"),
        )
        .await
    })
    .await;
    assert!(
        realtime_response_create.is_err(),
        "noop tool calls should not request a realtime response"
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_mirrors_assistant_message_text_to_realtime_handoff() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let api_server = start_mock_server().await;
    let _response_mock = responses::mount_sse_once(
        &api_server,
        responses::sse(vec![
            responses::ev_response_created("resp_1"),
            responses::ev_assistant_message("msg_1", "assistant says hi"),
            responses::ev_completed("resp_1"),
        ]),
    )
    .await;

    let realtime_server = start_websocket_server(vec![vec![
        vec![
            json!({
                "type": "session.updated",
                "session": { "id": "sess_1", "instructions": "backend prompt" }
            }),
            json!({
                "type": "conversation.input_transcript.delta",
                "delta": "delegate hello"
            }),
            json!({
                "type": "conversation.handoff.requested",
                "handoff_id": "handoff_1",
                "item_id": "item_1",
                "input_transcript": "delegate hello"
            }),
        ],
        vec![],
    ]])
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

    let _ = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::HandoffRequested(handoff),
        }) if handoff.handoff_id == "handoff_1" => Some(()),
        _ => None,
    })
    .await;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        let connections = realtime_server.connections();
        if connections.len() == 1 && connections[0].len() >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let realtime_connections = realtime_server.connections();
    assert_eq!(realtime_connections.len(), 1);
    assert_eq!(realtime_connections[0].len(), 2);
    assert_eq!(
        realtime_connections[0][0].body_json()["type"].as_str(),
        Some("session.update")
    );
    assert_eq!(
        realtime_connections[0][1].body_json()["type"].as_str(),
        Some("conversation.handoff.append")
    );
    assert_eq!(
        realtime_connections[0][1].body_json()["handoff_id"].as_str(),
        Some("handoff_1")
    );
    assert_eq!(
        realtime_connections[0][1].body_json()["output_text"].as_str(),
        Some("\"Agent Final Message\":\n\nassistant says hi")
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_handoff_persists_across_item_done_until_turn_complete() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let (gate_second_message_tx, gate_second_message_rx) = oneshot::channel();
    let first_chunks = vec![
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_response_created("resp-1")),
        },
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_assistant_message(
                "msg-1",
                "assistant message 1",
            )),
        },
        StreamingSseChunk {
            gate: Some(gate_second_message_rx),
            body: sse_event(responses::ev_assistant_message(
                "msg-2",
                "assistant message 2",
            )),
        },
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_completed("resp-1")),
        },
    ];
    let (api_server, completions) = start_streaming_sse_server(vec![first_chunks]).await;

    let realtime_server = start_websocket_server(vec![vec![
        vec![
            json!({
                "type": "session.updated",
                "session": { "id": "sess_item_done", "instructions": "backend prompt" }
            }),
            json!({
                "type": "conversation.input_transcript.delta",
                "delta": "delegate now"
            }),
            json!({
                "type": "conversation.handoff.requested",
                "handoff_id": "handoff_item_done",
                "item_id": "item_item_done",
                "input_transcript": "delegate now"
            }),
        ],
        vec![json!({
            "type": "conversation.item.done",
            "item": { "id": "item_item_done" }
        })],
        vec![],
    ]])
    .await;

    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
        }
    });
    let test = builder.build_with_streaming_server(&api_server).await?;

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
        }) if session_id == "sess_item_done" => Some(()),
        _ => None,
    })
    .await;

    let _ = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::HandoffRequested(handoff),
        }) if handoff.handoff_id == "handoff_item_done" => Some(()),
        _ => None,
    })
    .await;

    let first_append = realtime_server
        .wait_for_request(/*connection_index*/ 0, /*request_index*/ 1)
        .await;
    assert_eq!(
        first_append.body_json()["type"].as_str(),
        Some("conversation.handoff.append")
    );
    assert_eq!(
        first_append.body_json()["handoff_id"].as_str(),
        Some("handoff_item_done")
    );
    assert_eq!(
        first_append.body_json()["output_text"].as_str(),
        Some("\"Agent Final Message\":\n\nassistant message 1")
    );

    let _ = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::ConversationItemDone { item_id },
        }) if item_id == "item_item_done" => Some(()),
        _ => None,
    })
    .await;

    let _ = gate_second_message_tx.send(());

    let second_append = realtime_server
        .wait_for_request(/*connection_index*/ 0, /*request_index*/ 2)
        .await;
    assert_eq!(
        second_append.body_json()["type"].as_str(),
        Some("conversation.handoff.append")
    );
    assert_eq!(
        second_append.body_json()["handoff_id"].as_str(),
        Some("handoff_item_done")
    );
    assert_eq!(
        second_append.body_json()["output_text"].as_str(),
        Some("\"Agent Final Message\":\n\nassistant message 2")
    );

    let completion = completions
        .into_iter()
        .next()
        .expect("missing delegated turn completion");
    completion
        .await
        .expect("delegated turn request did not complete");
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    realtime_server.shutdown().await;
    api_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbound_handoff_request_starts_turn() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let api_server = start_mock_server().await;
    let response_mock = responses::mount_sse_once(
        &api_server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_assistant_message("msg-1", "ok"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;

    let realtime_server = start_websocket_server(vec![vec![vec![
        json!({
            "type": "session.updated",
            "session": { "id": "sess_inbound", "instructions": "backend prompt" }
        }),
        json!({
            "type": "conversation.input_transcript.delta",
            "delta": "text from realtime"
        }),
        json!({
            "type": "conversation.handoff.requested",
            "handoff_id": "handoff_inbound",
            "item_id": "item_inbound",
            "input_transcript": "text from realtime"
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
    assert_eq!(session_updated, "sess_inbound");

    let _ = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::HandoffRequested(handoff),
        }) if handoff.handoff_id == "handoff_inbound"
            && handoff.input_transcript == "text from realtime" =>
        {
            Some(())
        }
        _ => None,
    })
    .await;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let request = response_mock.single_request();
    let user_texts = request.message_input_texts("user");
    assert!(user_texts.iter().any(|text| text
        == "<realtime_delegation>\n  <input>text from realtime</input>\n  <transcript_delta>user: text from realtime</transcript_delta>\n</realtime_delegation>"));

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbound_handoff_request_uses_active_transcript() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let api_server = start_mock_server().await;
    let response_mock = responses::mount_sse_once(
        &api_server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_assistant_message("msg-1", "ok"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;

    let realtime_server = start_websocket_server(vec![vec![vec![
        json!({
            "type": "session.updated",
            "session": { "id": "sess_inbound_multi", "instructions": "backend prompt" }
        }),
        json!({
            "type": "conversation.output_transcript.delta",
            "delta": "assistant context"
        }),
        json!({
            "type": "conversation.input_transcript.delta",
            "delta": "delegated query"
        }),
        json!({
            "type": "conversation.output_transcript.delta",
            "delta": "assist confirm"
        }),
        json!({
            "type": "conversation.handoff.requested",
            "handoff_id": "handoff_inbound_multi",
            "item_id": "item_inbound_multi",
            "input_transcript": "ignored"
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
        }) => Some(session_id.clone()),
        _ => None,
    })
    .await;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let request = response_mock.single_request();
    let user_texts = request.message_input_texts("user");
    assert!(user_texts.iter().any(|text| text
        == "<realtime_delegation>\n  <input>ignored</input>\n  <transcript_delta>assistant: assistant context\nuser: delegated query\nassistant: assist confirm\nuser: ignored</transcript_delta>\n</realtime_delegation>"));

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbound_handoff_request_sends_transcript_delta_after_each_handoff() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let api_server = start_mock_server().await;
    let response_mock = responses::mount_sse_sequence(
        &api_server,
        vec![
            responses::sse(vec![
                responses::ev_response_created("resp-1"),
                responses::ev_assistant_message("msg-1", "first ok"),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                responses::ev_response_created("resp-2"),
                responses::ev_assistant_message("msg-2", "second ok"),
                responses::ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let realtime_server = start_websocket_server(vec![vec![
        vec![
            json!({
                "type": "session.updated",
                "session": { "id": "sess_inbound_clear", "instructions": "backend prompt" }
            }),
            json!({
                "type": "conversation.input_transcript.delta",
                "delta": "first question"
            }),
            json!({
                "type": "conversation.handoff.requested",
                "handoff_id": "handoff_inbound_clear_1",
                "item_id": "item_inbound_clear_1",
                "input_transcript": "first question"
            }),
        ],
        vec![],
        vec![
            json!({
                "type": "conversation.input_transcript.delta",
                "delta": "second question"
            }),
            json!({
                "type": "conversation.handoff.requested",
                "handoff_id": "handoff_inbound_clear_2",
                "item_id": "item_inbound_clear_2",
                "input_transcript": "second question"
            }),
        ],
    ]])
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
        }) => Some(session_id.clone()),
        _ => None,
    })
    .await;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

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

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = response_mock.requests();
    assert_eq!(requests.len(), 2);

    let first_user_texts = requests[0].message_input_texts("user");
    assert!(first_user_texts.iter().any(|text| text
        == "<realtime_delegation>\n  <input>first question</input>\n  <transcript_delta>user: first question</transcript_delta>\n</realtime_delegation>"));

    let second_user_texts = requests[1].message_input_texts("user");
    assert!(second_user_texts.iter().any(|text| text
        == "<realtime_delegation>\n  <input>second question</input>\n  <transcript_delta>user: second question</transcript_delta>\n</realtime_delegation>"));
    assert!(!second_user_texts.iter().any(|text| text
        == "<realtime_delegation>\n  <input>second question</input>\n  <transcript_delta>user: first question\nuser: second question</transcript_delta>\n</realtime_delegation>"));

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delegated_turn_user_role_echo_does_not_redelegate_and_still_forwards_audio() -> Result<()>
{
    skip_if_no_network!(Ok(()));
    let start = std::time::Instant::now();

    let (gate_completed_tx, gate_completed_rx) = oneshot::channel();
    let first_chunks = vec![
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_response_created("resp-1")),
        },
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_assistant_message(
                "msg-1",
                "assistant says hi",
            )),
        },
        StreamingSseChunk {
            gate: Some(gate_completed_rx),
            body: sse_event(responses::ev_completed("resp-1")),
        },
    ];
    let (api_server, completions) = start_streaming_sse_server(vec![first_chunks]).await;

    let realtime_server = start_websocket_server(vec![vec![
        vec![
            json!({
                "type": "session.updated",
                "session": { "id": "sess_echo_guard", "instructions": "backend prompt" }
            }),
            json!({
                "type": "conversation.input_transcript.delta",
                "delta": "delegate now"
            }),
            json!({
                "type": "conversation.handoff.requested",
                "handoff_id": "handoff_echo_guard",
                "item_id": "item_echo_guard",
                "input_transcript": "delegate now"
            }),
        ],
        vec![
            json!({
                "type": "conversation.item.added",
                "item": {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "text", "text": "assistant says hi"}]
                }
            }),
            json!({
                "type": "conversation.output_audio.delta",
                "delta": "AQID",
                "sample_rate": 24000,
                "channels": 1
            }),
        ],
    ]])
    .await;

    let mut builder = test_codex().with_model("gpt-5.4").with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
        }
    });
    let test = builder.build_with_streaming_server(&api_server).await?;

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
        }) if session_id == "sess_echo_guard" => Some(()),
        _ => None,
    })
    .await;

    let _ = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::HandoffRequested(handoff),
        }) if handoff.input_transcript == "delegate now" => Some(()),
        _ => None,
    })
    .await;
    eprintln!(
        "[realtime test +{}ms] saw trigger text={:?}",
        start.elapsed().as_millis(),
        "delegate now"
    );

    let mirrored_request = realtime_server
        .wait_for_request(/*connection_index*/ 0, /*request_index*/ 1)
        .await;
    let mirrored_request_body = mirrored_request.body_json();
    eprintln!(
        "[realtime test +{}ms] saw mirrored request type={:?} handoff_id={:?} text={:?}",
        start.elapsed().as_millis(),
        mirrored_request_body["type"].as_str(),
        mirrored_request_body["handoff_id"].as_str(),
        mirrored_request_body["output_text"].as_str(),
    );
    assert_eq!(
        mirrored_request_body["type"].as_str(),
        Some("conversation.handoff.append")
    );
    assert_eq!(
        mirrored_request_body["handoff_id"].as_str(),
        Some("handoff_echo_guard")
    );
    assert_eq!(
        mirrored_request_body["output_text"].as_str(),
        Some("\"Agent Final Message\":\n\nassistant says hi")
    );

    let audio_out = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::AudioOut(frame),
        }) => Some(frame.clone()),
        _ => None,
    })
    .await;
    eprintln!(
        "[realtime test +{}ms] saw audio out data={} sample_rate={} num_channels={}",
        start.elapsed().as_millis(),
        audio_out.data,
        audio_out.sample_rate,
        audio_out.num_channels
    );
    assert_eq!(audio_out.data, "AQID");

    let completion = completions
        .into_iter()
        .next()
        .expect("missing delegated turn completion");
    let _ = gate_completed_tx.send(());
    completion
        .await
        .expect("delegated turn request did not complete");
    eprintln!(
        "[realtime test +{}ms] delegated completion resolved",
        start.elapsed().as_millis()
    );
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = api_server.requests().await;
    assert_eq!(requests.len(), 1);

    realtime_server.shutdown().await;
    api_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbound_handoff_request_does_not_block_realtime_event_forwarding() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let (gate_completed_tx, gate_completed_rx) = oneshot::channel();
    let first_chunks = vec![
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_response_created("resp-1")),
        },
        StreamingSseChunk {
            gate: Some(gate_completed_rx),
            body: sse_event(responses::ev_completed("resp-1")),
        },
    ];
    let (api_server, completions) = start_streaming_sse_server(vec![first_chunks]).await;

    let realtime_server = start_websocket_server(vec![vec![vec![
        json!({
            "type": "session.updated",
            "session": { "id": "sess_non_blocking", "instructions": "backend prompt" }
        }),
        json!({
            "type": "conversation.input_transcript.delta",
            "delta": "delegate now"
        }),
        json!({
            "type": "conversation.handoff.requested",
            "handoff_id": "handoff_non_blocking",
            "item_id": "item_non_blocking",
            "input_transcript": "delegate now"
        }),
        json!({
            "type": "conversation.output_audio.delta",
            "delta": "AQID",
            "sample_rate": 24000,
            "channels": 1
        }),
    ]]])
    .await;

    let mut builder = test_codex().with_model("gpt-5.4").with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
        }
    });
    let test = builder.build_with_streaming_server(&api_server).await?;

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
        }) if session_id == "sess_non_blocking" => Some(()),
        _ => None,
    })
    .await;

    let _ = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::HandoffRequested(handoff),
        }) if handoff.input_transcript == "delegate now" => Some(()),
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
    .expect("timed out waiting for realtime audio while delegated turn was still pending");
    assert_eq!(audio_out.data, "AQID");

    let completion = completions
        .into_iter()
        .next()
        .expect("missing delegated turn completion");
    let _ = gate_completed_tx.send(());
    completion
        .await
        .expect("delegated turn request did not complete");
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    realtime_server.shutdown().await;
    api_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbound_handoff_request_steers_active_turn() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let (gate_completed_tx, gate_completed_rx) = oneshot::channel();
    let first_chunks = vec![
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_response_created("resp-1")),
        },
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_message_item_added("msg-1", "")),
        },
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_output_text_delta("first ")),
        },
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_output_text_delta("turn")),
        },
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_assistant_message("msg-1", "first turn")),
        },
        StreamingSseChunk {
            gate: Some(gate_completed_rx),
            body: sse_event(responses::ev_completed("resp-1")),
        },
    ];
    let second_chunks = vec![
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_response_created("resp-2")),
        },
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_completed("resp-2")),
        },
    ];
    let (api_server, completions) =
        start_streaming_sse_server(vec![first_chunks, second_chunks]).await;

    let realtime_server = start_websocket_server(vec![vec![
        vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_steer", "instructions": "backend prompt" }
        })],
        vec![],
        vec![
            json!({
                "type": "conversation.input_transcript.delta",
                "delta": "steer via realtime"
            }),
            json!({
                "type": "conversation.handoff.requested",
                "handoff_id": "handoff_steer",
                "item_id": "item_steer",
                "input_transcript": "steer via realtime"
            }),
        ],
    ]])
    .await;

    let mut builder = test_codex().with_model("gpt-5.4").with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
        }
    });
    let test = builder.build_with_streaming_server(&api_server).await?;

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
        }) if session_id == "sess_steer" => Some(()),
        _ => None,
    })
    .await;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "first prompt".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::AgentMessageContentDelta(_))
    })
    .await;
    let _ = wait_for_matching_websocket_request(
        &realtime_server,
        "first prompt mirrored to realtime",
        |request| websocket_request_text(request).as_deref() == Some("first prompt"),
    )
    .await;

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

    let _ = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::HandoffRequested(handoff),
        }) if handoff.input_transcript == "steer via realtime" => Some(()),
        _ => None,
    })
    .await;

    let mut completion_iter = completions.into_iter();
    let first_completion = completion_iter.next().expect("missing first completion");
    let second_completion = completion_iter.next().expect("missing second completion");

    let _ = gate_completed_tx.send(());
    first_completion
        .await
        .expect("first request did not complete");
    second_completion
        .await
        .expect("second request did not complete");
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = api_server.requests().await;
    assert_eq!(requests.len(), 2);

    let first_body: Value = serde_json::from_slice(&requests[0]).expect("parse first request");
    let second_body: Value = serde_json::from_slice(&requests[1]).expect("parse second request");
    let first_texts = message_input_texts(&first_body, "user");
    let second_texts = message_input_texts(&second_body, "user");

    assert!(first_texts.iter().any(|text| text == "first prompt"));
    assert!(
        !first_texts
            .iter()
            .any(|text| text
                == "<realtime_delegation>\n  <input>steer via realtime</input>\n  <transcript_delta>user: steer via realtime</transcript_delta>\n</realtime_delegation>")
    );
    assert!(second_texts.iter().any(|text| text == "first prompt"));
    assert!(second_texts.iter().any(|text| text
        == "<realtime_delegation>\n  <input>steer via realtime</input>\n  <transcript_delta>user: steer via realtime</transcript_delta>\n</realtime_delegation>"));

    realtime_server.shutdown().await;
    api_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbound_handoff_request_starts_turn_and_does_not_block_realtime_audio() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let (gate_completed_tx, gate_completed_rx) = oneshot::channel();
    let first_chunks = vec![
        StreamingSseChunk {
            gate: None,
            body: sse_event(responses::ev_response_created("resp-1")),
        },
        StreamingSseChunk {
            gate: Some(gate_completed_rx),
            body: sse_event(responses::ev_completed("resp-1")),
        },
    ];
    let (api_server, completions) = start_streaming_sse_server(vec![first_chunks]).await;

    let delegated_text = "delegate from handoff request";
    let realtime_server = start_websocket_server(vec![vec![vec![
        json!({
            "type": "session.updated",
            "session": { "id": "sess_handoff_request", "instructions": "backend prompt" }
        }),
        json!({
            "type": "conversation.input_transcript.delta",
            "delta": delegated_text
        }),
        json!({
            "type": "conversation.handoff.requested",
            "handoff_id": "handoff_audio",
            "item_id": "item_audio",
            "input_transcript": delegated_text
        }),
        json!({
            "type": "conversation.output_audio.delta",
            "delta": "AQID",
            "sample_rate": 24000,
            "channels": 1
        }),
    ]]])
    .await;

    let mut builder = test_codex().with_model("gpt-5.4").with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
        }
    });
    let test = builder.build_with_streaming_server(&api_server).await?;

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
        }) if session_id == "sess_handoff_request" => Some(()),
        _ => None,
    })
    .await;

    let _ = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::HandoffRequested(handoff),
        }) => (handoff.handoff_id == "handoff_audio" && handoff.input_transcript == delegated_text)
            .then_some(()),
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
    .expect("timed out waiting for realtime audio after handoff request");
    assert_eq!(audio_out.data, "AQID");

    let completion = completions
        .into_iter()
        .next()
        .expect("missing delegated turn completion");
    let _ = gate_completed_tx.send(());
    completion
        .await
        .expect("delegated turn request did not complete");
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = api_server.requests().await;
    assert_eq!(requests.len(), 1);
    let first_body: Value = serde_json::from_slice(&requests[0]).expect("parse first request");
    let first_texts = message_input_texts(&first_body, "user");
    let expected_text = format!(
        "<realtime_delegation>\n  <input>{delegated_text}</input>\n  <transcript_delta>user: {delegated_text}</transcript_delta>\n</realtime_delegation>"
    );
    assert!(first_texts.iter().any(|text| text == &expected_text));

    realtime_server.shutdown().await;
    api_server.shutdown().await;
    Ok(())
}
