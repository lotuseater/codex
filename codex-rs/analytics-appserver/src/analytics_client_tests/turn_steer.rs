use super::*;
use pretty_assertions::assert_eq;
use super::common::*;

#[tokio::test]
async fn accepted_turn_steer_emits_expected_event() {
    let mut reducer = AnalyticsReducer::default();
    let mut out = Vec::new();

    ingest_turn_prerequisites(
        &mut reducer,
        &mut out,
        /*include_initialize*/ true,
        /*include_resolved_config*/ false,
        /*include_started*/ false,
        /*include_token_usage*/ false,
    )
    .await;
    reducer
        .ingest(
            AnalyticsFact::ClientRequest {
                connection_id: 7,
                request_id: RequestId::Integer(4),
                request: Box::new(sample_turn_steer_request(
                    "thread-2", "turn-2", /*request_id*/ 4,
                )),
            },
            &mut out,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::ClientResponse {
                connection_id: 7,
                request_id: RequestId::Integer(4),
                response: Box::new(sample_turn_steer_response("turn-2")),
            },
            &mut out,
        )
        .await;

    assert_eq!(out.len(), 1);
    let payload = serde_json::to_value(&out[0]).expect("serialize turn steer event");
    assert_eq!(payload["event_type"], json!("codex_turn_steer_event"));
    assert_eq!(payload["event_params"]["thread_id"], json!("thread-2"));
    assert_eq!(payload["event_params"]["expected_turn_id"], json!("turn-2"));
    assert_eq!(payload["event_params"]["accepted_turn_id"], json!("turn-2"));
    assert_eq!(payload["event_params"]["num_input_images"], json!(1));
    assert_eq!(payload["event_params"]["result"], json!("accepted"));
    assert_eq!(payload["event_params"]["rejection_reason"], json!(null));
    assert!(
        payload["event_params"]["created_at"]
            .as_u64()
            .expect("created_at")
            > 0
    );
    assert_eq!(
        payload["event_params"]["app_server_client"]["product_client_id"],
        json!("codex-tui")
    );
    assert_eq!(
        payload["event_params"]["runtime"]["codex_rs_version"],
        json!("0.1.0")
    );
    assert_eq!(payload["event_params"]["thread_source"], json!("user"));
    assert_eq!(payload["event_params"]["subagent_source"], json!(null));
    assert_eq!(payload["event_params"]["parent_thread_id"], json!(null));
    assert!(payload["event_params"].get("product_client_id").is_none());
}

#[tokio::test]
async fn rejected_turn_steer_uses_request_connection_metadata() {
    let mut reducer = AnalyticsReducer::default();
    let mut out = Vec::new();
    let payload = ingest_rejected_turn_steer(
        &mut reducer,
        &mut out,
        no_active_turn_steer_error(),
        Some(no_active_turn_steer_error_type()),
    )
    .await;

    assert_eq!(payload["event_type"], json!("codex_turn_steer_event"));
    assert_eq!(payload["event_params"]["thread_id"], json!("thread-2"));
    assert_eq!(payload["event_params"]["expected_turn_id"], json!("turn-2"));
    assert_eq!(payload["event_params"]["accepted_turn_id"], json!(null));
    assert_eq!(payload["event_params"]["num_input_images"], json!(1));
    assert_eq!(
        payload["event_params"]["app_server_client"]["product_client_id"],
        json!("codex-tui")
    );
    assert_eq!(
        payload["event_params"]["runtime"]["codex_rs_version"],
        json!("0.1.0")
    );
    assert_eq!(payload["event_params"]["thread_source"], json!("user"));
    assert_eq!(payload["event_params"]["subagent_source"], json!(null));
    assert_eq!(payload["event_params"]["parent_thread_id"], json!(null));
    assert_eq!(payload["event_params"]["result"], json!("rejected"));
    assert_eq!(
        payload["event_params"]["rejection_reason"],
        json!("no_active_turn")
    );
    assert!(
        payload["event_params"]["created_at"]
            .as_u64()
            .expect("created_at")
            > 0
    );
}

#[tokio::test]
async fn rejected_turn_steer_maps_active_turn_not_steerable_error_type() {
    let mut reducer = AnalyticsReducer::default();
    let mut out = Vec::new();
    let payload = ingest_rejected_turn_steer(
        &mut reducer,
        &mut out,
        non_steerable_review_error(),
        Some(non_steerable_review_error_type()),
    )
    .await;

    assert_eq!(
        payload["event_params"]["rejection_reason"],
        json!("non_steerable_review")
    );
}

#[tokio::test]
async fn rejected_turn_steer_maps_input_too_large_error_type() {
    let mut reducer = AnalyticsReducer::default();
    let mut out = Vec::new();
    let payload = ingest_rejected_turn_steer(
        &mut reducer,
        &mut out,
        input_too_large_steer_error(),
        Some(input_too_large_error_type()),
    )
    .await;

    assert_eq!(
        payload["event_params"]["rejection_reason"],
        json!("input_too_large")
    );
}

#[tokio::test]
async fn turn_steer_does_not_emit_without_pending_request() {
    let mut reducer = AnalyticsReducer::default();
    let mut out = Vec::new();

    reducer
        .ingest(
            AnalyticsFact::ErrorResponse {
                connection_id: 7,
                request_id: RequestId::Integer(4),
                error: no_active_turn_steer_error(),
                error_type: Some(no_active_turn_steer_error_type()),
            },
            &mut out,
        )
        .await;

    assert!(out.is_empty());
}
