use super::*;
use pretty_assertions::assert_eq;
use super::common::*;

#[test]
fn app_mentioned_event_serializes_expected_shape() {
    let tracking = TrackEventsContext {
        model_slug: "gpt-5".to_string(),
        thread_id: "thread-1".to_string(),
        turn_id: "turn-1".to_string(),
    };
    let event = TrackEventRequest::AppMentioned(CodexAppMentionedEventRequest {
        event_type: "codex_app_mentioned",
        event_params: codex_app_metadata(
            &tracking,
            AppInvocation {
                connector_id: Some("calendar".to_string()),
                app_name: Some("Calendar".to_string()),
                invocation_type: Some(InvocationType::Explicit),
            },
        ),
    });

    let payload = serde_json::to_value(&event).expect("serialize app mentioned event");

    assert_eq!(
        payload,
        json!({
            "event_type": "codex_app_mentioned",
            "event_params": {
                "connector_id": "calendar",
                "thread_id": "thread-1",
                "turn_id": "turn-1",
                "app_name": "Calendar",
                "product_client_id": originator().value,
                "invoke_type": "explicit",
                "model_slug": "gpt-5"
            }
        })
    );
}

#[test]
fn app_used_event_serializes_expected_shape() {
    let tracking = TrackEventsContext {
        model_slug: "gpt-5".to_string(),
        thread_id: "thread-2".to_string(),
        turn_id: "turn-2".to_string(),
    };
    let event = TrackEventRequest::AppUsed(CodexAppUsedEventRequest {
        event_type: "codex_app_used",
        event_params: codex_app_metadata(
            &tracking,
            AppInvocation {
                connector_id: Some("drive".to_string()),
                app_name: Some("Google Drive".to_string()),
                invocation_type: Some(InvocationType::Implicit),
            },
        ),
    });

    let payload = serde_json::to_value(&event).expect("serialize app used event");

    assert_eq!(
        payload,
        json!({
            "event_type": "codex_app_used",
            "event_params": {
                "connector_id": "drive",
                "thread_id": "thread-2",
                "turn_id": "turn-2",
                "app_name": "Google Drive",
                "product_client_id": originator().value,
                "invoke_type": "implicit",
                "model_slug": "gpt-5"
            }
        })
    );
}

#[test]
fn accepted_line_fingerprints_event_serializes_expected_shape() {
    let event = TrackEventRequest::AcceptedLineFingerprints(Box::new(
        CodexAcceptedLineFingerprintsEventRequest {
            event_type: "codex_accepted_line_fingerprints",
            event_params: CodexAcceptedLineFingerprintsEventParams {
                event_type: "codex.accepted_line_fingerprints",
                turn_id: "turn-1".to_string(),
                thread_id: "thread-1".to_string(),
                product_surface: Some("codex".to_string()),
                model_slug: Some("gpt-5.1-codex".to_string()),
                completed_at: 1710000000,
                repo_hash: Some("repo-hash-1".to_string()),
                accepted_added_lines: 42,
                accepted_deleted_lines: 40,
                line_fingerprints: Vec::new(),
            },
        },
    ));

    let payload = serde_json::to_value(&event).expect("serialize accepted line fingerprints event");

    assert_eq!(
        payload,
        json!({
            "event_type": "codex_accepted_line_fingerprints",
            "event_params": {
                "event_type": "codex.accepted_line_fingerprints",
                "turn_id": "turn-1",
                "thread_id": "thread-1",
                "product_surface": "codex",
                "model_slug": "gpt-5.1-codex",
                "completed_at": 1710000000,
                "repo_hash": "repo-hash-1",
                "accepted_added_lines": 42,
                "accepted_deleted_lines": 40,
                "line_fingerprints": []
            }
        })
    );
}
#[test]
fn compaction_event_serializes_expected_shape() {
    let event = TrackEventRequest::Compaction(Box::new(CodexCompactionEventRequest {
        event_type: "codex_compaction_event",
        event_params: crate::events::codex_compaction_event_params(
            CodexCompactionEvent {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                trigger: CompactionTrigger::Auto,
                reason: CompactionReason::ContextLimit,
                implementation: CompactionImplementation::ResponsesCompact,
                phase: CompactionPhase::MidTurn,
                strategy: CompactionStrategy::Memento,
                status: CompactionStatus::Completed,
                error: None,
                active_context_tokens_before: 120_000,
                active_context_tokens_after: 18_000,
                started_at: 100,
                completed_at: 106,
                duration_ms: Some(6543),
            },
            sample_app_server_client_metadata(),
            sample_runtime_metadata(),
            Some(ThreadSource::User),
            /*subagent_source*/ None,
            /*parent_thread_id*/ None,
        ),
    }));

    let payload = serde_json::to_value(&event).expect("serialize compaction event");

    assert_eq!(
        payload,
        json!({
            "event_type": "codex_compaction_event",
            "event_params": {
                "thread_id": "thread-1",
                "turn_id": "turn-1",
                "app_server_client": {
                    "product_client_id": DEFAULT_ORIGINATOR,
                    "client_name": "codex-tui",
                    "client_version": "1.0.0",
                    "rpc_transport": "stdio",
                    "experimental_api_enabled": true
                },
                "runtime": {
                    "codex_rs_version": "0.1.0",
                    "runtime_os": "macos",
                    "runtime_os_version": "15.3.1",
                    "runtime_arch": "aarch64"
                },
                "thread_source": "user",
                "subagent_source": null,
                "parent_thread_id": null,
                "trigger": "auto",
                "reason": "context_limit",
                "implementation": "responses_compact",
                "phase": "mid_turn",
                "strategy": "memento",
                "status": "completed",
                "error": null,
                "active_context_tokens_before": 120000,
                "active_context_tokens_after": 18000,
                "started_at": 100,
                "completed_at": 106,
                "duration_ms": 6543
            }
        })
    );
}

