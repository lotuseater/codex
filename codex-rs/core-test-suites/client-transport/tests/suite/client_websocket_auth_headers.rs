#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::client_websockets_support::*;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_includes_timing_metrics_header_when_runtime_metrics_enabled() {
    skip_if_no_network!();

    let server = start_websocket_server(vec![vec![vec![
        ev_response_created("resp-1"),
        serde_json::json!({
            "type": "responsesapi.websocket_timing",
            "timing_metrics": {
                "responses_duration_excl_engine_and_client_tool_time_ms": 120,
                "engine_service_total_ms": 450,
                "engine_iapi_ttft_total_ms": 310,
                "engine_service_ttft_total_ms": 340,
                "engine_iapi_tbt_across_engine_calls_ms": 220,
                "engine_service_tbt_across_engine_calls_ms": 260
            }
        }),
        ev_completed("resp-1"),
    ]]])
    .await;

    let harness =
        websocket_harness_with_runtime_metrics(&server, /*runtime_metrics_enabled*/ true).await;
    harness.session_telemetry.reset_runtime_metrics();
    let mut client_session = harness.client.new_session();
    let prompt = prompt_with_input(vec![message_item("hello")]);

    stream_until_complete(&mut client_session, &harness, &prompt).await;
    tokio::time::sleep(Duration::from_millis(10)).await;

    let handshake = server.single_handshake();
    assert_eq!(
        handshake.header(X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER),
        Some("true".to_string())
    );

    let summary = harness
        .session_telemetry
        .runtime_metrics_summary()
        .expect("runtime metrics summary");
    assert_eq!(summary.responses_api_overhead_ms, 120);
    assert_eq!(summary.responses_api_inference_time_ms, 450);
    assert_eq!(summary.responses_api_engine_iapi_ttft_ms, 310);
    assert_eq!(summary.responses_api_engine_service_ttft_ms, 340);
    assert_eq!(summary.responses_api_engine_iapi_tbt_ms, 220);
    assert_eq!(summary.responses_api_engine_service_tbt_ms, 260);

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_omits_timing_metrics_header_when_runtime_metrics_disabled() {
    skip_if_no_network!();

    let server = start_websocket_server(vec![vec![vec![
        ev_response_created("resp-1"),
        ev_completed("resp-1"),
    ]]])
    .await;

    let harness =
        websocket_harness_with_runtime_metrics(&server, /*runtime_metrics_enabled*/ false).await;
    let mut client_session = harness.client.new_session();
    let prompt = prompt_with_input(vec![message_item("hello")]);

    stream_until_complete(&mut client_session, &harness, &prompt).await;

    let handshake = server.single_handshake();
    assert_eq!(
        handshake.header(X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER),
        None
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_forwards_turn_metadata_on_initial_and_incremental_create() {
    skip_if_no_network!();

    let server = start_websocket_server(vec![vec![
        vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "assistant output"),
            ev_completed("resp-1"),
        ],
        vec![ev_response_created("resp-2"), ev_completed("resp-2")],
    ]])
    .await;

    let harness = websocket_harness(&server).await;
    let mut client_session = harness.client.new_session();
    let first_turn_metadata =
        r#"{"turn_id":"turn-123","thread_source":"user","sandbox":"workspace-write"}"#;
    let enriched_turn_metadata = r#"{"turn_id":"turn-123","thread_source":"user","sandbox":"workspace-write","workspaces":[{"root_path":"/tmp/repo","latest_git_commit_hash":"abc123","associated_remote_urls":["git@github.com:openai/codex.git"],"has_changes":true}]}"#;
    let prompt_one = prompt_with_input(vec![message_item("hello")]);
    let prompt_two = prompt_with_input(vec![
        message_item("hello"),
        assistant_message_item("msg-1", "assistant output"),
        message_item("second"),
    ]);

    stream_until_complete_with_turn_metadata(
        &mut client_session,
        &harness,
        &prompt_one,
        /*service_tier*/ None,
        Some(first_turn_metadata),
    )
    .await;
    stream_until_complete_with_turn_metadata(
        &mut client_session,
        &harness,
        &prompt_two,
        /*service_tier*/ None,
        Some(enriched_turn_metadata),
    )
    .await;

    let connection = server.single_connection();
    assert_eq!(connection.len(), 2);
    let first = connection.first().expect("missing request").body_json();
    let second = connection.get(1).expect("missing request").body_json();

    assert_eq!(first["type"].as_str(), Some("response.create"));
    assert_eq!(
        first["client_metadata"]["x-codex-turn-metadata"].as_str(),
        Some(first_turn_metadata)
    );
    assert_eq!(second["type"].as_str(), Some("response.create"));
    assert_eq!(second["previous_response_id"].as_str(), Some("resp-1"));
    assert_eq!(
        second["client_metadata"]["x-codex-turn-metadata"].as_str(),
        Some(enriched_turn_metadata)
    );

    let first_metadata: serde_json::Value =
        serde_json::from_str(first_turn_metadata).expect("first metadata should be valid json");
    let second_metadata: serde_json::Value = serde_json::from_str(enriched_turn_metadata)
        .expect("enriched metadata should be valid json");

    assert_eq!(first_metadata["turn_id"].as_str(), Some("turn-123"));
    assert_eq!(second_metadata["turn_id"].as_str(), Some("turn-123"));
    assert_eq!(first_metadata["thread_source"].as_str(), Some("user"));
    assert_eq!(second_metadata["thread_source"].as_str(), Some("user"));
    assert_eq!(
        second_metadata["workspaces"][0]["has_changes"].as_bool(),
        Some(true)
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_preserves_custom_turn_metadata_fields() {
    skip_if_no_network!();

    let server = start_websocket_server(vec![vec![vec![
        ev_response_created("resp-1"),
        ev_completed("resp-1"),
    ]]])
    .await;

    let harness = websocket_harness(&server).await;
    let mut client_session = harness.client.new_session();
    let prompt = prompt_with_input(vec![message_item("hello")]);
    let turn_metadata = json!({
        "turn_id": "turn-123",
        "fiber_run_id": "fiber-123",
        "origin": "app-server",
    })
    .to_string();

    stream_until_complete_with_turn_metadata(
        &mut client_session,
        &harness,
        &prompt,
        /*service_tier*/ None,
        Some(&turn_metadata),
    )
    .await;

    let body = server
        .single_connection()
        .first()
        .expect("missing request")
        .body_json();

    assert_eq!(body["type"].as_str(), Some("response.create"));
    assert_eq!(
        body["client_metadata"]["x-codex-turn-metadata"]
            .as_str()
            .map(|value| serde_json::from_str::<serde_json::Value>(value).expect("valid json")),
        Some(json!({
            "turn_id": "turn-123",
            "fiber_run_id": "fiber-123",
            "origin": "app-server",
        }))
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_websocket_v2_sets_openai_beta_header() {
    skip_if_no_network!();

    let server = start_websocket_server(vec![vec![vec![
        ev_response_created("resp-1"),
        ev_completed("resp-1"),
    ]]])
    .await;

    let harness = websocket_harness_with_v2(&server, /*runtime_metrics_enabled*/ true).await;
    let mut session = harness.client.new_session();
    let prompt = prompt_with_input(vec![message_item("hello")]);

    stream_until_complete(&mut session, &harness, &prompt).await;

    let handshake = server.single_handshake();
    let openai_beta_header = handshake
        .header(OPENAI_BETA_HEADER)
        .expect("missing OpenAI-Beta header");
    assert!(
        openai_beta_header
            .split(',')
            .map(str::trim)
            .any(|value| value == WS_V2_BETA_HEADER_VALUE)
    );
    server.shutdown().await;
}
