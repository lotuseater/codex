use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn inactive_subagent_activity_renders_indented_in_primary_view() -> Result<()> {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let primary_thread_id = ThreadId::new();
    app.enqueue_primary_thread_session(
        test_thread_session(primary_thread_id, test_path_buf("/tmp/project")),
        Vec::new(),
    )
    .await?;
    while app_event_rx.try_recv().is_ok() {}

    let agent_thread_id = ThreadId::new();
    app.upsert_agent_picker_thread(
        agent_thread_id,
        Some("Robie".to_string()),
        Some("worker".to_string()),
        /*is_closed*/ false,
    );
    app.update_agent_runtime_details(
        agent_thread_id,
        Some("gpt-5".to_string()),
        Some(ReasoningEffortConfig::High),
    );
    app.update_agent_token_context_percent_used(agent_thread_id, Some(12));

    app.enqueue_thread_notification(
        agent_thread_id,
        ServerNotification::ItemCompleted(ItemCompletedNotification {
            thread_id: agent_thread_id.to_string(),
            turn_id: "turn-1".to_string(),
            completed_at_ms: 0,
            item: ThreadItem::AgentMessage {
                id: "msg-1".to_string(),
                text: "child result ready".to_string(),
                phase: None,
                memory_citation: None,
            },
        }),
    )
    .await?;

    let mut rendered = String::new();
    while let Ok(event) = app_event_rx.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            rendered.push_str(&lines_to_single_string(&cell.display_lines(/*width*/ 120)));
        }
    }

    assert!(
        rendered.contains("    • Robie [worker] (gpt-5 high, 12% used): Message"),
        "{rendered}"
    );
    assert!(
        rendered.contains("      └ child result ready"),
        "{rendered}"
    );
    Ok(())
}

#[tokio::test]
async fn refresh_pending_thread_approvals_only_lists_inactive_threads() {
    let mut app = make_test_app().await;
    let main_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000001").expect("valid thread");
    let agent_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000002").expect("valid thread");

    app.primary_thread_id = Some(main_thread_id);
    app.active_thread_id = Some(main_thread_id);
    app.thread_event_channels
        .insert(main_thread_id, ThreadEventChannel::new(/*capacity*/ 1));

    let agent_channel = ThreadEventChannel::new(/*capacity*/ 1);
    {
        let mut store = agent_channel.store.lock().await;
        store.push_request(exec_approval_request(
            agent_thread_id,
            "turn-1",
            "call-1",
            /*approval_id*/ None,
        ));
    }
    app.thread_event_channels
        .insert(agent_thread_id, agent_channel);
    app.agent_navigation.upsert(
        agent_thread_id,
        Some("Robie".to_string()),
        Some("explorer".to_string()),
        /*is_closed*/ false,
    );

    app.refresh_pending_thread_approvals().await;
    assert_eq!(
        app.chat_widget.pending_thread_approvals(),
        &["Robie [explorer]".to_string()]
    );

    app.active_thread_id = Some(agent_thread_id);
    app.refresh_pending_thread_approvals().await;
    assert!(app.chat_widget.pending_thread_approvals().is_empty());
}

#[tokio::test]
async fn inactive_thread_approval_bubbles_into_active_view() -> Result<()> {
    let mut app = make_test_app().await;
    let main_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000011").expect("valid thread");
    let agent_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000022").expect("valid thread");

    app.primary_thread_id = Some(main_thread_id);
    app.active_thread_id = Some(main_thread_id);
    app.thread_event_channels
        .insert(main_thread_id, ThreadEventChannel::new(/*capacity*/ 1));
    app.thread_event_channels.insert(
        agent_thread_id,
        ThreadEventChannel::new_with_session(
            /*capacity*/ 1,
            ThreadSessionState {
                approval_policy: AskForApproval::OnRequest.to_core(),
                permission_profile: PermissionProfile::workspace_write(),
                rollout_path: Some(test_path_buf("/tmp/agent-rollout.jsonl")),
                ..test_thread_session(agent_thread_id, test_path_buf("/tmp/agent"))
            },
            Vec::new(),
        ),
    );
    app.agent_navigation.upsert(
        agent_thread_id,
        Some("Robie".to_string()),
        Some("explorer".to_string()),
        /*is_closed*/ false,
    );

    app.enqueue_thread_request(
        agent_thread_id,
        exec_approval_request(
            agent_thread_id,
            "turn-approval",
            "call-approval",
            /*approval_id*/ None,
        ),
    )
    .await?;

    assert_eq!(app.chat_widget.has_active_view(), true);
    assert_eq!(
        app.chat_widget.pending_thread_approvals(),
        &["Robie [explorer]".to_string()]
    );

    Ok(())
}