#[test]
fn compaction_implementation_serializes_remote_v2() {
    let payload = serde_json::to_value(CompactionImplementation::ResponsesCompactionV2)
        .expect("serialize compaction implementation");

    assert_eq!(payload, json!("responses_compaction_v2"));
}

#[test]
fn thread_initialized_event_serializes_expected_shape() {
    let event = TrackEventRequest::ThreadInitialized(ThreadInitializedEvent {
        event_type: "codex_thread_initialized",
        event_params: ThreadInitializedEventParams {
            thread_id: "thread-0".to_string(),
            app_server_client: CodexAppServerClientMetadata {
                product_client_id: DEFAULT_ORIGINATOR.to_string(),
                client_name: Some("codex-tui".to_string()),
                client_version: Some("1.0.0".to_string()),
                rpc_transport: AppServerRpcTransport::Stdio,
                experimental_api_enabled: Some(true),
            },
            runtime: CodexRuntimeMetadata {
                codex_rs_version: "0.1.0".to_string(),
                runtime_os: "macos".to_string(),
                runtime_os_version: "15.3.1".to_string(),
                runtime_arch: "aarch64".to_string(),
            },
            model: "gpt-5".to_string(),
            ephemeral: true,
            thread_source: Some(ThreadSource::User),
            initialization_mode: ThreadInitializationMode::New,
            subagent_source: None,
            parent_thread_id: None,
            created_at: 1,
        },
    });

    let payload = serde_json::to_value(&event).expect("serialize thread initialized event");

    assert_eq!(
        payload,
        json!({
            "event_type": "codex_thread_initialized",
            "event_params": {
                "thread_id": "thread-0",
                "app_server_client": {
                    "product_client_id": DEFAULT_ORIGINATOR,
                    "client_name": "codex-tui",
                    "client_version": "1.0.0",
                    "rpc_transport": "stdio",
                    "experimental_api_enabled": true
                },
                "runtime": {
                    "codex_rs_version": "0.1.0",
                    "runtime_os": "macos",
                    "runtime_os_version": "15.3.1",
                    "runtime_arch": "aarch64"
                },
                "model": "gpt-5",
                "ephemeral": true,
                "thread_source": "user",
                "initialization_mode": "new",
                "subagent_source": null,
                "parent_thread_id": null,
                "created_at": 1
            }
        })
    );
}

