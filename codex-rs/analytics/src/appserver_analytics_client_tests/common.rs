use super::*;
use pretty_assertions::assert_eq;

pub(super) fn sample_thread_with_metadata(
    thread_id: &str,
    ephemeral: bool,
    source: AppServerSessionSource,
    thread_source: Option<AppServerThreadSource>,
) -> Thread {
    Thread {
        id: thread_id.to_string(),
        session_id: format!("session-{thread_id}"),
        forked_from_id: None,
        preview: "first prompt".to_string(),
        ephemeral,
        model_provider: "openai".to_string(),
        created_at: 1,
        updated_at: 2,
        status: AppServerThreadStatus::Idle,
        path: None,
        cwd: test_path_buf("/tmp").abs(),
        cli_version: "0.0.0".to_string(),
        source,
        thread_source,
        agent_nickname: None,
        agent_role: None,
        git_info: None,
        name: None,
        turns: Vec::new(),
    }
}

pub(super) fn sample_thread_start_response(
    thread_id: &str,
    ephemeral: bool,
    model: &str,
) -> ClientResponsePayload {
    ClientResponsePayload::ThreadStart(ThreadStartResponse {
        thread: sample_thread_with_metadata(
            thread_id,
            ephemeral,
            AppServerSessionSource::Exec,
            Some(AppServerThreadSource::User),
        ),
        model: model.to_string(),
        model_provider: "openai".to_string(),
        service_tier: None,
        cwd: test_path_buf("/tmp").abs(),
        runtime_workspace_roots: Vec::new(),
        instruction_sources: Vec::new(),
        approval_policy: AppServerAskForApproval::OnFailure,
        approvals_reviewer: AppServerApprovalsReviewer::User,
        sandbox: AppServerSandboxPolicy::DangerFullAccess,
        active_permission_profile: None,
        reasoning_effort: None,
    })
}

pub(super) fn sample_app_server_client_metadata() -> CodexAppServerClientMetadata {
    CodexAppServerClientMetadata {
        product_client_id: DEFAULT_ORIGINATOR.to_string(),
        client_name: Some("codex-tui".to_string()),
        client_version: Some("1.0.0".to_string()),
        rpc_transport: AppServerRpcTransport::Stdio,
        experimental_api_enabled: Some(true),
    }
}

pub(super) fn sample_runtime_metadata() -> CodexRuntimeMetadata {
    CodexRuntimeMetadata {
        codex_rs_version: "0.1.0".to_string(),
        runtime_os: "macos".to_string(),
        runtime_os_version: "15.3.1".to_string(),
        runtime_arch: "aarch64".to_string(),
    }
}

pub(super) fn sample_thread_resume_response(
    thread_id: &str,
    ephemeral: bool,
    model: &str,
) -> ClientResponsePayload {
    sample_thread_resume_response_with_source(
        thread_id,
        ephemeral,
        model,
        AppServerSessionSource::Exec,
        Some(AppServerThreadSource::User),
    )
}

pub(super) fn sample_thread_resume_response_with_source(
    thread_id: &str,
    ephemeral: bool,
    model: &str,
    source: AppServerSessionSource,
    thread_source: Option<AppServerThreadSource>,
) -> ClientResponsePayload {
    ClientResponsePayload::ThreadResume(ThreadResumeResponse {
        thread: sample_thread_with_metadata(thread_id, ephemeral, source, thread_source),
        model: model.to_string(),
        model_provider: "openai".to_string(),
        service_tier: None,
        cwd: test_path_buf("/tmp").abs(),
        runtime_workspace_roots: Vec::new(),
        instruction_sources: Vec::new(),
        approval_policy: AppServerAskForApproval::OnFailure,
        approvals_reviewer: AppServerApprovalsReviewer::User,
        sandbox: AppServerSandboxPolicy::DangerFullAccess,
        active_permission_profile: None,
        reasoning_effort: None,
    })
}

pub(super) fn sample_turn_start_request(thread_id: &str, request_id: i64) -> ClientRequest {
    ClientRequest::TurnStart {
        request_id: RequestId::Integer(request_id),
        params: TurnStartParams {
            thread_id: thread_id.to_string(),
            input: vec![
                UserInput::Text {
                    text: "hello".to_string(),
                    text_elements: vec![],
                },
                UserInput::Image {
                    url: "https://example.com/a.png".to_string(),
                    detail: None,
                },
            ],
            ..Default::default()
        },
    }
}

