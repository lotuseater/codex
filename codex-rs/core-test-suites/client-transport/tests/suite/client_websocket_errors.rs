#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::client_websockets_support::*;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_usage_limit_error_emits_rate_limit_event() {
    skip_if_no_network!();

    let usage_limit_error = json!({
        "type": "error",
        "status": 429,
        "error": {
            "type": "usage_limit_reached",
            "message": "The usage limit has been reached",
            "plan_type": "pro",
            "resets_at": 1704067242,
            "resets_in_seconds": 1234
        },
        "headers": {
            "x-codex-primary-used-percent": "100.0",
            "x-codex-secondary-used-percent": "87.5",
            "x-codex-primary-over-secondary-limit-percent": "95.0",
            "x-codex-primary-window-minutes": "15",
            "x-codex-secondary-window-minutes": "60"
        }
    });

    let server = start_websocket_server(vec![vec![
        vec![
            ev_response_created("resp-prewarm"),
            ev_completed("resp-prewarm"),
        ],
        vec![usage_limit_error],
    ]])
    .await;
    let mut builder = test_codex().with_config(|config| {
        config.model_provider.request_max_retries = Some(0);
        config.model_provider.stream_max_retries = Some(0);
    });
    let test = builder
        .build_with_websocket_server(&server)
        .await
        .expect("build websocket codex");

    let submission_id = test
        .codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submission should succeed while emitting usage limit error events");

    let token_event =
        wait_for_event(&test.codex, |msg| matches!(msg, EventMsg::TokenCount(_))).await;
    let EventMsg::TokenCount(event) = token_event else {
        unreachable!();
    };

    let event_json = serde_json::to_value(&event).expect("serialize token count event");
    pretty_assertions::assert_eq!(
        event_json,
        json!({
            "info": null,
            "rate_limits": {
                "limit_id": "codex",
                "limit_name": null,
                "primary": {
                    "used_percent": 100.0,
                    "window_minutes": 15,
                    "resets_at": null
                },
                "secondary": {
                    "used_percent": 87.5,
                    "window_minutes": 60,
                    "resets_at": null
                },
                "credits": null,
                "plan_type": null,
                "rate_limit_reached_type": null
            }
        })
    );

    let error_event = wait_for_event(&test.codex, |msg| matches!(msg, EventMsg::Error(_))).await;
    let EventMsg::Error(error_event) = error_event else {
        unreachable!();
    };
    assert!(
        error_event.message.to_lowercase().contains("usage limit"),
        "unexpected error message for submission {submission_id}: {}",
        error_event.message
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_invalid_request_error_with_status_is_forwarded() {
    skip_if_no_network!();

    let invalid_request_error = json!({
        "type": "error",
        "status": 400,
        "error": {
            "type": "invalid_request_error",
            "message": "Model 'castor-raikou-0205-ev3' does not support image inputs."
        }
    });

    let server = start_websocket_server(vec![vec![
        vec![
            ev_response_created("resp-prewarm"),
            ev_completed("resp-prewarm"),
        ],
        vec![invalid_request_error],
    ]])
    .await;
    let mut builder = test_codex().with_config(|config| {
        config.model_provider.request_max_retries = Some(0);
        config.model_provider.stream_max_retries = Some(0);
    });
    let test = builder
        .build_with_websocket_server(&server)
        .await
        .expect("build websocket codex");

    let submission_id = test
        .codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submission should succeed while emitting invalid request events");

    let error_event = wait_for_event(&test.codex, |msg| matches!(msg, EventMsg::Error(_))).await;
    let EventMsg::Error(error_event) = error_event else {
        unreachable!();
    };
    assert!(
        error_event
            .message
            .to_lowercase()
            .contains("does not support image inputs"),
        "unexpected error message for submission {submission_id}: {}",
        error_event.message
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_connection_limit_error_reconnects_and_completes() {
    skip_if_no_network!();

    let websocket_connection_limit_error = json!({
        "type": "error",
        "status": 400,
        "error": {
            "type": "invalid_request_error",
            "code": "websocket_connection_limit_reached",
            "message": "Responses websocket connection limit reached (60 minutes). Create a new websocket connection to continue."
        }
    });

    let server = start_websocket_server(vec![
        vec![vec![websocket_connection_limit_error]],
        vec![vec![ev_response_created("resp-1"), ev_completed("resp-1")]],
    ])
    .await;
    let mut builder = test_codex().with_config(|config| {
        config.model_provider.request_max_retries = Some(0);
        config.model_provider.stream_max_retries = Some(1);
    });
    let test = builder
        .build_with_websocket_server(&server)
        .await
        .expect("build websocket codex");

    test.submit_turn("hello")
        .await
        .expect("submission should reconnect after websocket connection limit error");

    let total_websocket_requests: usize = server.connections().iter().map(Vec::len).sum();
    assert_eq!(total_websocket_requests, 2);
    let handshake_user_agents: Vec<_> = server
        .handshakes()
        .iter()
        .map(|handshake| handshake.header(USER_AGENT_HEADER))
        .collect();
    assert_eq!(
        handshake_user_agents,
        vec![
            Some(codex_login::default_client::get_codex_user_agent()),
            Some(codex_login::default_client::get_codex_user_agent()),
        ]
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_v2_after_error_uses_full_create_without_previous_response_id() {
    skip_if_no_network!();

    let server = start_websocket_server(vec![
        vec![
            vec![ev_response_created("resp-1"), ev_completed("resp-1")],
            vec![json!({
                "type": "response.failed",
                "response": {
                    "error": {
                        "code": "invalid_prompt",
                        "message": "synthetic websocket failure"
                    }
                }
            })],
        ],
        vec![vec![ev_response_created("resp-3"), ev_completed("resp-3")]],
    ])
    .await;

    let harness = websocket_harness_with_v2(&server, /*runtime_metrics_enabled*/ true).await;
    let mut session = harness.client.new_session();
    let prompt_one = prompt_with_input(vec![message_item("hello")]);
    let prompt_two = prompt_with_input(vec![message_item("hello"), message_item("second")]);
    let prompt_three = prompt_with_input(vec![
        message_item("hello"),
        message_item("second"),
        message_item("third"),
    ]);

    stream_until_complete(&mut session, &harness, &prompt_one).await;

    let mut second_stream = session
        .stream(
            &prompt_two,
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
    let mut saw_error = false;
    while let Some(event) = second_stream.next().await {
        if event.is_err() {
            saw_error = true;
            break;
        }
    }
    assert!(saw_error, "expected second websocket stream to error");

    stream_until_complete(&mut session, &harness, &prompt_three).await;

    assert_eq!(server.handshakes().len(), 2);

    let connections = server.connections();
    assert_eq!(connections.len(), 2);
    let first_connection = connections.first().expect("missing first connection");
    assert_eq!(first_connection.len(), 2);

    let first = first_connection
        .first()
        .expect("missing first request")
        .body_json();
    let second = first_connection
        .get(1)
        .expect("missing second request")
        .body_json();
    let third = connections
        .get(1)
        .and_then(|connection| connection.first())
        .expect("missing third request")
        .body_json();

    assert_eq!(first["type"].as_str(), Some("response.create"));
    assert_eq!(second["type"].as_str(), Some("response.create"));
    assert_eq!(second["previous_response_id"].as_str(), Some("resp-1"));
    assert_eq!(third["type"].as_str(), Some("response.create"));
    assert_eq!(third.get("previous_response_id"), None);
    assert_eq!(
        third["input"],
        serde_json::to_value(&prompt_three.input).unwrap()
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_v2_surfaces_terminal_error_without_close_handshake() {
    skip_if_no_network!();

    let server = start_websocket_server_with_headers(vec![WebSocketConnectionConfig {
        requests: vec![
            vec![ev_response_created("resp-1"), ev_completed("resp-1")],
            vec![json!({
                "type": "response.failed",
                "response": {
                    "error": {
                        "code": "invalid_prompt",
                        "message": "synthetic websocket failure"
                    }
                }
            })],
        ],
        response_headers: Vec::new(),
        accept_delay: None,
        close_after_requests: false,
    }])
    .await;

    let harness = websocket_harness_with_v2(&server, /*runtime_metrics_enabled*/ true).await;
    let mut session = harness.client.new_session();
    let prompt_one = prompt_with_input(vec![message_item("hello")]);
    let prompt_two = prompt_with_input(vec![message_item("hello"), message_item("second")]);

    stream_until_complete(&mut session, &harness, &prompt_one).await;

    let mut second_stream = session
        .stream(
            &prompt_two,
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

    let saw_error = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(event) = second_stream.next().await {
            if event.is_err() {
                return true;
            }
        }
        false
    })
    .await
    .expect("timed out waiting for terminal websocket error");

    assert!(saw_error, "expected second websocket stream to error");

    server.shutdown().await;
}