#[tokio::test]
async fn inactive_thread_exec_approval_preserves_context() {
    let app = make_test_app().await;
    let thread_id = ThreadId::new();
    let mut request = exec_approval_request(
        thread_id,
        "turn-approval",
        "call-approval",
        /*approval_id*/ None,
    );
    let ServerRequest::CommandExecutionRequestApproval { params, .. } = &mut request else {
        panic!("expected exec approval request");
    };
    params.network_approval_context = Some(AppServerNetworkApprovalContext {
        host: "example.com".to_string(),
        protocol: AppServerNetworkApprovalProtocol::Socks5Tcp,
    });
    params.additional_permissions = Some(AdditionalPermissionProfile {
        network: Some(AdditionalNetworkPermissions {
            enabled: Some(true),
        }),
        file_system: Some(AdditionalFileSystemPermissions {
            read: Some(vec![test_absolute_path("/tmp/read-only")]),
            write: Some(vec![test_absolute_path("/tmp/write")]),
            glob_scan_max_depth: None,
            entries: None,
        }),
    });
    params.proposed_network_policy_amendments = Some(vec![AppServerNetworkPolicyAmendment {
        host: "example.com".to_string(),
        action: AppServerNetworkPolicyRuleAction::Allow,
    }]);

    let Some(ThreadInteractiveRequest::Approval(ApprovalRequest::Exec {
        available_decisions,
        network_approval_context,
        additional_permissions,
        ..
    })) = app
        .interactive_request_for_thread_request(thread_id, &request)
        .await
    else {
        panic!("expected exec approval request");
    };

    assert_eq!(
        network_approval_context,
        Some(AppServerNetworkApprovalContext {
            host: "example.com".to_string(),
            protocol: AppServerNetworkApprovalProtocol::Socks5Tcp,
        })
    );
    assert_eq!(
        additional_permissions,
        Some(AdditionalPermissionProfile {
            network: Some(AdditionalNetworkPermissions {
                enabled: Some(true),
            }),
            file_system: Some(AdditionalFileSystemPermissions {
                read: Some(vec![test_absolute_path("/tmp/read-only")]),
                write: Some(vec![test_absolute_path("/tmp/write")]),
                glob_scan_max_depth: None,
                entries: None,
            }),
        })
    );
    assert_eq!(
        available_decisions,
        vec![
            codex_app_server_protocol::CommandExecutionApprovalDecision::Accept,
            codex_app_server_protocol::CommandExecutionApprovalDecision::AcceptForSession,
            codex_app_server_protocol::CommandExecutionApprovalDecision::ApplyNetworkPolicyAmendment {
                network_policy_amendment: AppServerNetworkPolicyAmendment {
                    host: "example.com".to_string(),
                    action: AppServerNetworkPolicyRuleAction::Allow,
                },
            },
            codex_app_server_protocol::CommandExecutionApprovalDecision::Cancel,
        ]
    );
}

#[tokio::test]
async fn inactive_thread_exec_approval_splits_shell_wrapped_command() {
    let app = make_test_app().await;
    let thread_id = ThreadId::new();
    let script = r#"python3 -c 'print("Hello, world!")'"#;
    let mut request = exec_approval_request(
        thread_id,
        "turn-approval",
        "call-approval",
        /*approval_id*/ None,
    );
    let ServerRequest::CommandExecutionRequestApproval { params, .. } = &mut request else {
        panic!("expected exec approval request");
    };
    params.command =
        Some(shlex::try_join(["/bin/zsh", "-lc", script]).expect("round-trippable shell wrapper"));

    let Some(ThreadInteractiveRequest::Approval(ApprovalRequest::Exec { command, .. })) = app
        .interactive_request_for_thread_request(thread_id, &request)
        .await
    else {
        panic!("expected exec approval request");
    };

    assert_eq!(
        command,
        vec![
            "/bin/zsh".to_string(),
            "-lc".to_string(),
            script.to_string(),
        ]
    );
}