pub(super) fn sample_turn_start_response(turn_id: &str) -> ClientResponsePayload {
    ClientResponsePayload::TurnStart(codex_app_server_protocol::TurnStartResponse {
        turn: Turn {
            id: turn_id.to_string(),
            items_view: codex_app_server_protocol::TurnItemsView::Full,
            items: vec![],
            status: AppServerTurnStatus::InProgress,
            error: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
        },
    })
}

pub(super) fn sample_turn_started_notification(
    thread_id: &str,
    turn_id: &str,
) -> ServerNotification {
    ServerNotification::TurnStarted(TurnStartedNotification {
        thread_id: thread_id.to_string(),
        turn: Turn {
            id: turn_id.to_string(),
            items_view: codex_app_server_protocol::TurnItemsView::Full,
            items: vec![],
            status: AppServerTurnStatus::InProgress,
            error: None,
            started_at: Some(455),
            completed_at: None,
            duration_ms: None,
        },
    })
}

pub(super) fn sample_turn_token_usage_fact(thread_id: &str, turn_id: &str) -> TurnTokenUsageFact {
    TurnTokenUsageFact {
        thread_id: thread_id.to_string(),
        turn_id: turn_id.to_string(),
        token_usage: TokenUsage {
            total_tokens: 321,
            input_tokens: 123,
            cached_input_tokens: 45,
            output_tokens: 140,
            reasoning_output_tokens: 13,
        },
    }
}

pub(super) fn sample_turn_completed_notification(
    thread_id: &str,
    turn_id: &str,
    status: AppServerTurnStatus,
    codex_error_info: Option<codex_app_server_protocol::CodexErrorInfo>,
) -> ServerNotification {
    ServerNotification::TurnCompleted(TurnCompletedNotification {
        thread_id: thread_id.to_string(),
        turn: Turn {
            id: turn_id.to_string(),
            items_view: codex_app_server_protocol::TurnItemsView::Full,
            items: vec![],
            status,
            error: codex_error_info.map(|codex_error_info| AppServerTurnError {
                message: "turn failed".to_string(),
                codex_error_info: Some(codex_error_info),
                additional_details: None,
            }),
            started_at: None,
            completed_at: Some(456),
            duration_ms: Some(1234),
        },
    })
}

pub(super) fn sample_turn_resolved_config(
    thread_id: &str,
    turn_id: &str,
) -> TurnResolvedConfigFact {
    TurnResolvedConfigFact {
        turn_id: turn_id.to_string(),
        thread_id: thread_id.to_string(),
        num_input_images: 1,
        submission_type: None,
        ephemeral: false,
        session_source: SessionSource::Exec,
        model: "gpt-5".to_string(),
        model_provider: "openai".to_string(),
        permission_profile: CorePermissionProfile::read_only(),
        permission_profile_cwd: PathBuf::from("/tmp"),
        reasoning_effort: None,
        reasoning_summary: None,
        service_tier: None,
        approval_policy: AskForApproval::OnRequest,
        approvals_reviewer: ApprovalsReviewer::AutoReview,
        sandbox_network_access: true,
        collaboration_mode: ModeKind::Plan,
        personality: None,
        is_first_turn: true,
    }
}

pub(super) fn sample_turn_steer_request(
    thread_id: &str,
    expected_turn_id: &str,
    request_id: i64,
) -> ClientRequest {
    ClientRequest::TurnSteer {
        request_id: RequestId::Integer(request_id),
        params: TurnSteerParams {
            thread_id: thread_id.to_string(),
            expected_turn_id: expected_turn_id.to_string(),
            input: vec![
                UserInput::Text {
                    text: "more".to_string(),
                    text_elements: vec![],
                },
                UserInput::LocalImage {
                    path: "/tmp/a.png".into(),
                    detail: None,
                },
            ],
            responsesapi_client_metadata: None,
        },
    }
}

pub(super) fn sample_turn_steer_response(turn_id: &str) -> ClientResponsePayload {
    ClientResponsePayload::TurnSteer(TurnSteerResponse {
        turn_id: turn_id.to_string(),
    })
}

