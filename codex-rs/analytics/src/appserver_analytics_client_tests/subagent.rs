use super::common::*;
use super::*;
use pretty_assertions::assert_eq;

#[test]
fn subagent_thread_started_review_serializes_expected_shape() {
    let event = TrackEventRequest::ThreadInitialized(subagent_thread_started_event_request(
        SubAgentThreadStartedInput {
            thread_id: "thread-review".to_string(),
            parent_thread_id: None,
            product_client_id: "codex-tui".to_string(),
            client_name: "codex-tui".to_string(),
            client_version: "1.0.0".to_string(),
            model: "gpt-5".to_string(),
            ephemeral: false,
            subagent_source: SubAgentSource::Review,
            created_at: 123,
        },
    ));

    let payload = serde_json::to_value(&event).expect("serialize review subagent event");
    assert_eq!(payload["event_params"]["thread_source"], "subagent");
    assert_eq!(
        payload["event_params"]["app_server_client"]["product_client_id"],
        "codex-tui"
    );
    assert_eq!(
        payload["event_params"]["app_server_client"]["client_name"],
        "codex-tui"
    );
    assert_eq!(
        payload["event_params"]["app_server_client"]["client_version"],
        "1.0.0"
    );
    assert_eq!(
        payload["event_params"]["app_server_client"]["rpc_transport"],
        "in_process"
    );
    assert_eq!(payload["event_params"]["created_at"], 123);
    assert_eq!(payload["event_params"]["initialization_mode"], "new");
    assert_eq!(payload["event_params"]["subagent_source"], "review");
    assert_eq!(payload["event_params"]["parent_thread_id"], json!(null));
}

#[test]
fn subagent_thread_started_thread_spawn_serializes_parent_thread_id() {
    let parent_thread_id =
        codex_protocol::ThreadId::from_string("11111111-1111-1111-1111-111111111111")
            .expect("valid thread id");
    let event = TrackEventRequest::ThreadInitialized(subagent_thread_started_event_request(
        SubAgentThreadStartedInput {
            thread_id: "thread-spawn".to_string(),
            parent_thread_id: None,
            product_client_id: "codex-tui".to_string(),
            client_name: "codex-tui".to_string(),
            client_version: "1.0.0".to_string(),
            model: "gpt-5".to_string(),
            ephemeral: true,
            subagent_source: SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            },
            created_at: 124,
        },
    ));

    let payload = serde_json::to_value(&event).expect("serialize thread spawn subagent event");
    assert_eq!(payload["event_params"]["thread_source"], "subagent");
    assert_eq!(payload["event_params"]["subagent_source"], "thread_spawn");
    assert_eq!(
        payload["event_params"]["parent_thread_id"],
        "11111111-1111-1111-1111-111111111111"
    );
}

#[test]
fn subagent_thread_started_memory_consolidation_serializes_expected_shape() {
    let event = TrackEventRequest::ThreadInitialized(subagent_thread_started_event_request(
        SubAgentThreadStartedInput {
            thread_id: "thread-memory".to_string(),
            parent_thread_id: None,
            product_client_id: "codex-tui".to_string(),
            client_name: "codex-tui".to_string(),
            client_version: "1.0.0".to_string(),
            model: "gpt-5".to_string(),
            ephemeral: false,
            subagent_source: SubAgentSource::MemoryConsolidation,
            created_at: 125,
        },
    ));

    let payload =
        serde_json::to_value(&event).expect("serialize memory consolidation subagent event");
    assert_eq!(
        payload["event_params"]["subagent_source"],
        "memory_consolidation"
    );
    assert_eq!(payload["event_params"]["parent_thread_id"], json!(null));
}

