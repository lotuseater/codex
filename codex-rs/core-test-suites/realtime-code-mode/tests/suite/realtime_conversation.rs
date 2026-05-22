#[path = "realtime_conversation/support.rs"]
mod support;

use support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_start_defaults_to_v2_and_gpt_realtime_1_5() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let api_server = start_mock_server().await;
    let realtime_server = start_websocket_server(vec![vec![vec![]]]).await;
    let realtime_base_url = realtime_server.uri().to_string();
    let mut builder = test_codex().with_config(move |config| {
        config.experimental_realtime_ws_base_url = Some(realtime_base_url);
        config.experimental_realtime_ws_startup_context = Some(String::new());
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

    let started = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationStarted(started) => Some(Ok(started.clone())),
        EventMsg::Error(err) => Some(Err(err.clone())),
        _ => None,
    })
    .await
    .unwrap_or_else(|err: ErrorEvent| panic!("conversation start failed: {err:?}"));

    assert!(
        realtime_server
            .wait_for_handshakes(/*expected*/ 1, Duration::from_secs(2))
            .await
    );

    let session_update = realtime_server
        .wait_for_request(/*connection_index*/ 0, /*request_index*/ 0)
        .await;
    let body = session_update.body_json();
    assert_eq!(
        json!({
            "startedVersion": started.version,
            "handshakeUri": realtime_server.single_handshake().uri(),
            "voice": body["session"]["audio"]["output"]["voice"],
            "instructions": body["session"]["instructions"],
        }),
        json!({
            "startedVersion": RealtimeConversationVersion::V2,
            "handshakeUri": "/v1/realtime?model=gpt-realtime-1.5",
            "voice": "marin",
            "instructions": "backend prompt",
        })
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_start_uses_openai_env_key_fallback_with_chatgpt_auth() -> Result<()> {
    if std::env::var_os(REALTIME_CONVERSATION_TEST_SUBPROCESS_ENV_VAR).is_none() {
        return run_realtime_conversation_test_in_subprocess(
            "suite::realtime_conversation::conversation_start_uses_openai_env_key_fallback_with_chatgpt_auth",
            Some("env-realtime-key"),
        );
    }

    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![
        vec![],
        vec![vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_env", "instructions": "backend prompt" }
        })]],
    ])
    .await;

    let mut builder = test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing());
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
    assert_eq!(session_updated, "sess_env");

    assert_eq!(
        server.handshakes()[1].header("authorization").as_deref(),
        Some("Bearer env-realtime-key")
    );

    test.codex.submit(Op::RealtimeConversationClose).await?;
    let _closed = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationClosed(closed) => Some(closed.clone()),
        _ => None,
    })
    .await;

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_audio_before_start_emits_error() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![]).await;
    let mut builder = test_codex();
    let test = builder.build_with_websocket_server(&server).await?;

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

    let err = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::Error(err) => Some(err.clone()),
        _ => None,
    })
    .await;
    assert_eq!(err.codex_error_info, Some(CodexErrorInfo::BadRequest));
    assert_eq!(err.message, "conversation is not running");

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_start_preflight_failure_emits_realtime_error_only() -> Result<()> {
    if std::env::var_os(REALTIME_CONVERSATION_TEST_SUBPROCESS_ENV_VAR).is_none() {
        return run_realtime_conversation_test_in_subprocess(
            "suite::realtime_conversation::conversation_start_preflight_failure_emits_realtime_error_only",
            /*openai_api_key*/ None,
        );
    }

    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![]).await;
    let mut builder = test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing());
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
    assert_eq!(err, "realtime conversation requires API key auth");

    let closed = timeout(Duration::from_millis(200), async {
        wait_for_event_match(&test.codex, |msg| match msg {
            EventMsg::RealtimeConversationClosed(closed) => Some(closed.clone()),
            _ => None,
        })
        .await
    })
    .await;
    assert!(closed.is_err(), "preflight failure should not emit closed");

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_text_before_start_emits_error() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![]).await;
    let mut builder = test_codex();
    let test = builder.build_with_websocket_server(&server).await?;

    test.codex
        .submit(Op::RealtimeConversationText(ConversationTextParams {
            text: "hello".to_string(),
        }))
        .await?;

    let err = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::Error(err) => Some(err.clone()),
        _ => None,
    })
    .await;
    assert_eq!(err.codex_error_info, Some(CodexErrorInfo::BadRequest));
    assert_eq!(err.message, "conversation is not running");

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_second_start_replaces_runtime() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![
        vec![],
        vec![vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_old", "instructions": "old" }
        })]],
        vec![
            vec![json!({
                "type": "session.updated",
                "session": { "id": "sess_new", "instructions": "new" }
            })],
            vec![json!({
                "type": "conversation.output_audio.delta",
                "delta": "AQID",
                "sample_rate": 24000,
                "channels": 1
            })],
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
            prompt: Some(Some("old".to_string())),
            realtime_session_id: Some("conv_old".to_string()),
            transport: None,
            voice: None,
        }))
        .await?;
    wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload:
                RealtimeEvent::SessionUpdated {
                    realtime_session_id: session_id,
                    ..
                },
        }) if session_id == "sess_old" => Some(Ok(())),
        EventMsg::Error(err) => Some(Err(err.clone())),
        _ => None,
    })
    .await
    .unwrap_or_else(|err: ErrorEvent| panic!("first conversation start failed: {err:?}"));

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("new".to_string())),
            realtime_session_id: Some("conv_new".to_string()),
            transport: None,
            voice: None,
        }))
        .await?;
    wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload:
                RealtimeEvent::SessionUpdated {
                    realtime_session_id: session_id,
                    ..
                },
        }) if session_id == "sess_new" => Some(Ok(())),
        EventMsg::Error(err) => Some(Err(err.clone())),
        _ => None,
    })
    .await
    .unwrap_or_else(|err: ErrorEvent| panic!("second conversation start failed: {err:?}"));

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
            payload: RealtimeEvent::AudioOut(frame),
        }) if frame.data == "AQID" => Some(()),
        _ => None,
    })
    .await;

    let connections = server.connections();
    assert_eq!(connections.len(), 3);
    assert_eq!(connections[1].len(), 1);
    let old_instructions =
        websocket_request_instructions(&connections[1][0]).expect("old session instructions");
    assert!(old_instructions.starts_with("old"));
    assert_eq!(
        server.handshakes()[1].header("x-session-id").as_deref(),
        Some("conv_old")
    );
    assert_eq!(connections[2].len(), 2);
    let new_instructions =
        websocket_request_instructions(&connections[2][0]).expect("new session instructions");
    assert!(new_instructions.starts_with("new"));
    assert_eq!(
        server.handshakes()[2].header("x-session-id").as_deref(),
        Some("conv_new")
    );
    assert_eq!(
        connections[2][1].body_json()["type"].as_str(),
        Some("input_audio_buffer.append")
    );

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_uses_experimental_realtime_ws_base_url_override() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let startup_server = start_websocket_server(vec![vec![]]).await;
    let realtime_server = start_websocket_server(vec![vec![vec![json!({
        "type": "session.updated",
        "session": { "id": "sess_override", "instructions": "backend prompt" }
    })]]])
    .await;

    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
        }
    });
    let test = builder.build_with_websocket_server(&startup_server).await?;
    assert!(
        startup_server
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
    assert_eq!(session_updated, "sess_override");

    let startup_connections = startup_server.connections();
    assert_eq!(startup_connections.len(), 1);

    let realtime_connections = realtime_server.connections();
    assert_eq!(realtime_connections.len(), 1);
    assert_eq!(
        realtime_connections[0][0].body_json()["type"].as_str(),
        Some("session.update")
    );

    startup_server.shutdown().await;
    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_uses_default_realtime_backend_prompt() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![
        vec![],
        vec![vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_default", "instructions": "default" }
        })]],
    ])
    .await;

    let mut builder = test_codex().with_config(|config| {
        config.experimental_realtime_ws_startup_context =
            Some("controlled startup context".to_string());
    });
    let test = builder.build_with_websocket_server(&server).await?;
    assert!(
        server
            .wait_for_handshakes(/*expected*/ 1, Duration::from_secs(2))
            .await
    );

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: None,
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
    assert_eq!(session_updated, "sess_default");

    let connections = server.connections();
    assert_eq!(connections.len(), 2);
    let instructions =
        websocket_request_instructions(&connections[1][0]).expect("default session instructions");
    assert_eq!(
        instructions,
        format!(
            "{}\n\ncontrolled startup context",
            expected_realtime_backend_prompt()
        )
    );

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_uses_empty_instructions_for_null_or_empty_prompt() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![
        vec![],
        vec![vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_null", "instructions": "" }
        })]],
        vec![vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_empty", "instructions": "" }
        })]],
    ])
    .await;

    let mut builder = test_codex().with_config(|config| {
        config.experimental_realtime_ws_startup_context = Some(String::new());
    });
    let test = builder.build_with_websocket_server(&server).await?;
    assert!(
        server
            .wait_for_handshakes(/*expected*/ 1, Duration::from_secs(2))
            .await
    );

    for (prompt, expected_session_id) in [
        (Some(None), "sess_null"),
        (Some(Some(String::new())), "sess_empty"),
    ] {
        test.codex
            .submit(Op::RealtimeConversationStart(ConversationStartParams {
                output_modality: RealtimeOutputModality::Audio,
                prompt,
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
        assert_eq!(session_updated, expected_session_id);

        test.codex.submit(Op::RealtimeConversationClose).await?;
        let _closed = wait_for_event_match(&test.codex, |msg| match msg {
            EventMsg::RealtimeConversationClosed(closed) => Some(closed.clone()),
            _ => None,
        })
        .await;
    }

    let connections = server.connections();
    assert_eq!(connections.len(), 3);
    let null_instructions =
        websocket_request_instructions(&connections[1][0]).expect("null prompt instructions");
    let empty_instructions =
        websocket_request_instructions(&connections[2][0]).expect("empty prompt instructions");
    assert_eq!(null_instructions, "");
    assert_eq!(empty_instructions, "");

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_uses_explicit_start_voice() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![
        vec![],
        vec![vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_voice", "instructions": "backend prompt" }
        })]],
    ])
    .await;
    let test = test_codex().build_with_websocket_server(&server).await?;
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
            voice: Some(RealtimeVoice::Breeze),
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
    assert_eq!(session_updated, "sess_voice");

    let connections = server.connections();
    assert_eq!(
        connections[1][0].body_json()["session"]["audio"]["output"]["voice"],
        "breeze"
    );

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_uses_configured_realtime_voice() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![
        vec![],
        vec![vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_config_voice", "instructions": "backend prompt" }
        })]],
    ])
    .await;
    let mut builder = test_codex().with_config(|config| {
        config.realtime.voice = Some(RealtimeVoice::Cove);
    });
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
    assert_eq!(session_updated, "sess_config_voice");

    let connections = server.connections();
    assert_eq!(
        connections[1][0].body_json()["session"]["audio"]["output"]["voice"],
        "cove"
    );

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_rejects_voice_for_wrong_realtime_version() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let api_server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config.realtime.version = RealtimeWsVersion::V2;
    });
    let test = builder.build(&api_server).await?;

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: Some(RealtimeVoice::Cove),
        }))
        .await?;

    let error = wait_for_event_match(&test.codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload: RealtimeEvent::Error(message),
        }) => Some(message.clone()),
        _ => None,
    })
    .await;
    assert!(error.contains("realtime voice `cove` is not supported for v2"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_uses_experimental_realtime_ws_backend_prompt_override() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_websocket_server(vec![
        vec![],
        vec![vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_override", "instructions": "prompt from config" }
        })]],
    ])
    .await;

    let mut builder = test_codex().with_config(|config| {
        config.experimental_realtime_ws_backend_prompt = Some("prompt from config".to_string());
    });
    let test = builder.build_with_websocket_server(&server).await?;
    assert!(
        server
            .wait_for_handshakes(/*expected*/ 1, Duration::from_secs(2))
            .await
    );

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("prompt from op".to_string())),
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
    assert_eq!(session_updated, "sess_override");

    let connections = server.connections();
    assert_eq!(connections.len(), 2);
    let overridden_instructions = websocket_request_instructions(&connections[1][0])
        .expect("overridden session instructions");
    assert!(overridden_instructions.starts_with("prompt from config"));

    server.shutdown().await;
    Ok(())
}