pub(super) fn no_active_turn_steer_error() -> JSONRPCErrorError {
    JSONRPCErrorError {
        code: -32600,
        message: "no active turn to steer".to_string(),
        data: None,
    }
}

pub(super) fn no_active_turn_steer_error_type() -> AnalyticsJsonRpcError {
    AnalyticsJsonRpcError::TurnSteer(TurnSteerRequestError::NoActiveTurn)
}

pub(super) fn non_steerable_review_error() -> JSONRPCErrorError {
    JSONRPCErrorError {
        code: -32600,
        message: "cannot steer a review turn".to_string(),
        data: Some(
            serde_json::to_value(AppServerTurnError {
                message: "cannot steer a review turn".to_string(),
                codex_error_info: Some(CodexErrorInfo::ActiveTurnNotSteerable {
                    turn_kind: NonSteerableTurnKind::Review,
                }),
                additional_details: None,
            })
            .expect("serialize turn error"),
        ),
    }
}

pub(super) fn non_steerable_review_error_type() -> AnalyticsJsonRpcError {
    AnalyticsJsonRpcError::TurnSteer(TurnSteerRequestError::NonSteerableReview)
}

pub(super) fn input_too_large_steer_error() -> JSONRPCErrorError {
    JSONRPCErrorError {
        code: -32602,
        message: "Input exceeds the maximum length of 1048576 characters.".to_string(),
        data: Some(json!({
            "input_error_code": "input_too_large",
            "actual_chars": 1048577,
            "max_chars": 1048576,
        })),
    }
}

pub(super) fn input_too_large_error_type() -> AnalyticsJsonRpcError {
    AnalyticsJsonRpcError::Input(InputError::TooLarge)
}

pub(super) async fn ingest_rejected_turn_steer(
    reducer: &mut AnalyticsReducer,
    out: &mut Vec<TrackEventRequest>,
    error: JSONRPCErrorError,
    error_type: Option<AnalyticsJsonRpcError>,
) -> serde_json::Value {
    ingest_turn_prerequisites(
        reducer, out, /*include_initialize*/ true, /*include_resolved_config*/ false,
        /*include_started*/ false, /*include_token_usage*/ false,
    )
    .await;
    reducer
        .ingest(
            AnalyticsFact::Initialize {
                connection_id: 8,
                params: InitializeParams {
                    client_info: ClientInfo {
                        name: "codex-web".to_string(),
                        title: None,
                        version: "1.0.0".to_string(),
                    },
                    capabilities: None,
                },
                product_client_id: "codex-web".to_string(),
                runtime: sample_runtime_metadata(),
                rpc_transport: AppServerRpcTransport::Stdio,
            },
            out,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::ClientResponse {
                connection_id: 8,
                request_id: RequestId::Integer(6),
                response: Box::new(sample_thread_resume_response(
                    "thread-2", /*ephemeral*/ false, "gpt-5",
                )),
            },
            out,
        )
        .await;
    out.clear();
    reducer
        .ingest(
            AnalyticsFact::ClientRequest {
                connection_id: 7,
                request_id: RequestId::Integer(4),
                request: Box::new(sample_turn_steer_request(
                    "thread-2", "turn-2", /*request_id*/ 4,
                )),
            },
            out,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::ErrorResponse {
                connection_id: 7,
                request_id: RequestId::Integer(4),
                error,
                error_type,
            },
            out,
        )
        .await;

    assert_eq!(out.len(), 1);
    serde_json::to_value(&out[0]).expect("serialize turn steer event")
}

pub(super) async fn ingest_initialize(
    reducer: &mut AnalyticsReducer,
    out: &mut Vec<TrackEventRequest>,
) {
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
                    capabilities: None,
                },
                product_client_id: "codex-tui".to_string(),
                runtime: sample_runtime_metadata(),
                rpc_transport: AppServerRpcTransport::Stdio,
            },
            out,
        )
        .await;
}