#[tokio::test]
async fn inactive_thread_file_change_approval_recovers_buffered_changes() {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    app.enqueue_thread_notification(
        thread_id,
        ServerNotification::ItemStarted(ItemStartedNotification {
            thread_id: thread_id.to_string(),
            turn_id: "turn-approval".to_string(),
            started_at_ms: 0,
            item: ThreadItem::FileChange {
                id: "patch-approval".to_string(),
                changes: vec![FileUpdateChange {
                    path: "README.md".to_string(),
                    kind: PatchChangeKind::Add,
                    diff: "hello\n".to_string(),
                }],
                status: codex_app_server_protocol::PatchApplyStatus::InProgress,
            },
        }),
    )
    .await
    .expect("enqueue file change item");

    let request = ServerRequest::FileChangeRequestApproval {
        request_id: AppServerRequestId::Integer(9),
        params: FileChangeRequestApprovalParams {
            thread_id: thread_id.to_string(),
            turn_id: "turn-approval".to_string(),
            item_id: "patch-approval".to_string(),
            started_at_ms: 0,
            reason: Some("command failed; retry without sandbox?".to_string()),
            grant_root: None,
        },
    };

    let request = app
        .interactive_request_for_thread_request(thread_id, &request)
        .await
        .expect("expected file change approval request");

    let ThreadInteractiveRequest::Approval(ApprovalRequest::ApplyPatch {
        changes, reason, ..
    }) = &request
    else {
        panic!("expected apply-patch approval request");
    };
    assert_eq!(
        changes,
        &HashMap::from([(
            PathBuf::from("README.md"),
            FileChange::Add {
                content: "hello\n".to_string(),
            },
        )])
    );
    assert_eq!(
        reason,
        &Some("command failed; retry without sandbox?".to_string())
    );

    app.push_thread_interactive_request(request);
    let cell = match app_event_rx.try_recv() {
        Ok(AppEvent::InsertHistoryCell(cell)) => cell,
        other => panic!("expected patch preview history cell, saw {other:?}"),
    };
    let rendered = lines_to_single_string(&cell.display_lines(/*width*/ 80));
    assert!(rendered.contains("• Added README.md (+1 -0)"));
    assert!(rendered.contains("1 +hello"));
}

#[tokio::test]
async fn inactive_thread_permissions_approval_preserves_file_system_permissions() {
    let app = make_test_app().await;
    let thread_id = ThreadId::new();
    let request = ServerRequest::PermissionsRequestApproval {
        request_id: AppServerRequestId::Integer(7),
        params: PermissionsRequestApprovalParams {
            thread_id: thread_id.to_string(),
            turn_id: "turn-approval".to_string(),
            item_id: "call-approval".to_string(),
            started_at_ms: 0,
            cwd: test_absolute_path("/tmp"),
            reason: Some("Need access to .git".to_string()),
            permissions: codex_app_server_protocol::RequestPermissionProfile {
                network: Some(AdditionalNetworkPermissions {
                    enabled: Some(true),
                }),
                file_system: Some(AdditionalFileSystemPermissions {
                    read: Some(vec![test_absolute_path("/tmp/read-only")]),
                    write: Some(vec![test_absolute_path("/tmp/write")]),
                    glob_scan_max_depth: None,
                    entries: None,
                }),
            },
        },
    };

    let Some(ThreadInteractiveRequest::Approval(ApprovalRequest::Permissions {
        permissions, ..
    })) = app
        .interactive_request_for_thread_request(thread_id, &request)
        .await
    else {
        panic!("expected permissions approval request");
    };

    assert_eq!(
        permissions,
        RequestPermissionProfile {
            network: Some(NetworkPermissions {
                enabled: Some(true),
            }),
            file_system: Some(FileSystemPermissions::from_read_write_roots(
                Some(vec![test_absolute_path("/tmp/read-only")]),
                Some(vec![test_absolute_path("/tmp/write")]),
            )),
        }
    );
}

