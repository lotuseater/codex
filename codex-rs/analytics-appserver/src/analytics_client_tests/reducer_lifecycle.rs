use super::*;
use pretty_assertions::assert_eq;
use super::common::*;

#[tokio::test]
async fn initialize_caches_client_and_thread_lifecycle_publishes_once_initialized() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();

    reducer
        .ingest(
            AnalyticsFact::ClientResponse {
                connection_id: 7,
                request_id: RequestId::Integer(1),
                response: Box::new(sample_thread_start_response(
                    "thread-no-client",
                    /*ephemeral*/ false,
                    "gpt-5",
                )),
            },
            &mut events,
        )
        .await;
    assert!(events.is_empty(), "thread events should require initialize");

    reducer
        .ingest(
            AnalyticsFact::Initialize {
                connection_id: 7,
                params: InitializeParams {
                    client_info: ClientInfo {
                        name: "codex-tui".to_string(),
                        title: None,
                        version: "1.0.0".to_string(),
                    },
                    capabilities: Some(InitializeCapabilities {
                        experimental_api: false,
                        request_attestation: false,
                        opt_out_notification_methods: None,
                    }),
                },
                product_client_id: DEFAULT_ORIGINATOR.to_string(),
                runtime: CodexRuntimeMetadata {
                    codex_rs_version: "0.99.0".to_string(),
                    runtime_os: "linux".to_string(),
                    runtime_os_version: "24.04".to_string(),
                    runtime_arch: "x86_64".to_string(),
                },
                rpc_transport: AppServerRpcTransport::Websocket,
            },
            &mut events,
        )
        .await;
    assert!(events.is_empty(), "initialize should not publish by itself");

    reducer
        .ingest(
            AnalyticsFact::ClientResponse {
                connection_id: 7,
                request_id: RequestId::Integer(2),
                response: Box::new(sample_thread_resume_response(
                    "thread-1", /*ephemeral*/ true, "gpt-5",
                )),
            },
            &mut events,
        )
        .await;

    let payload = serde_json::to_value(&events).expect("serialize events");
    assert_eq!(payload.as_array().expect("events array").len(), 1);
    assert_eq!(payload[0]["event_type"], "codex_thread_initialized");
    assert_eq!(
        payload[0]["event_params"]["app_server_client"]["product_client_id"],
        DEFAULT_ORIGINATOR
    );
    assert_eq!(
        payload[0]["event_params"]["app_server_client"]["client_name"],
        "codex-tui"
    );
    assert_eq!(
        payload[0]["event_params"]["app_server_client"]["client_version"],
        "1.0.0"
    );
    assert_eq!(
        payload[0]["event_params"]["app_server_client"]["rpc_transport"],
        "websocket"
    );
    assert_eq!(
        payload[0]["event_params"]["app_server_client"]["experimental_api_enabled"],
        false
    );
    assert_eq!(
        payload[0]["event_params"]["runtime"]["codex_rs_version"],
        "0.99.0"
    );
    assert_eq!(payload[0]["event_params"]["runtime"]["runtime_os"], "linux");
    assert_eq!(
        payload[0]["event_params"]["runtime"]["runtime_os_version"],
        "24.04"
    );
    assert_eq!(
        payload[0]["event_params"]["runtime"]["runtime_arch"],
        "x86_64"
    );
}

#[tokio::test]
async fn unrelated_client_requests_are_ignored_by_reducer() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();

    reducer
        .ingest(
            AnalyticsFact::ClientRequest {
                connection_id: 7,
                request_id: RequestId::Integer(3),
                request: Box::new(ClientRequest::ThreadArchive {
                    request_id: RequestId::Integer(3),
                    params: ThreadArchiveParams {
                        thread_id: "thread-2".to_string(),
                    },
                }),
            },
            &mut events,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::ClientResponse {
                connection_id: 7,
                request_id: RequestId::Integer(3),
                response: Box::new(sample_turn_start_response("turn-2")),
            },
            &mut events,
        )
        .await;

    assert!(
        events.is_empty(),
        "unrelated requests must not create pending turn state"
    );
}