pub(super) async fn ingest_turn_prerequisites(
    reducer: &mut AnalyticsReducer,
    out: &mut Vec<TrackEventRequest>,
    include_initialize: bool,
    include_resolved_config: bool,
    include_started: bool,
    include_token_usage: bool,
) {
    if include_initialize {
        ingest_initialize(reducer, out).await;
        reducer
            .ingest(
                AnalyticsFact::ClientResponse {
                    connection_id: 7,
                    request_id: RequestId::Integer(1),
                    response: Box::new(sample_thread_start_response(
                        "thread-2", /*ephemeral*/ false, "gpt-5",
                    )),
                },
                out,
            )
            .await;
        out.clear();
    }

    reducer
        .ingest(
            AnalyticsFact::ClientRequest {
                connection_id: 7,
                request_id: RequestId::Integer(3),
                request: Box::new(sample_turn_start_request("thread-2", /*request_id*/ 3)),
            },
            out,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::ClientResponse {
                connection_id: 7,
                request_id: RequestId::Integer(3),
                response: Box::new(sample_turn_start_response("turn-2")),
            },
            out,
        )
        .await;

    if include_resolved_config {
        reducer
            .ingest(
                AnalyticsFact::Custom(CustomAnalyticsFact::TurnResolvedConfig(Box::new(
                    sample_turn_resolved_config("thread-2", "turn-2"),
                ))),
                out,
            )
            .await;
    }

    if include_started {
        reducer
            .ingest(
                AnalyticsFact::Notification(Box::new(sample_turn_started_notification(
                    "thread-2", "turn-2",
                ))),
                out,
            )
            .await;
    }

    if include_token_usage {
        reducer
            .ingest(
                AnalyticsFact::Custom(CustomAnalyticsFact::TurnTokenUsage(Box::new(
                    sample_turn_token_usage_fact("thread-2", "turn-2"),
                ))),
                out,
            )
            .await;
    }
}

pub(super) async fn ingest_review_prerequisites(
    reducer: &mut AnalyticsReducer,
    events: &mut Vec<TrackEventRequest>,
) {
    reducer
        .ingest(sample_initialize_fact(/*connection_id*/ 7), events)
        .await;
    reducer
        .ingest(
            AnalyticsFact::ClientResponse {
                connection_id: 7,
                request_id: RequestId::Integer(1),
                response: Box::new(sample_thread_start_response(
                    "thread-1", /*ephemeral*/ false, "gpt-5",
                )),
            },
            events,
        )
        .await;
    events.clear();
}

pub(super) async fn ingest_completed_command_execution_item(
    reducer: &mut AnalyticsReducer,
    events: &mut Vec<TrackEventRequest>,
    thread_id: &str,
    item_id: &str,
) {
    reducer
        .ingest(
            AnalyticsFact::Notification(Box::new(sample_turn_started_notification(
                thread_id, "turn-1",
            ))),
            events,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::Notification(Box::new(ServerNotification::ItemStarted(
                ItemStartedNotification {
                    thread_id: thread_id.to_string(),
                    turn_id: "turn-1".to_string(),
                    started_at_ms: 1_000,
                    item: sample_command_execution_item_with_id(
                        item_id,
                        CommandExecutionStatus::InProgress,
                        /*exit_code*/ None,
                        /*duration_ms*/ None,
                    ),
                },
            ))),
            events,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::Notification(Box::new(ServerNotification::ItemCompleted(
                ItemCompletedNotification {
                    thread_id: thread_id.to_string(),
                    turn_id: "turn-1".to_string(),
                    completed_at_ms: 1_042,
                    item: sample_command_execution_item_with_id(
                        item_id,
                        CommandExecutionStatus::Completed,
                        Some(0),
                        Some(42),
                    ),
                },
            ))),
            events,
        )
        .await;
}

pub(super) fn sample_initialize_fact(connection_id: u64) -> AnalyticsFact {
    AnalyticsFact::Initialize {
        connection_id,
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
    }
}

pub(super) fn sample_command_execution_item(
    status: CommandExecutionStatus,
    exit_code: Option<i32>,
    duration_ms: Option<i64>,
) -> ThreadItem {
    sample_command_execution_item_with_id("item-1", status, exit_code, duration_ms)
}

pub(super) fn sample_command_execution_item_with_id(
    id: &str,
    status: CommandExecutionStatus,
    exit_code: Option<i32>,
    duration_ms: Option<i64>,
) -> ThreadItem {
    ThreadItem::CommandExecution {
        id: id.to_string(),
        command: "echo hi".to_string(),
        cwd: test_path_buf("/tmp").abs(),
        process_id: Some("pid-1".to_string()),
        source: CommandExecutionSource::Agent,
        status,
        command_actions: Vec::new(),
        aggregated_output: None,
        exit_code,
        duration_ms,
    }
}