#[test]
fn subagent_thread_started_other_serializes_expected_shape() {
    let event = TrackEventRequest::ThreadInitialized(subagent_thread_started_event_request(
        SubAgentThreadStartedInput {
            thread_id: "thread-guardian".to_string(),
            parent_thread_id: None,
            product_client_id: "codex-tui".to_string(),
            client_name: "codex-tui".to_string(),
            client_version: "1.0.0".to_string(),
            model: "gpt-5".to_string(),
            ephemeral: false,
            subagent_source: SubAgentSource::Other("guardian".to_string()),
            created_at: 126,
        },
    ));

    let payload = serde_json::to_value(&event).expect("serialize other subagent event");
    assert_eq!(payload["event_params"]["subagent_source"], "guardian");
    assert_eq!(payload["event_params"]["parent_thread_id"], json!(null));
}

#[test]
fn subagent_thread_started_other_serializes_explicit_parent_thread_id() {
    let event = TrackEventRequest::ThreadInitialized(subagent_thread_started_event_request(
        SubAgentThreadStartedInput {
            thread_id: "thread-guardian".to_string(),
            parent_thread_id: Some("parent-thread-guardian".to_string()),
            product_client_id: "codex-tui".to_string(),
            client_name: "codex-tui".to_string(),
            client_version: "1.0.0".to_string(),
            model: "gpt-5".to_string(),
            ephemeral: false,
            subagent_source: SubAgentSource::Other("guardian".to_string()),
            created_at: 126,
        },
    ));

    let payload = serde_json::to_value(&event).expect("serialize auto-review subagent event");
    assert_eq!(payload["event_params"]["subagent_source"], "guardian");
    assert_eq!(
        payload["event_params"]["parent_thread_id"],
        "parent-thread-guardian"
    );
}

#[tokio::test]
async fn subagent_thread_started_publishes_without_initialize() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();

    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::SubAgentThreadStarted(
                SubAgentThreadStartedInput {
                    thread_id: "thread-review".to_string(),
                    parent_thread_id: None,
                    product_client_id: "codex-tui".to_string(),
                    client_name: "codex-tui".to_string(),
                    client_version: "1.0.0".to_string(),
                    model: "gpt-5".to_string(),
                    ephemeral: false,
                    subagent_source: SubAgentSource::Review,
                    created_at: 127,
                },
            )),
            &mut events,
        )
        .await;

    let payload = serde_json::to_value(&events).expect("serialize events");
    assert_eq!(payload.as_array().expect("events array").len(), 1);
    assert_eq!(payload[0]["event_type"], "codex_thread_initialized");
    assert_eq!(
        payload[0]["event_params"]["app_server_client"]["product_client_id"],
        "codex-tui"
    );
    assert_eq!(payload[0]["event_params"]["thread_source"], "subagent");
    assert_eq!(payload[0]["event_params"]["subagent_source"], "review");
}