#[tokio::test]
async fn inactive_thread_url_elicitation_routes_to_app_link() {
    let app = make_test_app().await;
    let thread_id = ThreadId::new();
    let request = ServerRequest::McpServerElicitationRequest {
        request_id: AppServerRequestId::Integer(9),
        params: McpServerElicitationRequestParams {
            thread_id: thread_id.to_string(),
            turn_id: Some("turn-auth".to_string()),
            server_name: "payments".to_string(),
            request: McpServerElicitationRequest::Url {
                meta: None,
                message: "Review the payment details to continue.".to_string(),
                url: "https://payments.example/checkout/123".to_string(),
                elicitation_id: "payment-123".to_string(),
            },
        },
    };

    let Some(ThreadInteractiveRequest::AppLink(params)) = app
        .interactive_request_for_thread_request(thread_id, &request)
        .await
    else {
        panic!("expected app link request");
    };

    assert_eq!(params.title, "Action required");
    assert_eq!(params.description, Some("Server: payments".to_string()));
    assert_eq!(params.url, "https://payments.example/checkout/123");
    assert_eq!(
        params.elicitation_target,
        Some(crate::bottom_pane::AppLinkElicitationTarget {
            thread_id,
            server_name: "payments".to_string(),
            request_id: AppServerRequestId::Integer(9),
        })
    );
}

#[tokio::test]
async fn inactive_thread_invalid_url_elicitation_is_declined() {
    let (app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let request = ServerRequest::McpServerElicitationRequest {
        request_id: AppServerRequestId::Integer(10),
        params: McpServerElicitationRequestParams {
            thread_id: thread_id.to_string(),
            turn_id: Some("turn-auth".to_string()),
            server_name: "payments".to_string(),
            request: McpServerElicitationRequest::Url {
                meta: None,
                message: "Review the payment details to continue.".to_string(),
                url: "http://payments.example/checkout/123".to_string(),
                elicitation_id: "payment-123".to_string(),
            },
        },
    };

    assert!(
        app.interactive_request_for_thread_request(thread_id, &request)
            .await
            .is_none()
    );
    assert_matches!(
        app_event_rx.try_recv(),
        Ok(AppEvent::SubmitThreadOp {
            thread_id: op_thread_id,
            op: Op::ResolveElicitation {
                server_name,
                request_id: AppServerRequestId::Integer(10),
                decision: codex_app_server_protocol::McpServerElicitationAction::Decline,
                content: None,
                meta: None,
            },
        }) if op_thread_id == thread_id && server_name == "payments"
    );
}

#[tokio::test]
async fn inactive_thread_approval_badge_clears_after_turn_completion_notification() -> Result<()> {
    let mut app = make_test_app().await;
    let main_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000101").expect("valid thread");
    let agent_thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000202").expect("valid thread");

    app.primary_thread_id = Some(main_thread_id);
    app.active_thread_id = Some(main_thread_id);
    app.thread_event_channels
        .insert(main_thread_id, ThreadEventChannel::new(/*capacity*/ 1));
    app.thread_event_channels.insert(
        agent_thread_id,
        ThreadEventChannel::new_with_session(
            /*capacity*/ 4,
            ThreadSessionState {
                approval_policy: AskForApproval::OnRequest.to_core(),
                permission_profile: PermissionProfile::workspace_write(),
                rollout_path: Some(test_path_buf("/tmp/agent-rollout.jsonl")),
                ..test_thread_session(agent_thread_id, test_path_buf("/tmp/agent"))
            },
            Vec::new(),
        ),
    );
    app.agent_navigation.upsert(
        agent_thread_id,
        Some("Robie".to_string()),
        Some("explorer".to_string()),
        /*is_closed*/ false,
    );

    app.enqueue_thread_request(
        agent_thread_id,
        exec_approval_request(
            agent_thread_id,
            "turn-approval",
            "call-approval",
            /*approval_id*/ None,
        ),
    )
    .await?;
    assert_eq!(
        app.chat_widget.pending_thread_approvals(),
        &["Robie [explorer]".to_string()]
    );

    app.enqueue_thread_notification(
        agent_thread_id,
        turn_completed_notification(agent_thread_id, "turn-approval", TurnStatus::Completed),
    )
    .await?;

    assert!(
        app.chat_widget.pending_thread_approvals().is_empty(),
        "turn completion should clear inactive-thread approval badge immediately"
    );

    Ok(())
}
