#![allow(clippy::expect_used, clippy::unwrap_used, dead_code, unused_imports)]
pub(crate) use codex_api::WS_REQUEST_HEADER_TRACEPARENT_CLIENT_METADATA_KEY;
pub(crate) use codex_api::WS_REQUEST_HEADER_TRACESTATE_CLIENT_METADATA_KEY;
pub(crate) use codex_core::ModelClient;
pub(crate) use codex_core::ModelClientSession;
pub(crate) use codex_core::Prompt;
pub(crate) use codex_core::ResponseEvent;
pub(crate) use codex_core::X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER;
pub(crate) use codex_core_test_runtime::load_default_config_for_test;
pub(crate) use codex_core_test_runtime::responses::WebSocketConnectionConfig;
pub(crate) use codex_core_test_runtime::responses::WebSocketTestServer;
pub(crate) use codex_core_test_runtime::responses::ev_assistant_message;
pub(crate) use codex_core_test_runtime::responses::ev_completed;
pub(crate) use codex_core_test_runtime::responses::ev_response_created;
pub(crate) use codex_core_test_runtime::responses::start_websocket_server;
pub(crate) use codex_core_test_runtime::responses::start_websocket_server_with_headers;
pub(crate) use codex_core_test_runtime::skip_if_no_network;
pub(crate) use codex_core_test_runtime::test_codex::test_codex;
pub(crate) use codex_core_test_runtime::tracing::install_test_tracing;
pub(crate) use codex_core_test_runtime::wait_for_event;
pub(crate) use codex_features::Feature;
pub(crate) use codex_login::CodexAuth;
pub(crate) use codex_model_provider_info::ModelProviderInfo;
pub(crate) use codex_model_provider_info::WireApi;
pub(crate) use codex_otel::MetricsClient;
pub(crate) use codex_otel::MetricsConfig;
pub(crate) use codex_otel::SessionTelemetry;
pub(crate) use codex_otel::TelemetryAuthMode;
pub(crate) use codex_otel::current_span_w3c_trace_context;
pub(crate) use codex_protocol::SessionId;
pub(crate) use codex_protocol::ThreadId;
pub(crate) use codex_protocol::account::PlanType;
pub(crate) use codex_protocol::config_types::ReasoningSummary;
pub(crate) use codex_protocol::config_types::ServiceTier;
pub(crate) use codex_protocol::models::BaseInstructions;
pub(crate) use codex_protocol::models::ContentItem;
pub(crate) use codex_protocol::models::ResponseItem;
pub(crate) use codex_protocol::openai_models::ModelInfo;
pub(crate) use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
pub(crate) use codex_protocol::protocol::EventMsg;
pub(crate) use codex_protocol::protocol::Op;
pub(crate) use codex_protocol::protocol::SessionSource;
pub(crate) use codex_protocol::protocol::W3cTraceContext;
pub(crate) use codex_protocol::user_input::UserInput;
pub(crate) use futures::StreamExt;
pub(crate) use opentelemetry_sdk::metrics::InMemoryMetricExporter;
pub(crate) use pretty_assertions::assert_eq;
pub(crate) use serde_json::json;
pub(crate) use std::sync::Arc;
pub(crate) use std::time::Duration;
pub(crate) use tempfile::TempDir;
pub(crate) use tracing::Instrument;
pub(crate) use tracing_test::traced_test;

pub(crate) const MODEL: &str = "gpt-5.3-codex";
pub(crate) const OPENAI_BETA_HEADER: &str = "OpenAI-Beta";
pub(crate) const USER_AGENT_HEADER: &str = "user-agent";
pub(crate) const WS_V2_BETA_HEADER_VALUE: &str = "responses_websockets=2026-02-06";
pub(crate) const X_CLIENT_REQUEST_ID_HEADER: &str = "x-client-request-id";
pub(crate) const TEST_INSTALLATION_ID: &str = "11111111-1111-4111-8111-111111111111";
pub(crate) const X_CODEX_WS_STREAM_REQUEST_START_MS_CLIENT_METADATA_KEY: &str =
    "x-codex-ws-stream-request-start-ms";