#[test]
fn command_execution_event_serializes_expected_shape() {
    let event = TrackEventRequest::CommandExecution(CodexCommandExecutionEventRequest {
        event_type: "codex_command_execution_event",
        event_params: CodexCommandExecutionEventParams {
            base: CodexToolItemEventBase {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "item-1".to_string(),
                app_server_client: CodexAppServerClientMetadata {
                    product_client_id: "codex_tui".to_string(),
                    client_name: Some("codex-tui".to_string()),
                    client_version: Some("1.2.3".to_string()),
                    rpc_transport: AppServerRpcTransport::Websocket,
                    experimental_api_enabled: Some(true),
                },
                runtime: CodexRuntimeMetadata {
                    codex_rs_version: "0.99.0".to_string(),
                    runtime_os: "macos".to_string(),
                    runtime_os_version: "15.3.1".to_string(),
                    runtime_arch: "aarch64".to_string(),
                },
                thread_source: Some(ThreadSource::User),
                subagent_source: None,
                parent_thread_id: None,
                tool_name: "shell".to_string(),
                started_at_ms: 123_000,
                completed_at_ms: 125_000,
                duration_ms: Some(2000),
                execution_duration_ms: Some(1900),
                review_count: 0,
                guardian_review_count: 0,
                user_review_count: 0,
                final_approval_outcome: FinalApprovalOutcome::NotNeeded,
                terminal_status: ToolItemTerminalStatus::Completed,
                failure_kind: None,
                requested_additional_permissions: false,
                requested_network_access: false,
            },
            command_execution_source: CommandExecutionSource::Agent,
            exit_code: Some(0),
            command_total_action_count: 4,
            command_read_action_count: 1,
            command_list_files_action_count: 1,
            command_search_action_count: 1,
            command_unknown_action_count: 1,
        },
    });

    let payload = serde_json::to_value(&event).expect("serialize command execution event");
    assert_eq!(
        payload,
        json!({
            "event_type": "codex_command_execution_event",
            "event_params": {
                "thread_id": "thread-1",
                "turn_id": "turn-1",
                "item_id": "item-1",
                "app_server_client": {
                    "product_client_id": "codex_tui",
                    "client_name": "codex-tui",
                    "client_version": "1.2.3",
                    "rpc_transport": "websocket",
                    "experimental_api_enabled": true
                },
                "runtime": {
                    "codex_rs_version": "0.99.0",
                    "runtime_os": "macos",
                    "runtime_os_version": "15.3.1",
                    "runtime_arch": "aarch64"
                },
                "thread_source": "user",
                "subagent_source": null,
                "parent_thread_id": null,
                "tool_name": "shell",
                "started_at_ms": 123000,
                "completed_at_ms": 125000,
                "duration_ms": 2000,
                "execution_duration_ms": 1900,
                "review_count": 0,
                "guardian_review_count": 0,
                "user_review_count": 0,
                "final_approval_outcome": "not_needed",
                "terminal_status": "completed",
                "failure_kind": null,
                "requested_additional_permissions": false,
                "requested_network_access": false,
                "command_execution_source": "agent",
                "exit_code": 0,
                "command_total_action_count": 4,
                "command_read_action_count": 1,
                "command_list_files_action_count": 1,
                "command_search_action_count": 1,
                "command_unknown_action_count": 1
            }
        })
    );
}

#[test]
fn review_event_serializes_expected_shape() {
    let event = TrackEventRequest::ReviewEvent(CodexReviewEventRequest {
        event_type: "codex_review_event",
        event_params: CodexReviewEventParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: None,
            review_id: "review-1".to_string(),
            app_server_client: CodexAppServerClientMetadata {
                product_client_id: "codex_tui".to_string(),
                client_name: Some("codex-tui".to_string()),
                client_version: Some("1.2.3".to_string()),
                rpc_transport: AppServerRpcTransport::Websocket,
                experimental_api_enabled: Some(true),
            },
            runtime: CodexRuntimeMetadata {
                codex_rs_version: "0.99.0".to_string(),
                runtime_os: "macos".to_string(),
                runtime_os_version: "15.3.1".to_string(),
                runtime_arch: "aarch64".to_string(),
            },
            thread_source: Some(ThreadSource::Subagent),
            subagent_source: Some("thread_spawn".to_string()),
            parent_thread_id: Some("parent-thread-1".to_string()),
            subject_kind: ReviewSubjectKind::NetworkAccess,
            subject_name: "network_access".to_string(),
            reviewer: Reviewer::User,
            trigger: ReviewTrigger::NetworkPolicyDenial,
            status: ReviewStatus::Approved,
            resolution: ReviewResolution::NetworkPolicyAmendment,
            started_at_ms: 123,
            completed_at_ms: 125,
            duration_ms: Some(2),
        },
    });

    let payload = serde_json::to_value(&event).expect("serialize review event");
    assert_eq!(
        payload,
        json!({
            "event_type": "codex_review_event",
            "event_params": {
                "thread_id": "thread-1",
                "turn_id": "turn-1",
                "item_id": null,
                "review_id": "review-1",
                "app_server_client": {
                    "product_client_id": "codex_tui",
                    "client_name": "codex-tui",
                    "client_version": "1.2.3",
                    "rpc_transport": "websocket",
                    "experimental_api_enabled": true
                },
                "runtime": {
                    "codex_rs_version": "0.99.0",
                    "runtime_os": "macos",
                    "runtime_os_version": "15.3.1",
                    "runtime_arch": "aarch64"
                },
                "thread_source": "subagent",
                "subagent_source": "thread_spawn",
                "parent_thread_id": "parent-thread-1",
                "subject_kind": "network_access",
                "subject_name": "network_access",
                "reviewer": "user",
                "trigger": "network_policy_denial",
                "status": "approved",
                "resolution": "network_policy_amendment",
                "started_at_ms": 123,
                "completed_at_ms": 125,
                "duration_ms": 2
            }
        })
    );
}