pub(super) fn sample_command_execution_item_with_actions(
    status: CommandExecutionStatus,
    exit_code: Option<i32>,
    duration_ms: Option<i64>,
    command_actions: Vec<CommandAction>,
) -> ThreadItem {
    let mut item = sample_command_execution_item(status, exit_code, duration_ms);
    let ThreadItem::CommandExecution {
        command_actions: item_command_actions,
        ..
    } = &mut item
    else {
        unreachable!("sample command execution item should be CommandExecution");
    };
    *item_command_actions = command_actions;
    item
}

pub(super) fn sample_command_approval_request(
    request_id: i64,
    approval_id: Option<&str>,
) -> ServerRequest {
    ServerRequest::CommandExecutionRequestApproval {
        request_id: RequestId::Integer(request_id),
        params: CommandExecutionRequestApprovalParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: "item-1".to_string(),
            started_at_ms: 1_000,
            approval_id: approval_id.map(str::to_string),
            reason: None,
            network_approval_context: None,
            command: Some("echo hi".to_string()),
            cwd: None,
            command_actions: None,
            additional_permissions: None,
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            available_decisions: None,
        },
    }
}

pub(super) fn sample_command_approval_response(
    request_id: i64,
    decision: CommandExecutionApprovalDecision,
) -> ServerResponse {
    ServerResponse::CommandExecutionRequestApproval {
        request_id: RequestId::Integer(request_id),
        response: CommandExecutionRequestApprovalResponse { decision },
    }
}

pub(super) fn sample_permissions_approval_request(request_id: i64) -> ServerRequest {
    ServerRequest::PermissionsRequestApproval {
        request_id: RequestId::Integer(request_id),
        params: PermissionsRequestApprovalParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: "permissions-1".to_string(),
            started_at_ms: 1_000,
            cwd: test_path_buf("/tmp").abs(),
            reason: Some("need network".to_string()),
            permissions: RequestPermissionProfile {
                network: Some(codex_app_server_protocol::AdditionalNetworkPermissions {
                    enabled: Some(true),
                }),
                file_system: None,
            },
        },
    }
}

pub(super) fn sample_effective_permissions_approval_response(
    permissions: CoreRequestPermissionProfile,
    scope: CorePermissionGrantScope,
) -> CoreRequestPermissionsResponse {
    CoreRequestPermissionsResponse {
        permissions,
        scope,
        strict_auto_review: false,
    }
}

pub(super) fn sample_guardian_review_completed(
    review_id: &str,
    target_item_id: Option<&str>,
    status: GuardianApprovalReviewStatus,
) -> ServerNotification {
    ServerNotification::ItemGuardianApprovalReviewCompleted(
        ItemGuardianApprovalReviewCompletedNotification {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            started_at_ms: 1_000,
            completed_at_ms: 1_042,
            review_id: review_id.to_string(),
            target_item_id: target_item_id.map(str::to_string),
            decision_source: codex_app_server_protocol::AutoReviewDecisionSource::Agent,
            review: GuardianApprovalReview {
                status,
                risk_level: None,
                user_authorization: None,
                rationale: None,
            },
            action: GuardianApprovalReviewAction::Command {
                source: AppServerGuardianCommandSource::Shell,
                command: "echo hi".to_string(),
                cwd: test_path_buf("/tmp").abs(),
            },
        },
    )
}
pub(super) fn sample_plugin_metadata() -> PluginTelemetryMetadata {
    PluginTelemetryMetadata {
        plugin_id: PluginId::parse("sample@test").expect("valid plugin id"),
        remote_plugin_id: None,
        capability_summary: Some(PluginCapabilitySummary {
            config_name: "sample@test".to_string(),
            display_name: "sample".to_string(),
            description: None,
            has_skills: true,
            mcp_server_names: vec!["mcp-1".to_string(), "mcp-2".to_string()],
            app_connector_ids: vec![
                AppConnectorId("calendar".to_string()),
                AppConnectorId("drive".to_string()),
            ],
        }),
    }
}