#[tokio::test]
async fn unrelated_client_responses_are_ignored_by_reducer() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();

    ingest_initialize(&mut reducer, &mut events).await;
    reducer
        .ingest(
            AnalyticsFact::ClientResponse {
                connection_id: 7,
                request_id: RequestId::Integer(9),
                response: Box::new(ClientResponsePayload::ThreadArchive(
                    ThreadArchiveResponse {},
                )),
            },
            &mut events,
        )
        .await;

    assert!(events.is_empty());
}

#[tokio::test]
async fn compaction_event_ingests_custom_fact() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();
    let parent_thread_id =
        codex_protocol::ThreadId::from_string("22222222-2222-2222-2222-222222222222")
            .expect("valid parent thread id");

    reducer
        .ingest(
            AnalyticsFact::Initialize {
                connection_id: 7,
                params: InitializeParams {
                    client_info: ClientInfo {
                        name: "codex-tui".to_string(),
                        title: None,
                        version: "1.0.0".to_string(),
                    },
                    capabilities: Some(InitializeCapabilities {
                        experimental_api: false,
                        request_attestation: false,
                        opt_out_notification_methods: None,
                    }),
                },
                product_client_id: DEFAULT_ORIGINATOR.to_string(),
                runtime: sample_runtime_metadata(),
                rpc_transport: AppServerRpcTransport::Websocket,
            },
            &mut events,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::ClientResponse {
                connection_id: 7,
                request_id: RequestId::Integer(2),
                response: Box::new(sample_thread_resume_response_with_source(
                    "thread-1",
                    /*ephemeral*/ false,
                    "gpt-5",
                    AppServerSessionSource::SubAgent(codex_app_server_protocol::SubAgentSource::ThreadSpawn {
                        parent_thread_id: parent_thread_id.to_string(),
                        depth: 1,
                        agent_path: None,
                        agent_nickname: None,
                        agent_role: None,
                    }),
                    Some(AppServerThreadSource::Subagent),
                )),
            },
            &mut events,
        )
        .await;
    events.clear();

    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::Compaction(Box::new(
                CodexCompactionEvent {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-compact".to_string(),
                    trigger: CompactionTrigger::Manual,
                    reason: CompactionReason::UserRequested,
                    implementation: CompactionImplementation::Responses,
                    phase: CompactionPhase::StandaloneTurn,
                    strategy: CompactionStrategy::Memento,
                    status: CompactionStatus::Failed,
                    error: Some("context limit exceeded".to_string()),
                    active_context_tokens_before: 131_000,
                    active_context_tokens_after: 131_000,
                    started_at: 100,
                    completed_at: 101,
                    duration_ms: Some(1200),
                },
            ))),
            &mut events,
        )
        .await;

    let payload = serde_json::to_value(&events).expect("serialize events");
    assert_eq!(payload.as_array().expect("events array").len(), 1);
    assert_eq!(payload[0]["event_type"], "codex_compaction_event");
    assert_eq!(payload[0]["event_params"]["thread_id"], "thread-1");
    assert_eq!(payload[0]["event_params"]["turn_id"], "turn-compact");
    assert_eq!(
        payload[0]["event_params"]["app_server_client"]["product_client_id"],
        DEFAULT_ORIGINATOR
    );
    assert_eq!(
        payload[0]["event_params"]["app_server_client"]["client_name"],
        "codex-tui"
    );
    assert_eq!(
        payload[0]["event_params"]["app_server_client"]["rpc_transport"],
        "websocket"
    );
    assert_eq!(
        payload[0]["event_params"]["runtime"]["codex_rs_version"],
        "0.1.0"
    );
    assert_eq!(payload[0]["event_params"]["thread_source"], "subagent");
    assert_eq!(
        payload[0]["event_params"]["subagent_source"],
        "thread_spawn"
    );
    assert_eq!(
        payload[0]["event_params"]["parent_thread_id"],
        "22222222-2222-2222-2222-222222222222"
    );
    assert_eq!(payload[0]["event_params"]["trigger"], "manual");
    assert_eq!(payload[0]["event_params"]["reason"], "user_requested");
    assert_eq!(payload[0]["event_params"]["implementation"], "responses");
    assert_eq!(payload[0]["event_params"]["phase"], "standalone_turn");
    assert_eq!(payload[0]["event_params"]["strategy"], "memento");
    assert_eq!(payload[0]["event_params"]["status"], "failed");
}