#[tokio::test]
async fn subagent_thread_started_inherits_parent_connection_for_new_thread() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();
    let parent_thread_id =
        codex_protocol::ThreadId::from_string("44444444-4444-4444-4444-444444444444")
            .expect("valid parent thread id");
    let parent_thread_id_string = parent_thread_id.to_string();

    reducer
        .ingest(
            AnalyticsFact::Initialize {
                connection_id: 7,
                params: InitializeParams {
                    client_info: ClientInfo {
                        name: "parent-client".to_string(),
                        title: None,
                        version: "1.0.0".to_string(),
                    },
                    capabilities: None,
                },
                product_client_id: "parent-client".to_string(),
                runtime: sample_runtime_metadata(),
                rpc_transport: AppServerRpcTransport::Stdio,
            },
            &mut events,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::ClientResponse {
                connection_id: 7,
                request_id: RequestId::Integer(1),
                response: Box::new(sample_thread_start_response(
                    &parent_thread_id_string,
                    /*ephemeral*/ false,
                    "gpt-5",
                )),
            },
            &mut events,
        )
        .await;

    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::SubAgentThreadStarted(
                SubAgentThreadStartedInput {
                    thread_id: "thread-review".to_string(),
                    parent_thread_id: None,
                    product_client_id: "parent-client".to_string(),
                    client_name: "parent-client".to_string(),
                    client_version: "1.0.0".to_string(),
                    model: "gpt-5".to_string(),
                    ephemeral: false,
                    subagent_source: SubAgentSource::ThreadSpawn {
                        parent_thread_id,
                        depth: 1,
                        agent_path: None,
                        agent_nickname: None,
                        agent_role: None,
                    },
                    created_at: 130,
                },
            )),
            &mut events,
        )
        .await;

    events.clear();
    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::Compaction(Box::new(
                CodexCompactionEvent {
                    thread_id: "thread-review".to_string(),
                    turn_id: "turn-compact".to_string(),
                    trigger: CompactionTrigger::Manual,
                    reason: CompactionReason::UserRequested,
                    implementation: CompactionImplementation::Responses,
                    phase: CompactionPhase::StandaloneTurn,
                    strategy: CompactionStrategy::Memento,
                    status: CompactionStatus::Completed,
                    error: None,
                    active_context_tokens_before: 131_000,
                    active_context_tokens_after: 64_000,
                    started_at: 100,
                    completed_at: 101,
                    duration_ms: Some(1200),
                },
            ))),
            &mut events,
        )
        .await;

    let payload = serde_json::to_value(&events).expect("serialize events");
    assert_eq!(
        payload[0]["event_params"]["app_server_client"]["product_client_id"],
        "parent-client"
    );
    assert_eq!(
        payload[0]["event_params"]["parent_thread_id"],
        "44444444-4444-4444-4444-444444444444"
    );
}

#[tokio::test]
async fn subagent_tool_items_inherit_parent_connection_metadata() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();

    ingest_review_prerequisites(&mut reducer, &mut events).await;
    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::SubAgentThreadStarted(
                SubAgentThreadStartedInput {
                    thread_id: "thread-subagent".to_string(),
                    parent_thread_id: Some("thread-1".to_string()),
                    product_client_id: "codex-tui".to_string(),
                    client_name: "codex-tui".to_string(),
                    client_version: "1.0.0".to_string(),
                    model: "gpt-5".to_string(),
                    ephemeral: false,
                    subagent_source: SubAgentSource::Review,
                    created_at: 128,
                },
            )),
            &mut events,
        )
        .await;
    events.clear();
    reducer
        .ingest(
            AnalyticsFact::Notification(Box::new(sample_turn_started_notification(
                "thread-subagent",
                "turn-subagent",
            ))),
            &mut events,
        )
        .await;

    reducer
        .ingest(
            AnalyticsFact::Notification(Box::new(ServerNotification::ItemStarted(
                ItemStartedNotification {
                    thread_id: "thread-subagent".to_string(),
                    turn_id: "turn-subagent".to_string(),
                    started_at_ms: 1_000,
                    item: sample_command_execution_item(
                        CommandExecutionStatus::InProgress,
                        /*exit_code*/ None,
                        /*duration_ms*/ None,
                    ),
                },
            ))),
            &mut events,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::Notification(Box::new(ServerNotification::ItemCompleted(
                ItemCompletedNotification {
                    thread_id: "thread-subagent".to_string(),
                    turn_id: "turn-subagent".to_string(),
                    completed_at_ms: 1_042,
                    item: sample_command_execution_item(
                        CommandExecutionStatus::Completed,
                        Some(0),
                        Some(42),
                    ),
                },
            ))),
            &mut events,
        )
        .await;

    let payload = serde_json::to_value(&events).expect("serialize events");
    assert_eq!(payload.as_array().expect("events array").len(), 1);
    assert_eq!(payload[0]["event_type"], "codex_command_execution_event");
    assert_eq!(payload[0]["event_params"]["thread_source"], "subagent");
    assert_eq!(payload[0]["event_params"]["subagent_source"], "review");
    assert_eq!(payload[0]["event_params"]["parent_thread_id"], "thread-1");
    assert_eq!(
        payload[0]["event_params"]["app_server_client"]["client_name"],
        "codex-tui"
    );
}