pub(crate) fn assert_request_trace_matches(
    body: &serde_json::Value,
    expected_trace: &W3cTraceContext,
) {
    let client_metadata = body["client_metadata"]
        .as_object()
        .expect("missing client_metadata payload");
    let actual_traceparent = client_metadata
        .get(WS_REQUEST_HEADER_TRACEPARENT_CLIENT_METADATA_KEY)
        .and_then(serde_json::Value::as_str)
        .expect("missing traceparent");
    let expected_traceparent = expected_trace
        .traceparent
        .as_deref()
        .expect("missing expected traceparent");

    assert_eq!(actual_traceparent, expected_traceparent);
    assert_eq!(
        client_metadata
            .get(WS_REQUEST_HEADER_TRACESTATE_CLIENT_METADATA_KEY)
            .and_then(serde_json::Value::as_str),
        expected_trace.tracestate.as_deref()
    );
    assert!(
        body.get("trace").is_none(),
        "top-level trace should not be sent"
    );
}

pub(crate) struct WebsocketTestHarness {
    pub(crate) _codex_home: TempDir,
    pub(crate) client: ModelClient,
    pub(crate) session_id: SessionId,
    pub(crate) thread_id: ThreadId,
    pub(crate) model_info: ModelInfo,
    pub(crate) effort: Option<ReasoningEffortConfig>,
    pub(crate) summary: ReasoningSummary,
    pub(crate) session_telemetry: SessionTelemetry,
}

pub(crate) fn message_item(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".into(),
        content: vec![ContentItem::InputText { text: text.into() }],
        phase: None,
    }
}

pub(crate) fn assistant_message_item(id: &str, text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: Some(id.to_string()),
        role: "assistant".into(),
        content: vec![ContentItem::OutputText { text: text.into() }],
        phase: None,
    }
}

pub(crate) fn prompt_with_input(input: Vec<ResponseItem>) -> Prompt {
    let mut prompt = Prompt::default();
    prompt.input = input;
    prompt
}

pub(crate) fn prompt_with_input_and_instructions(
    input: Vec<ResponseItem>,
    instructions: &str,
) -> Prompt {
    let mut prompt = prompt_with_input(input);
    prompt.base_instructions = BaseInstructions {
        text: instructions.to_string(),
    };
    prompt
}

pub(crate) fn websocket_provider(server: &WebSocketTestServer) -> ModelProviderInfo {
    websocket_provider_with_connect_timeout(server, /*websocket_connect_timeout_ms*/ None)
}

pub(crate) fn websocket_provider_with_connect_timeout(
    server: &WebSocketTestServer,
    websocket_connect_timeout_ms: Option<u64>,
) -> ModelProviderInfo {
    ModelProviderInfo {
        name: "mock-ws".into(),
        base_url: Some(format!("{}/v1", server.uri())),
        env_key: None,
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        aws: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: Some(0),
        stream_max_retries: Some(0),
        stream_idle_timeout_ms: Some(5_000),
        websocket_connect_timeout_ms,
        requires_openai_auth: false,
        supports_websockets: true,
    }
}

pub(crate) async fn websocket_harness(server: &WebSocketTestServer) -> WebsocketTestHarness {
    websocket_harness_with_runtime_metrics(server, /*runtime_metrics_enabled*/ false).await
}

pub(crate) async fn websocket_harness_with_runtime_metrics(
    server: &WebSocketTestServer,
    runtime_metrics_enabled: bool,
) -> WebsocketTestHarness {
    websocket_harness_with_options(server, runtime_metrics_enabled).await
}

pub(crate) async fn websocket_harness_with_v2(
    server: &WebSocketTestServer,
    runtime_metrics_enabled: bool,
) -> WebsocketTestHarness {
    websocket_harness_with_options(server, runtime_metrics_enabled).await
}

pub(crate) async fn websocket_harness_with_options(
    server: &WebSocketTestServer,
    runtime_metrics_enabled: bool,
) -> WebsocketTestHarness {
    websocket_harness_with_provider_options(websocket_provider(server), runtime_metrics_enabled)
        .await
}

