#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::client_websockets_support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_streams_request() {
    skip_if_no_network!();

    let server = start_websocket_server(vec![vec![vec![
        ev_response_created("resp-1"),
        ev_completed("resp-1"),
    ]]])
    .await;

    let harness = websocket_harness(&server).await;
    let mut client_session = harness.client.new_session();
    let prompt = prompt_with_input(vec![message_item("hello")]);

    stream_until_complete(&mut client_session, &harness, &prompt).await;

    let connection = server.single_connection();
    assert_eq!(connection.len(), 1);
    let body = connection.first().expect("missing request").body_json();

    assert_eq!(body["type"].as_str(), Some("response.create"));
    assert_eq!(body["model"].as_str(), Some(MODEL));
    assert_eq!(body["stream"], serde_json::Value::Bool(true));
    assert_eq!(body["input"].as_array().map(Vec::len), Some(1));
    let handshake = server.single_handshake();
    assert_eq!(
        handshake.header(OPENAI_BETA_HEADER),
        Some(WS_V2_BETA_HEADER_VALUE.to_string())
    );
    assert_eq!(
        handshake.header(X_CLIENT_REQUEST_ID_HEADER),
        Some(harness.thread_id.to_string())
    );
    assert_eq!(
        handshake.header("session-id"),
        Some(harness.session_id.to_string())
    );
    assert_eq!(
        handshake.header("thread-id"),
        Some(harness.thread_id.to_string())
    );
    assert_eq!(
        handshake.header(USER_AGENT_HEADER),
        Some(codex_login::default_client::get_codex_user_agent())
    );
    assert_eq!(
        body["client_metadata"]["x-codex-installation-id"].as_str(),
        Some(TEST_INSTALLATION_ID)
    );
    let stream_request_start_ms = body["client_metadata"]
        [X_CODEX_WS_STREAM_REQUEST_START_MS_CLIENT_METADATA_KEY]
        .as_str()
        .expect("missing websocket stream request start timestamp")
        .parse::<i64>()
        .expect("websocket stream request start timestamp should be an integer");
    assert!(stream_request_start_ms > 0);

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_sends_response_processed_when_feature_enabled() {
    skip_if_no_network!();

    let server = start_websocket_server(vec![vec![
        vec![
            ev_response_created("resp-prewarm"),
            ev_completed("resp-prewarm"),
        ],
        vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "hi"),
            ev_completed("resp-1"),
        ],
        vec![],
    ]])
    .await;

    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::ResponsesWebsocketResponseProcessed)
            .expect("test config should allow feature update");
    });
    let test = builder
        .build_with_websocket_server(&server)
        .await
        .expect("build websocket codex");

    test.submit_turn("hello")
        .await
        .expect("submission should send response.processed after processing");

    let processed = server
        .wait_for_request(/*connection_index*/ 0, /*request_index*/ 2)
        .await;
    assert_eq!(
        processed.body_json(),
        json!({
            "type": "response.processed",
            "response_id": "resp-1",
        })
    );

    let connection = server.single_connection();
    assert_eq!(connection.len(), 3);
    assert_eq!(
        connection[1].body_json()["type"].as_str(),
        Some("response.create")
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_sends_response_processed_after_remote_compaction_v2() {
    skip_if_no_network!();

    let server = start_websocket_server(vec![vec![
        vec![
            ev_response_created("resp-prewarm"),
            ev_completed("resp-prewarm"),
        ],
        vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "hi"),
            ev_completed("resp-1"),
        ],
        vec![],
        vec![
            json!({
                "type": "response.output_item.done",
                "item": {
                    "type": "compaction",
                    "encrypted_content": "ENCRYPTED_CONTEXT_COMPACTION_SUMMARY",
                }
            }),
            ev_completed("resp-compact"),
        ],
        vec![],
    ]])
    .await;

    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::RemoteCompactionV2)
            .expect("test config should allow feature update");
        config
            .features
            .enable(Feature::ResponsesWebsocketResponseProcessed)
            .expect("test config should allow feature update");
    });
    let test = builder
        .build_with_websocket_server(&server)
        .await
        .expect("build websocket codex");

    test.submit_turn("hello")
        .await
        .expect("submission should send response.processed after processing");

    test.codex
        .submit(Op::Compact)
        .await
        .expect("compact submission should succeed");
    wait_for_event(&test.codex, |msg| matches!(msg, EventMsg::TurnComplete(_))).await;

    let compact_processed = server
        .wait_for_request(/*connection_index*/ 0, /*request_index*/ 4)
        .await;
    assert_eq!(
        compact_processed.body_json(),
        json!({
            "type": "response.processed",
            "response_id": "resp-compact",
        })
    );

    let connection = server.single_connection();
    assert_eq!(connection.len(), 5);
    assert_eq!(
        connection[3].body_json()["type"].as_str(),
        Some("response.create")
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_omits_response_processed_without_feature() {
    skip_if_no_network!();

    let server = start_websocket_server(vec![vec![
        vec![
            ev_response_created("resp-prewarm"),
            ev_completed("resp-prewarm"),
        ],
        vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "hi"),
            ev_completed("resp-1"),
        ],
        vec![],
    ]])
    .await;
    let mut builder = test_codex();
    let test = builder
        .build_with_websocket_server(&server)
        .await
        .expect("build websocket codex");

    test.submit_turn("hello")
        .await
        .expect("submission should complete without response.processed");

    let connection = server.single_connection();
    assert_eq!(connection.len(), 2);

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[traced_test]
async fn responses_websocket_emits_websocket_telemetry_events() {
    skip_if_no_network!();

    let server = start_websocket_server(vec![vec![vec![
        ev_response_created("resp-1"),
        ev_completed("resp-1"),
    ]]])
    .await;

    let harness = websocket_harness(&server).await;
    harness.session_telemetry.reset_runtime_metrics();
    let mut client_session = harness.client.new_session();
    let prompt = prompt_with_input(vec![message_item("hello")]);

    stream_until_complete(&mut client_session, &harness, &prompt).await;

    tokio::time::sleep(Duration::from_millis(10)).await;

    let summary = harness
        .session_telemetry
        .runtime_metrics_summary()
        .expect("runtime metrics summary");
    assert_eq!(summary.api_calls.count, 0);
    assert_eq!(summary.streaming_events.count, 0);
    assert_eq!(summary.websocket_calls.count, 1);
    assert_eq!(summary.websocket_events.count, 2);

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_emits_reasoning_included_event() {
    skip_if_no_network!();

    let server = start_websocket_server_with_headers(vec![WebSocketConnectionConfig {
        requests: vec![vec![ev_response_created("resp-1"), ev_completed("resp-1")]],
        response_headers: vec![("X-Reasoning-Included".to_string(), "true".to_string())],
        accept_delay: None,
        close_after_requests: true,
    }])
    .await;

    let harness = websocket_harness(&server).await;
    let mut client_session = harness.client.new_session();
    let prompt = prompt_with_input(vec![message_item("hello")]);

    let mut stream = client_session
        .stream(
            &prompt,
            &harness.model_info,
            &harness.session_telemetry,
            harness.effort,
            harness.summary,
            /*service_tier*/ None,
            /*turn_metadata_header*/ None,
            &codex_rollout_trace::InferenceTraceContext::disabled(),
        )
        .await
        .expect("websocket stream failed");

    let mut saw_reasoning_included = false;
    while let Some(event) = stream.next().await {
        match event.expect("event") {
            ResponseEvent::ServerReasoningIncluded(true) => {
                saw_reasoning_included = true;
            }
            ResponseEvent::Completed { .. } => break,
            _ => {}
        }
    }

    assert!(saw_reasoning_included);
    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_emits_rate_limit_events() {
    skip_if_no_network!();

    let rate_limit_event = json!({
        "type": "codex.rate_limits",
        "plan_type": "plus",
        "rate_limits": {
            "allowed": true,
            "limit_reached": false,
            "primary": {
                "used_percent": 42,
                "window_minutes": 60,
                "reset_at": 1700000000
            },
            "secondary": null
        },
        "code_review_rate_limits": null,
        "credits": {
            "has_credits": true,
            "unlimited": false,
            "balance": "123"
        },
        "promo": null
    });

    let server = start_websocket_server_with_headers(vec![WebSocketConnectionConfig {
        requests: vec![vec![
            rate_limit_event,
            ev_response_created("resp-1"),
            ev_completed("resp-1"),
        ]],
        response_headers: vec![
            ("X-Models-Etag".to_string(), "etag-123".to_string()),
            ("X-Reasoning-Included".to_string(), "true".to_string()),
        ],
        accept_delay: None,
        close_after_requests: true,
    }])
    .await;

    let harness = websocket_harness(&server).await;
    let mut client_session = harness.client.new_session();
    let prompt = prompt_with_input(vec![message_item("hello")]);

    let mut stream = client_session
        .stream(
            &prompt,
            &harness.model_info,
            &harness.session_telemetry,
            harness.effort,
            harness.summary,
            /*service_tier*/ None,
            /*turn_metadata_header*/ None,
            &codex_rollout_trace::InferenceTraceContext::disabled(),
        )
        .await
        .expect("websocket stream failed");

    let mut saw_rate_limits = None;
    let mut saw_models_etag = None;
    let mut saw_reasoning_included = false;

    while let Some(event) = stream.next().await {
        match event.expect("event") {
            ResponseEvent::RateLimits(snapshot) => {
                saw_rate_limits = Some(snapshot);
            }
            ResponseEvent::ModelsEtag(etag) => {
                saw_models_etag = Some(etag);
            }
            ResponseEvent::ServerReasoningIncluded(true) => {
                saw_reasoning_included = true;
            }
            ResponseEvent::Completed { .. } => break,
            _ => {}
        }
    }

    let rate_limits = saw_rate_limits.expect("missing rate limits");
    let primary = rate_limits.primary.expect("missing primary window");
    assert_eq!(primary.used_percent, 42.0);
    assert_eq!(primary.window_minutes, Some(60));
    assert_eq!(primary.resets_at, Some(1_700_000_000));
    assert_eq!(rate_limits.plan_type, Some(PlanType::Plus));
    let credits = rate_limits.credits.expect("missing credits");
    assert!(credits.has_credits);
    assert!(!credits.unlimited);
    assert_eq!(credits.balance.as_deref(), Some("123"));
    assert_eq!(saw_models_etag.as_deref(), Some("etag-123"));
    assert!(saw_reasoning_included);

    server.shutdown().await;
}