pub(crate) async fn websocket_harness_with_provider_options(
    provider: ModelProviderInfo,
    runtime_metrics_enabled: bool,
) -> WebsocketTestHarness {
    let codex_home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&codex_home).await;
    config.model = Some(MODEL.to_string());
    if runtime_metrics_enabled {
        config
            .features
            .enable(Feature::RuntimeMetrics)
            .expect("test config should allow feature update");
    }
    let config = Arc::new(config);
    let model_info = codex_core::test_support::construct_model_info_offline(MODEL, &config);
    let thread_id = ThreadId::new();
    let session_id = SessionId::new();
    let auth_manager =
        codex_core::test_support::auth_manager_from_auth(CodexAuth::from_api_key("Test API Key"));
    let exporter = InMemoryMetricExporter::default();
    let metrics = MetricsClient::new(
        MetricsConfig::in_memory("test", "codex-core", env!("CARGO_PKG_VERSION"), exporter)
            .with_runtime_reader(),
    )
    .expect("in-memory metrics client");
    let session_telemetry = SessionTelemetry::new(
        thread_id,
        MODEL,
        model_info.slug.as_str(),
        /*account_id*/ None,
        Some("test@test.com".to_string()),
        auth_manager.auth_mode().map(TelemetryAuthMode::from),
        "test_originator".to_string(),
        /*log_user_prompts*/ false,
        "test".to_string(),
        SessionSource::Exec,
    )
    .with_metrics(metrics);
    let effort = None;
    let summary = ReasoningSummary::Auto;
    let client = ModelClient::new(
        /*auth_manager*/ None,
        session_id,
        thread_id,
        /*installation_id*/ TEST_INSTALLATION_ID.to_string(),
        provider.clone(),
        SessionSource::Exec,
        config.model_verbosity,
        /*enable_request_compression*/ false,
        runtime_metrics_enabled,
        /*beta_features_header*/ None,
        /*attestation_provider*/ None,
    );

    WebsocketTestHarness {
        _codex_home: codex_home,
        client,
        session_id,
        thread_id,
        model_info,
        effort,
        summary,
        session_telemetry,
    }
}

pub(crate) async fn stream_until_complete(
    client_session: &mut ModelClientSession,
    harness: &WebsocketTestHarness,
    prompt: &Prompt,
) {
    stream_until_complete_with_service_tier(
        client_session,
        harness,
        prompt,
        /*service_tier*/ None,
    )
    .await;
}

pub(crate) async fn stream_until_complete_with_service_tier(
    client_session: &mut ModelClientSession,
    harness: &WebsocketTestHarness,
    prompt: &Prompt,
    service_tier: Option<ServiceTier>,
) {
    stream_until_complete_with_turn_metadata(
        client_session,
        harness,
        prompt,
        service_tier,
        /*turn_metadata_header*/ None,
    )
    .await;
}

pub(crate) async fn stream_until_complete_with_turn_metadata(
    client_session: &mut ModelClientSession,
    harness: &WebsocketTestHarness,
    prompt: &Prompt,
    service_tier: Option<ServiceTier>,
    turn_metadata_header: Option<&str>,
) {
    stream_until_complete_with_request_metadata(
        client_session,
        harness,
        prompt,
        service_tier,
        turn_metadata_header,
    )
    .await;
}

pub(crate) async fn stream_until_complete_with_request_metadata(
    client_session: &mut ModelClientSession,
    harness: &WebsocketTestHarness,
    prompt: &Prompt,
    service_tier: Option<ServiceTier>,
    turn_metadata_header: Option<&str>,
) {
    let mut stream = client_session
        .stream(
            prompt,
            &harness.model_info,
            &harness.session_telemetry,
            harness.effort,
            harness.summary,
            service_tier.map(|service_tier| service_tier.request_value().to_string()),
            turn_metadata_header,
            &codex_rollout_trace::InferenceTraceContext::disabled(),
        )
        .await
        .expect("websocket stream failed");

    while let Some(event) = stream.next().await {
        if matches!(event, Ok(ResponseEvent::Completed { .. })) {
            break;
        }
    }
}
