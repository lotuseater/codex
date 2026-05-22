use super::*;
use crate::session::input_queue::TurnInput;

#[tokio::test]
async fn request_mcp_server_elicitation_auto_accepts_when_auto_deny_is_enabled() {
    let (session, turn_context, rx) = make_session_and_context_with_rx().await;
    session
        .services
        .mcp_connection_manager
        .read()
        .await
        .set_elicitations_auto_deny(/*auto_deny*/ true);

    let requested_schema: McpElicitationSchema = serde_json::from_value(json!({
        "type": "object",
        "properties": {},
    }))
    .expect("schema should deserialize");
    let response = session
        .request_mcp_server_elicitation(
            turn_context.as_ref(),
            RequestId::String("request-1".into()),
            McpServerElicitationRequestParams {
                thread_id: session.conversation_id.to_string(),
                turn_id: Some(turn_context.sub_id.clone()),
                server_name: "codex_apps".to_string(),
                request: McpServerElicitationRequest::Form {
                    meta: None,
                    message: "Allow this request?".to_string(),
                    requested_schema,
                },
            },
        )
        .await;

    assert_eq!(
        response.response,
        Some(ElicitationResponse {
            action: ElicitationAction::Accept,
            content: Some(json!({})),
            meta: None,
        })
    );
    assert!(!response.sent);
    assert!(rx.try_recv().is_err());
}

#[test]
fn assistant_message_stream_parsers_can_be_seeded_from_output_item_added_text() {
    let mut parsers = AssistantMessageStreamParsers::new(/*plan_mode*/ false);
    let item_id = "msg-1";

    let seeded = parsers.seed_item_text(item_id, "hello <oai-mem-citation>doc");
    let parsed = parsers.parse_delta(item_id, "1</oai-mem-citation> world");
    let tail = parsers.finish_item(item_id);

    assert_eq!(seeded.visible_text, "hello ");
    assert_eq!(seeded.citations, Vec::<String>::new());
    assert_eq!(parsed.visible_text, " world");
    assert_eq!(parsed.citations, vec!["doc1".to_string()]);
    assert_eq!(tail.visible_text, "");
    assert_eq!(tail.citations, Vec::<String>::new());
}

#[test]
fn assistant_message_stream_parsers_seed_buffered_prefix_stays_out_of_finish_tail() {
    let mut parsers = AssistantMessageStreamParsers::new(/*plan_mode*/ false);
    let item_id = "msg-1";

    let seeded = parsers.seed_item_text(item_id, "hello <oai-mem-");
    let parsed = parsers.parse_delta(item_id, "citation>doc</oai-mem-citation> world");
    let tail = parsers.finish_item(item_id);

    assert_eq!(seeded.visible_text, "hello ");
    assert_eq!(seeded.citations, Vec::<String>::new());
    assert_eq!(parsed.visible_text, " world");
    assert_eq!(parsed.citations, vec!["doc".to_string()]);
    assert_eq!(tail.visible_text, "");
    assert_eq!(tail.citations, Vec::<String>::new());
}

#[test]
fn assistant_message_stream_parsers_seed_plan_parser_across_added_and_delta_boundaries() {
    let mut parsers = AssistantMessageStreamParsers::new(/*plan_mode*/ true);
    let item_id = "msg-1";

    let seeded = parsers.seed_item_text(item_id, "Intro\n<proposed");
    let parsed = parsers.parse_delta(item_id, "_plan>\n- step\n</proposed_plan>\nOutro");
    let tail = parsers.finish_item(item_id);

    assert_eq!(seeded.visible_text, "Intro\n");
    assert_eq!(
        seeded.plan_segments,
        vec![ProposedPlanSegment::Normal("Intro\n".to_string())]
    );
    assert_eq!(parsed.visible_text, "Outro");
    assert_eq!(
        parsed.plan_segments,
        vec![
            ProposedPlanSegment::ProposedPlanStart,
            ProposedPlanSegment::ProposedPlanDelta("- step\n".to_string()),
            ProposedPlanSegment::ProposedPlanEnd,
            ProposedPlanSegment::Normal("Outro".to_string()),
        ]
    );
    assert_eq!(tail.visible_text, "");
    assert!(tail.plan_segments.is_empty());
}

#[test]
fn filter_connectors_for_input_skips_duplicate_slug_mentions() {
    let connectors = vec![
        make_connector("one", "Foo Bar"),
        make_connector("two", "Foo-Bar"),
    ];
    let input = vec![user_message("use $foo-bar")];
    let explicitly_enabled_connectors = HashSet::new();
    let skill_name_counts_lower = HashMap::new();

    let selected = filter_connectors_for_input(
        &connectors,
        &input,
        &explicitly_enabled_connectors,
        &skill_name_counts_lower,
    );

    assert_eq!(selected, Vec::new());
}

#[test]
fn filter_connectors_for_input_skips_when_skill_name_conflicts() {
    let connectors = vec![make_connector("one", "Todoist")];
    let input = vec![user_message("use $todoist")];
    let explicitly_enabled_connectors = HashSet::new();
    let skill_name_counts_lower = HashMap::from([("todoist".to_string(), 1)]);

    let selected = filter_connectors_for_input(
        &connectors,
        &input,
        &explicitly_enabled_connectors,
        &skill_name_counts_lower,
    );

    assert_eq!(selected, Vec::new());
}

#[test]
fn filter_connectors_for_input_skips_disabled_connectors() {
    let mut connector = make_connector("calendar", "Calendar");
    connector.is_enabled = false;
    let input = vec![user_message("use $calendar")];
    let explicitly_enabled_connectors = HashSet::new();
    let selected = filter_connectors_for_input(
        &[connector],
        &input,
        &explicitly_enabled_connectors,
        &HashMap::new(),
    );

    assert_eq!(selected, Vec::new());
}

#[test]
fn filter_connectors_for_input_skips_plugin_mentions() {
    let connectors = vec![make_connector("figma", "Figma")];
    let input = vec![user_message("use [@figma](plugin://figma@openai-curated)")];
    let explicitly_enabled_connectors = HashSet::new();
    let selected = filter_connectors_for_input(
        &connectors,
        &input,
        &explicitly_enabled_connectors,
        &HashMap::new(),
    );

    assert_eq!(selected, Vec::new());
}

#[test]
fn collect_explicit_app_ids_from_skill_items_includes_linked_mentions() {
    let connectors = vec![make_connector("calendar", "Calendar")];
    let skill_items = vec![skill_message(
        "<skill>\n<name>demo</name>\n<path>/tmp/skills/demo/SKILL.md</path>\nuse [$calendar](app://calendar)\n</skill>",
    )];

    let connector_ids =
        collect_explicit_app_ids_from_skill_items(&skill_items, &connectors, &HashMap::new());

    assert_eq!(connector_ids, HashSet::from(["calendar".to_string()]));
}

#[test]
fn collect_explicit_app_ids_from_skill_items_resolves_unambiguous_plain_mentions() {
    let connectors = vec![make_connector("calendar", "Calendar")];
    let skill_items = vec![skill_message(
        "<skill>\n<name>demo</name>\n<path>/tmp/skills/demo/SKILL.md</path>\nuse $calendar\n</skill>",
    )];

    let connector_ids =
        collect_explicit_app_ids_from_skill_items(&skill_items, &connectors, &HashMap::new());

    assert_eq!(connector_ids, HashSet::from(["calendar".to_string()]));
}

#[test]
fn collect_explicit_app_ids_from_skill_items_skips_plain_mentions_with_skill_conflicts() {
    let connectors = vec![make_connector("calendar", "Calendar")];
    let skill_items = vec![skill_message(
        "<skill>\n<name>demo</name>\n<path>/tmp/skills/demo/SKILL.md</path>\nuse $calendar\n</skill>",
    )];
    let skill_name_counts_lower = HashMap::from([("calendar".to_string(), 1)]);

    let connector_ids = collect_explicit_app_ids_from_skill_items(
        &skill_items,
        &connectors,
        &skill_name_counts_lower,
    );

    assert_eq!(connector_ids, HashSet::<String>::new());
}

#[test]
fn prefers_structured_content_when_present() {
    let ctr = McpCallToolResult {
        // Content present but should be ignored because structured_content is set.
        content: vec![text_block("ignored")],
        is_error: None,
        structured_content: Some(json!({
            "ok": true,
            "value": 42
        })),
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&json!({
                "ok": true,
                "value": 42
            }))
            .unwrap(),
        ),
        success: Some(true),
    };

    assert_eq!(expected, got);
}

#[tokio::test]
async fn includes_timed_out_message() {
    let exec = ExecToolCallOutput {
        exit_code: 0,
        stdout: StreamOutput::new(String::new()),
        stderr: StreamOutput::new(String::new()),
        aggregated_output: StreamOutput::new("Command output".to_string()),
        duration: StdDuration::from_secs(1),
        timed_out: true,
    };
    let (_, turn_context) = make_session_and_context().await;

    let out = format_exec_output_str(&exec, turn_context.truncation_policy);

    assert_eq!(
        out,
        "command timed out after 1000 milliseconds\nCommand output"
    );
}

#[test]
fn falls_back_to_content_when_structured_is_null() {
    let ctr = McpCallToolResult {
        content: vec![text_block("hello"), text_block("world")],
        is_error: None,
        structured_content: Some(serde_json::Value::Null),
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&vec![text_block("hello"), text_block("world")]).unwrap(),
        ),
        success: Some(true),
    };

    assert_eq!(expected, got);
}

#[test]
fn success_flag_reflects_is_error_true() {
    let ctr = McpCallToolResult {
        content: vec![text_block("unused")],
        is_error: Some(true),
        structured_content: Some(json!({ "message": "bad" })),
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&json!({ "message": "bad" })).unwrap(),
        ),
        success: Some(false),
    };

    assert_eq!(expected, got);
}

#[test]
fn success_flag_true_with_no_error_and_content_used() {
    let ctr = McpCallToolResult {
        content: vec![text_block("alpha")],
        is_error: Some(false),
        structured_content: None,
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&vec![text_block("alpha")]).unwrap(),
        ),
        success: Some(true),
    };

    assert_eq!(expected, got);
}

#[test]
fn get_service_tier_defaults_enterprise_accounts_to_fast() {
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::Enterprise),
            /*fast_mode_enabled*/ true,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::EnterpriseCbpUsageBased),
            /*fast_mode_enabled*/ true,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::Business),
            /*fast_mode_enabled*/ true,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::Team),
            /*fast_mode_enabled*/ true,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::SelfServeBusinessUsageBased),
            /*fast_mode_enabled*/ true,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
}

#[test]
fn get_service_tier_respects_fast_default_opt_out() {
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ true,
            Some(AccountPlanType::Enterprise),
            /*fast_mode_enabled*/ true,
        ),
        None
    );
}

#[test]
fn get_service_tier_does_not_default_non_enterprise_or_disabled_fast_mode() {
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::Pro),
            /*fast_mode_enabled*/ true,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::Enterprise),
            /*fast_mode_enabled*/ false,
        ),
        None
    );
}

#[tokio::test]
async fn session_new_fails_when_zsh_fork_enabled_without_zsh_path() {
    let codex_home = tempfile::tempdir().expect("create temp dir");
    let mut config = build_test_config(codex_home.path()).await;
    config
        .features
        .enable(Feature::ShellZshFork)
        .expect("test config should allow shell_zsh_fork");
    config.zsh_path = None;
    let config = Arc::new(config);

    let auth_manager = AuthManager::from_auth_for_testing(CodexAuth::from_api_key("Test API Key"));
    let models_manager = models_manager_with_provider(
        config.codex_home.to_path_buf(),
        auth_manager.clone(),
        config.model_provider.clone(),
    );
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort: config.model_reasoning_effort,
            developer_instructions: None,
        },
    };
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        context_budget_mode: config.context_budget_mode,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile: config.permissions.permission_profile.clone(),
        active_permission_profile: config.permissions.active_permission_profile(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: vec![config.cwd.clone()],
        profile_workspace_roots: Vec::new(),
        codex_home: config.codex_home.clone(),
        thread_name: None,
        environments: Vec::new(),
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        thread_source: None,
        dynamic_tools: Vec::new(),
        persist_extended_history: false,
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };

    let (tx_event, _rx_event) = async_channel::unbounded();
    let (agent_status_tx, _agent_status_rx) = watch::channel(AgentStatus::PendingInit);
    let plugins_manager = Arc::new(PluginsManager::new(config.codex_home.to_path_buf()));
    let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
    let skills_manager = Arc::new(SkillsManager::new(
        config.codex_home.clone(),
        /*bundled_skills_enabled*/ true,
    ));
    let result = Session::new(
        session_configuration,
        Arc::clone(&config),
        "11111111-1111-4111-8111-111111111111".to_string(),
        auth_manager,
        models_manager,
        Arc::new(ExecPolicyManager::default()),
        tx_event,
        agent_status_tx,
        InitialHistory::New,
        SessionSource::Exec,
        skills_manager,
        plugins_manager,
        mcp_manager,
        Arc::new(codex_extension_api::ExtensionRegistryBuilder::new().build()),
        AgentControl::default(),
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        /*analytics_events_client*/ None,
        Arc::new(RecordingThreadStore::default()),
        Arc::new(RecordingLiveThreadFactory::new()),
        /*state_db*/ None,
        codex_rollout_trace::ThreadTraceContext::disabled(),
        /*attestation_provider*/ None,
    )
    .await;

    let err = match result {
        Ok(_) => panic!("expected startup to fail"),
        Err(err) => err,
    };
    let msg = format!("{err:#}");
    assert!(msg.contains("zsh fork feature enabled, but `zsh_path` is not configured"));
}

#[tokio::test]
async fn build_initial_context_uses_turn_collaboration_mode() {
    let (session, mut turn_context) = make_session_and_context().await;
    {
        let mut state = session.state.lock().await;
        state.session_configuration.collaboration_mode = CollaborationMode {
            mode: ModeKind::Default,
            settings: Settings {
                model: turn_context.model_info.slug.clone(),
                reasoning_effort: None,
                developer_instructions: Some("DEFAULT SESSION INSTRUCTIONS".to_string()),
            },
        };
    }
    turn_context.collaboration_mode = CollaborationMode {
        mode: ModeKind::Plan,
        settings: Settings {
            model: turn_context.model_info.slug.clone(),
            reasoning_effort: None,
            developer_instructions: Some("PLAN TURN INSTRUCTIONS".to_string()),
        },
    };

    let context = session.build_initial_context(&turn_context).await;
    let developer_text = developer_input_texts(&context).join("\n");

    assert!(developer_text.contains("PLAN TURN INSTRUCTIONS"));
    assert!(!developer_text.contains("DEFAULT SESSION INSTRUCTIONS"));
}

#[tokio::test]
async fn build_initial_context_includes_batch_mini_programming_when_workflow_batch_available() {
    let (session, mut turn_context) = make_session_and_context().await;
    turn_context.tools_config.workflow_batch_enabled = true;
    turn_context.tools_config.environment_mode = ToolEnvironmentMode::Single;

    let context = session.build_initial_context(&turn_context).await;
    let developer_text = developer_input_texts(&context).join("\n");

    assert!(developer_text.contains("<batch_mini_programming_instructions>"));
    assert!(developer_text.contains("never include `response_length`"));
    assert!(developer_text.contains("step payloads are objects"));
    assert!(developer_text.contains("Use focused shell/rg for one-off searches"));
    assert!(!developer_text.contains("When the `workflow_batch` tool is available"));
    assert!(developer_text.contains("compact root-confined deterministic local file/JSON IO"));
    assert!(
        developer_text.contains("bounded scans, transforms, assertions, loops, and reductions")
    );
    assert!(developer_text.contains("Use `workflow_batch` for compact root-confined"));
    assert!(developer_text.contains("`spec` is `{\"steps\":[...]}`"));
    assert!(developer_text.contains("exactly one of `spec` or `spec_path`"));
    assert!(developer_text.contains("Python for richer algorithms or reusable prototypes"));
    assert!(developer_text.contains("Keep batches small"));
}

#[tokio::test]
async fn build_initial_context_omits_batch_mini_programming_without_workflow_batch() {
    let (session, mut turn_context) = make_session_and_context().await;
    turn_context.tools_config.workflow_batch_enabled = false;
    turn_context.tools_config.environment_mode = ToolEnvironmentMode::Single;

    let context = session.build_initial_context(&turn_context).await;
    let developer_text = developer_input_texts(&context).join("\n");

    assert!(!developer_text.contains("<batch_mini_programming_instructions>"));
}

#[tokio::test]
async fn resumed_root_session_uses_thread_id_as_session_id() {
    let thread_id = ThreadId::new();
    let (session, rx_event) = make_session_with_history_source_and_agent_control_and_rx(
        InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Vec::new(),
            rollout_path: None,
        }),
        SessionSource::Exec,
        AgentControl::default(),
    )
    .await
    .expect("resume should succeed");

    assert_eq!(session.thread_id(), thread_id);
    assert_eq!(session.session_id(), SessionId::from(thread_id));

    let event = rx_event.recv().await.expect("session configured event");
    let EventMsg::SessionConfigured(event) = event.msg else {
        panic!("expected session configured event");
    };
    assert_eq!(event.session_id, SessionId::from(thread_id));
    assert_eq!(event.thread_id, thread_id);
}

#[tokio::test]
async fn resumed_subagent_session_keeps_inherited_session_id() {
    let parent_thread_id = ThreadId::new();
    let parent_session_id = SessionId::from(parent_thread_id);
    let thread_id = ThreadId::new();
    let session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id,
        depth: 1,
        agent_path: None,
        agent_nickname: None,
        agent_role: None,
    });
    let (session, rx_event) = make_session_with_history_source_and_agent_control_and_rx(
        InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Vec::new(),
            rollout_path: None,
        }),
        session_source,
        AgentControl::default().with_session_id(parent_session_id),
    )
    .await
    .expect("resume should succeed");

    assert_eq!(session.thread_id(), thread_id);
    assert_eq!(session.session_id(), parent_session_id);

    let event = rx_event.recv().await.expect("session configured event");
    let EventMsg::SessionConfigured(event) = event.msg else {
        panic!("expected session configured event");
    };
    assert_eq!(event.session_id, parent_session_id);
    assert_eq!(event.thread_id, thread_id);
}

#[tokio::test]
async fn enable_strict_auto_review_for_turn_uses_originating_turn() {
    let (session, _turn_context) = make_session_and_context().await;
    let originating_active_turn = ActiveTurn::default();
    let originating_turn_state = Arc::clone(&originating_active_turn.turn_state);
    *session.active_turn.lock().await = Some(originating_active_turn);

    let requested_permissions = RequestPermissionProfile {
        network: Some(codex_protocol::models::NetworkPermissions {
            enabled: Some(true),
        }),
        ..RequestPermissionProfile::default()
    };
    session
        .record_granted_request_permissions_for_turn(
            &codex_protocol::request_permissions::RequestPermissionsResponse {
                permissions: requested_permissions.clone(),
                scope: PermissionGrantScope::Turn,
                strict_auto_review: true,
            },
            Some(&originating_turn_state),
        )
        .await;

    assert!(
        originating_turn_state
            .lock()
            .await
            .strict_auto_review_enabled()
    );
}

#[tokio::test]
async fn submit_with_id_captures_current_span_trace_context() {
    let (session, _turn_context) = make_session_and_context().await;
    let (tx_sub, rx_sub) = async_channel::bounded(1);
    let (_tx_event, rx_event) = async_channel::unbounded();
    let (_agent_status_tx, agent_status) = watch::channel(AgentStatus::PendingInit);
    let codex = Codex {
        tx_sub,
        rx_event,
        agent_status,
        session: Arc::new(session),
        session_loop_termination: completed_session_loop_termination(),
    };

    let _trace_test_context = install_test_tracing("codex-core-tests");

    let request_parent = W3cTraceContext {
        traceparent: Some("00-00000000000000000000000000000011-0000000000000022-01".into()),
        tracestate: Some("vendor=value".into()),
    };
    let request_span = info_span!("app_server.request");
    assert!(set_parent_from_w3c_trace_context(
        &request_span,
        &request_parent
    ));

    let expected_trace = async {
        let expected_trace =
            current_span_w3c_trace_context().expect("current span should have trace context");
        codex
            .submit_with_id(Submission {
                id: "sub-1".into(),
                op: Op::Interrupt,
                trace: None,
            })
            .await
            .expect("submit should succeed");
        expected_trace
    }
    .instrument(request_span)
    .await;

    let submitted = rx_sub.recv().await.expect("submission");
    assert_eq!(submitted.trace, Some(expected_trace));
}

#[tokio::test]
async fn new_default_turn_captures_current_span_trace_id() {
    let (session, _turn_context) = make_session_and_context().await;

    let _trace_test_context = install_test_tracing("codex-core-tests");

    let request_parent = W3cTraceContext {
        traceparent: Some("00-00000000000000000000000000000011-0000000000000022-01".into()),
        tracestate: Some("vendor=value".into()),
    };
    let request_span = info_span!("app_server.request");
    assert!(set_parent_from_w3c_trace_context(
        &request_span,
        &request_parent
    ));

    let turn_context_item = async {
        let expected_trace_id = Span::current()
            .context()
            .span()
            .span_context()
            .trace_id()
            .to_string();
        let turn_context = session.new_default_turn().await;
        let turn_context_item = turn_context.to_turn_context_item();
        assert_eq!(turn_context_item.trace_id, Some(expected_trace_id));
        turn_context_item
    }
    .instrument(request_span)
    .await;

    assert_eq!(
        turn_context_item.trace_id.as_deref(),
        Some("00000000000000000000000000000011")
    );
}

#[test]
fn submission_dispatch_span_prefers_submission_trace_context() {
    let _trace_test_context = install_test_tracing("codex-core-tests");

    let ambient_parent = W3cTraceContext {
        traceparent: Some("00-00000000000000000000000000000033-0000000000000044-01".into()),
        tracestate: None,
    };
    let ambient_span = info_span!("ambient");
    assert!(set_parent_from_w3c_trace_context(
        &ambient_span,
        &ambient_parent
    ));

    let submission_trace = W3cTraceContext {
        traceparent: Some("00-00000000000000000000000000000055-0000000000000066-01".into()),
        tracestate: Some("vendor=value".into()),
    };
    let dispatch_span = ambient_span.in_scope(|| {
        submission_dispatch_span(&Submission {
            id: "sub-1".into(),
            op: Op::Interrupt,
            trace: Some(submission_trace),
        })
    });

    let trace_id = dispatch_span.context().span().span_context().trace_id();
    assert_eq!(
        trace_id,
        TraceId::from_hex("00000000000000000000000000000055").expect("trace id")
    );
}

#[test]
fn submission_dispatch_span_uses_debug_for_realtime_audio() {
    let _trace_test_context = install_test_tracing("codex-core-tests");

    let dispatch_span = submission_dispatch_span(&Submission {
        id: "sub-1".into(),
        op: Op::RealtimeConversationAudio(ConversationAudioParams {
            frame: RealtimeAudioFrame {
                data: "ZmFrZQ==".into(),
                sample_rate: 16_000,
                num_channels: 1,
                samples_per_channel: Some(160),
                item_id: None,
            },
        }),
        trace: None,
    });

    assert_eq!(
        dispatch_span.metadata().expect("span metadata").level(),
        &tracing::Level::DEBUG
    );
}

#[test]
fn op_kind_distinguishes_turn_ops() {
    assert_eq!(
        Op::OverrideTurnContext {
            cwd: None,
            approval_policy: None,
            approvals_reviewer: None,
            sandbox_policy: None,
            permission_profile: None,
            windows_sandbox_level: None,
            model: None,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        }
        .kind(),
        "override_turn_context"
    );
    assert_eq!(
        Op::UserInput {
            environments: None,
            items: vec![],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        }
        .kind(),
        "user_input"
    );
    assert_eq!(
        Op::UserInputWithTurnContext {
            environments: None,
            items: vec![],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            cwd: None,
            workspace_roots: None,
            profile_workspace_roots: None,
            approval_policy: None,
            approvals_reviewer: None,
            sandbox_policy: None,
            permission_profile: None,
            active_permission_profile: None,
            windows_sandbox_level: None,
            model: None,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        }
        .kind(),
        "user_input_with_turn_context"
    );
}

#[tokio::test]
async fn cached_guardian_subagent_exposes_its_rollout_path() {
    let (parent_session, _parent_turn_context) = make_session_and_context().await;
    let parent_session = Arc::new(parent_session);

    let (mut child_session, _child_turn_context) = make_session_and_context().await;
    let child_rollout_path = attach_thread_persistence(&mut child_session).await;
    let (child_tx_sub, _child_rx_sub) = async_channel::bounded(4);
    let (_child_tx_event, child_rx_event) = async_channel::unbounded();
    let (_child_status_tx, child_agent_status) = watch::channel(AgentStatus::PendingInit);
    let child_session_loop_handle = tokio::spawn(async {});
    let child_codex = Codex {
        tx_sub: child_tx_sub,
        rx_event: child_rx_event,
        agent_status: child_agent_status,
        session: Arc::new(child_session),
        session_loop_termination: session_loop_termination_from_handle(child_session_loop_handle),
    };
    parent_session
        .guardian_review_session
        .cache_for_test(child_codex)
        .await;

    assert_eq!(
        parent_session
            .guardian_review_session
            .trunk_rollout_path()
            .await,
        Some(child_rollout_path)
    );
}

#[tokio::test]
async fn refresh_mcp_servers_is_deferred_until_next_turn() {
    let (session, turn_context) = make_session_and_context().await;
    let old_token = session.mcp_startup_cancellation_token().await;
    assert!(!old_token.is_cancelled());

    let mcp_oauth_credentials_store_mode =
        serde_json::to_value(OAuthCredentialsStoreMode::Auto).expect("serialize store mode");
    let refresh_config = McpServerRefreshConfig {
        mcp_servers: json!({}),
        mcp_oauth_credentials_store_mode,
    };
    {
        let mut guard = session.pending_mcp_server_refresh_config.lock().await;
        *guard = Some(refresh_config);
    }

    assert!(!old_token.is_cancelled());
    assert!(
        session
            .pending_mcp_server_refresh_config
            .lock()
            .await
            .is_some()
    );

    session
        .refresh_mcp_servers_if_requested(&turn_context, /*elicitation_reviewer*/ None)
        .await;

    assert!(old_token.is_cancelled());
    assert!(
        session
            .pending_mcp_server_refresh_config
            .lock()
            .await
            .is_none()
    );
    let new_token = session.mcp_startup_cancellation_token().await;
    assert!(!new_token.is_cancelled());
}

#[tokio::test]
async fn build_initial_context_uses_previous_realtime_state() {
    let (session, mut turn_context) = make_session_and_context().await;
    turn_context.realtime_active = true;

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_texts = developer_input_texts(&initial_context);
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("<realtime_conversation>")),
        "expected initial context to describe active realtime state, got {developer_texts:?}"
    );

    let previous_context_item = turn_context.to_turn_context_item();
    {
        let mut state = session.state.lock().await;
        state.set_reference_context_item(Some(previous_context_item));
    }
    let resumed_context = session.build_initial_context(&turn_context).await;
    let resumed_developer_texts = developer_input_texts(&resumed_context);
    assert!(
        !resumed_developer_texts
            .iter()
            .any(|text| text.contains("<realtime_conversation>")),
        "did not expect a duplicate realtime update, got {resumed_developer_texts:?}"
    );
}

#[tokio::test]
async fn build_initial_context_includes_git_attribution_from_extensions() {
    let (mut session, turn_context) = make_session_and_context().await;
    session.services.extensions = git_attribution_test_registry();
    session
        .services
        .thread_extension_data
        .insert(GitAttributionTestState);

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_messages = developer_message_texts(&initial_context);

    assert!(
        developer_messages
            .iter()
            .flatten()
            .any(|text| *text == "git attribution extension enabled"),
        "expected git attribution developer text, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_omits_git_attribution_when_feature_is_disabled() {
    let (mut session, turn_context) = make_session_and_context().await;
    session.services.extensions = git_attribution_test_registry();

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_messages = developer_message_texts(&initial_context);

    assert!(
        !developer_messages
            .iter()
            .flatten()
            .any(|text| *text == "git attribution extension enabled"),
        "did not expect git attribution developer text, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_adds_multi_agent_v2_root_usage_hint_as_developer_message() {
    let (session, turn_context) =
        make_multi_agent_v2_usage_hint_test_session(/*enable_multi_agent_v2*/ true).await;

    let initial_context = session.build_initial_context(turn_context.as_ref()).await;

    let developer_messages = developer_message_texts(&initial_context);
    assert!(
        developer_messages
            .iter()
            .any(|message| message.as_slice() == ["Root guidance."]),
        "expected standalone root usage hint developer message, got {developer_messages:?}"
    );
    assert!(
        !developer_messages
            .iter()
            .any(|message| message.as_slice() == ["Subagent guidance."]),
        "did not expect subagent usage hint for root thread, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_adds_multi_agent_v2_subagent_usage_hint_as_developer_message() {
    let (session, mut turn_context) =
        make_multi_agent_v2_usage_hint_test_session(/*enable_multi_agent_v2*/ true).await;
    let session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: ThreadId::new(),
        depth: 1,
        agent_path: Some(AgentPath::try_from("/root/worker").expect("agent path should parse")),
        agent_nickname: Some("worker".to_string()),
        agent_role: None,
    });
    session
        .state
        .lock()
        .await
        .session_configuration
        .session_source = session_source.clone();
    Arc::get_mut(&mut turn_context)
        .expect("turn context should not be shared")
        .session_source = session_source;

    let initial_context = session.build_initial_context(turn_context.as_ref()).await;

    let developer_messages = developer_message_texts(&initial_context);
    assert!(
        developer_messages
            .iter()
            .any(|message| message.as_slice() == ["Subagent guidance."]),
        "expected standalone subagent usage hint developer message, got {developer_messages:?}"
    );
    assert!(
        !developer_messages
            .iter()
            .any(|message| message.as_slice() == ["Root guidance."]),
        "did not expect root usage hint for subagent thread, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_omits_multi_agent_v2_usage_hints_when_feature_disabled() {
    let (session, turn_context) =
        make_multi_agent_v2_usage_hint_test_session(/*enable_multi_agent_v2*/ false).await;

    let initial_context = session.build_initial_context(turn_context.as_ref()).await;

    let developer_messages = developer_message_texts(&initial_context);
    assert!(
        !developer_messages.iter().any(|message| {
            matches!(
                message.as_slice(),
                ["Root guidance."] | ["Subagent guidance."]
            )
        }),
        "did not expect multi-agent v2 usage hint developer messages, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_omits_multi_agent_v2_usage_hints_when_usage_hint_disabled() {
    let (session, turn_context, _rx_event) = make_session_and_context_with_auth_and_config_and_rx(
        CodexAuth::from_api_key("Test API Key"),
        Vec::new(),
        |config| {
            let _ = config.features.enable(Feature::MultiAgentV2);
            config.multi_agent_v2.usage_hint_enabled = false;
            config.multi_agent_v2.root_agent_usage_hint_text = Some("Root guidance.".to_string());
            config.multi_agent_v2.subagent_usage_hint_text = Some("Subagent guidance.".to_string());
        },
    )
    .await;

    let initial_context = session.build_initial_context(turn_context.as_ref()).await;

    let developer_messages = developer_message_texts(&initial_context);
    assert!(
        !developer_messages.iter().any(|message| {
            matches!(
                message.as_slice(),
                ["Root guidance."] | ["Subagent guidance."]
            )
        }),
        "did not expect multi-agent v2 usage hint developer messages, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_trims_skill_metadata_from_context_window_budget() {
    let (session, mut turn_context) = make_session_and_context().await;
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![
        SkillMetadata {
            name: "admin-skill".to_string(),
            description: "desc".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path_to_skills_md: test_path_buf("/tmp/admin-skill/SKILL.md").abs(),
            scope: SkillScope::Admin,
            plugin_id: None,
        },
        SkillMetadata {
            name: "repo-skill".to_string(),
            description: "desc".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path_to_skills_md: test_path_buf("/tmp/repo-skill/SKILL.md").abs(),
            scope: SkillScope::Repo,
            plugin_id: None,
        },
    ];
    turn_context.model_info.context_window = Some(100);
    turn_context.turn_skills = TurnSkillsContext::new(Arc::new(outcome));

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_texts = developer_input_texts(&initial_context);

    assert!(
        developer_texts
            .iter()
            .all(|text| !text.contains("Exceeded skills context budget")),
        "expected skill budget warning to stay out of the initial context, got {developer_texts:?}"
    );
    assert!(
        developer_texts
            .iter()
            .all(|text| !text.contains("- admin-skill:") && !text.contains("- repo-skill:")),
        "expected no skill metadata entries to fit the tiny budget, got {developer_texts:?}"
    );
}

#[test]
fn emit_thread_start_skill_metrics_records_enabled_kept_and_truncated_values() {
    let session_telemetry = test_session_telemetry_without_metadata();
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![SkillMetadata {
        name: "repo-skill".to_string(),
        description: "desc".to_string(),
        short_description: None,
        interface: None,
        dependencies: None,
        policy: None,
        path_to_skills_md: test_path_buf("/tmp/repo-skill/SKILL.md").abs(),
        scope: SkillScope::Repo,
        plugin_id: None,
    }];
    let rendered = build_available_skills(
        &outcome,
        SkillMetadataBudget::Characters(1),
        SkillRenderSideEffects::ThreadStart {
            session_telemetry: &session_telemetry,
        },
    )
    .expect("skills should render");

    assert_eq!(
        rendered.warning_message,
        Some(
            "Exceeded skills context budget. All skill descriptions were removed and 1 additional skill was not included in the model-visible skills list."
                .to_string()
        )
    );
    let snapshot = session_telemetry
        .snapshot_metrics()
        .expect("runtime metrics snapshot");
    assert_eq!(
        histogram_sum(&snapshot, THREAD_SKILLS_ENABLED_TOTAL_METRIC),
        1
    );
    assert_eq!(histogram_sum(&snapshot, THREAD_SKILLS_KEPT_TOTAL_METRIC), 0);
    assert_eq!(histogram_sum(&snapshot, THREAD_SKILLS_TRUNCATED_METRIC), 1);
    assert_eq!(
        histogram_sum(&snapshot, THREAD_SKILLS_DESCRIPTION_TRUNCATED_CHARS_METRIC),
        4
    );
}

#[test]
fn emit_thread_start_skill_metrics_records_description_truncated_chars_without_omitted_skills() {
    let session_telemetry = test_session_telemetry_without_metadata();
    let alpha = SkillMetadata {
        name: "alpha-skill".to_string(),
        description: "abcdef".to_string(),
        short_description: None,
        interface: None,
        dependencies: None,
        policy: None,
        path_to_skills_md: test_path_buf("/tmp/alpha-skill/SKILL.md").abs(),
        scope: SkillScope::Repo,
        plugin_id: None,
    };
    let beta = SkillMetadata {
        name: "beta-skill".to_string(),
        description: "uvwxyz".to_string(),
        short_description: None,
        interface: None,
        dependencies: None,
        policy: None,
        path_to_skills_md: test_path_buf("/tmp/beta-skill/SKILL.md").abs(),
        scope: SkillScope::Repo,
        plugin_id: None,
    };
    let minimum_skill_line_cost = |skill: &SkillMetadata| {
        let path = skill.path_to_skills_md.to_string_lossy().replace('\\', "/");
        format!("- {}: (file: {})\n", skill.name, path)
            .chars()
            .count()
    };
    let minimum_budget = minimum_skill_line_cost(&alpha) + minimum_skill_line_cost(&beta);
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![alpha, beta];

    let rendered = build_available_skills(
        &outcome,
        SkillMetadataBudget::Characters(minimum_budget + 6),
        SkillRenderSideEffects::ThreadStart {
            session_telemetry: &session_telemetry,
        },
    )
    .expect("skills should render");

    assert_eq!(rendered.report.omitted_count, 0);
    assert_eq!(rendered.report.truncated_description_chars, 8);
    let snapshot = session_telemetry
        .snapshot_metrics()
        .expect("runtime metrics snapshot");
    assert_eq!(histogram_sum(&snapshot, THREAD_SKILLS_TRUNCATED_METRIC), 0);
    assert_eq!(
        histogram_sum(&snapshot, THREAD_SKILLS_DESCRIPTION_TRUNCATED_CHARS_METRIC),
        8
    );
}

#[tokio::test]
async fn build_initial_context_emits_thread_start_skill_warning_on_repeated_builds() {
    let (session, turn_context, rx) = make_session_and_context_with_rx().await;
    let mut turn_context = Arc::into_inner(turn_context).expect("sole turn context owner");
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![
        SkillMetadata {
            name: "admin-skill".to_string(),
            description: "desc".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path_to_skills_md: test_path_buf("/tmp/admin-skill/SKILL.md").abs(),
            scope: SkillScope::Admin,
            plugin_id: None,
        },
        SkillMetadata {
            name: "repo-skill".to_string(),
            description: "desc".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path_to_skills_md: test_path_buf("/tmp/repo-skill/SKILL.md").abs(),
            scope: SkillScope::Repo,
            plugin_id: None,
        },
    ];
    turn_context.model_info.context_window = Some(100);
    turn_context.turn_skills = TurnSkillsContext::new(Arc::new(outcome));

    let _ = session.build_initial_context(&turn_context).await;
    let warning_event = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("warning event should arrive")
        .expect("warning event should be readable");
    assert!(matches!(
        warning_event.msg,
        EventMsg::Warning(WarningEvent { message })
            if message == "Exceeded skills context budget of 2%. All skill descriptions were removed and 2 additional skills were not included in the model-visible skills list."
    ));

    let _ = session.build_initial_context(&turn_context).await;
    let warning_event = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("warning event should arrive on repeated build")
        .expect("warning event should be readable");
    assert!(matches!(
        warning_event.msg,
        EventMsg::Warning(WarningEvent { message })
            if message == "Exceeded skills context budget of 2%. All skill descriptions were removed and 2 additional skills were not included in the model-visible skills list."
    ));
}

#[tokio::test]
async fn handle_output_item_done_skips_image_save_message_when_save_fails() {
    let (session, turn_context) = make_session_and_context().await;
    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let call_id = "ig_history_no_message";
    let expected_saved_path = crate::stream_events_utils::image_generation_artifact_path(
        &turn_context.config.codex_home,
        &session.conversation_id.to_string(),
        call_id,
    );
    let _ = std::fs::remove_file(&expected_saved_path);
    let item = ResponseItem::ImageGenerationCall {
        id: call_id.to_string(),
        status: "completed".to_string(),
        revised_prompt: Some("broken payload".to_string()),
        result: "_-8".to_string(),
    };

    let mut ctx = HandleOutputCtx {
        sess: Arc::clone(&session),
        turn_context: Arc::clone(&turn_context),
        turn_store: Arc::clone(&turn_context.extension_data),
        tool_runtime: test_tool_runtime(Arc::clone(&session), Arc::clone(&turn_context)),
        cancellation_token: CancellationToken::new(),
    };
    handle_output_item_done(&mut ctx, item.clone(), /*previously_active_item*/ None)
        .await
        .expect("image generation item should still complete");

    let history = session.clone_history().await;
    assert_eq!(history.raw_items(), &[item]);
    assert!(!expected_saved_path.exists());
}

#[tokio::test]
async fn realtime_conversation_list_voices_emits_builtin_list() {
    let (session, _turn_context, rx) = make_session_and_context_with_rx().await;

    handlers::realtime_conversation_list_voices(&session, "sub-id".to_string()).await;

    let event = rx.recv().await.expect("event");
    let voices = match event.msg {
        EventMsg::RealtimeConversationListVoicesResponse(
            RealtimeConversationListVoicesResponseEvent { voices },
        ) => voices,
        msg => panic!("expected list voices response, got {msg:?}"),
    };
    assert_eq!(
        voices,
        RealtimeVoicesList {
            v1: vec![
                RealtimeVoice::Juniper,
                RealtimeVoice::Maple,
                RealtimeVoice::Spruce,
                RealtimeVoice::Ember,
                RealtimeVoice::Vale,
                RealtimeVoice::Breeze,
                RealtimeVoice::Arbor,
                RealtimeVoice::Sol,
                RealtimeVoice::Cove,
            ],
            v2: vec![
                RealtimeVoice::Alloy,
                RealtimeVoice::Ash,
                RealtimeVoice::Ballad,
                RealtimeVoice::Coral,
                RealtimeVoice::Echo,
                RealtimeVoice::Sage,
                RealtimeVoice::Shimmer,
                RealtimeVoice::Verse,
                RealtimeVoice::Marin,
                RealtimeVoice::Cedar,
            ],
            default_v1: RealtimeVoice::Cove,
            default_v2: RealtimeVoice::Marin,
        },
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guardian_auto_review_interrupts_after_three_consecutive_denials() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![UserInput::Text {
        text: "trigger guardian denials".to_string(),
        text_elements: Vec::new(),
    }];
    sess.spawn_task(Arc::clone(&tc), input, GuardianDeniedApprovalTask)
        .await;

    let mut observed = Vec::new();
    let aborted = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let EventMsg::TurnAborted(event) = &event.msg {
                let event = event.clone();
                observed.push(EventMsg::TurnAborted(event.clone()));
                break event;
            }
            observed.push(event.msg);
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "guardian denial circuit breaker should interrupt the turn; observed events: {observed:?}"
        )
    });
    assert_eq!(aborted.reason, TurnAbortReason::Interrupted);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guardian_helper_review_interrupts_after_three_consecutive_denials() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![UserInput::Text {
        text: "keep turn active for helper reviews".to_string(),
        text_elements: Vec::new(),
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    let session_for_review = Arc::clone(&sess);
    let turn_for_review = Arc::clone(&tc);
    let turn_id = tc.sub_id.clone();
    let review_thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("helper review runtime");
        runtime.block_on(async move {
            for _ in 0..3 {
                crate::guardian::record_guardian_denial_for_test(
                    &session_for_review,
                    &turn_for_review,
                    &turn_id,
                )
                .await;
            }
        });
    });
    review_thread.join().expect("helper review thread");

    let mut observed = Vec::new();
    let aborted = timeout(StdDuration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let EventMsg::TurnAborted(event) = &event.msg {
                let event = event.clone();
                observed.push(EventMsg::TurnAborted(event.clone()));
                break event;
            }
            observed.push(event.msg);
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "helper review circuit breaker should interrupt the turn; observed events: {observed:?}"
        )
    });
    assert_eq!(aborted.reason, TurnAbortReason::Interrupted);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_log::test]
async fn abort_regular_task_emits_turn_aborted_only() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![UserInput::Text {
        text: "hello".to_string(),
        text_elements: Vec::new(),
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    // Interrupts persist a model-visible `<turn_aborted>` marker into history, but there is no
    // separate client-visible event for that marker (only `EventMsg::TurnAborted`).
    let evt = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for event")
        .expect("event");
    match evt.msg {
        EventMsg::TurnAborted(e) => assert_eq!(TurnAbortReason::Interrupted, e.reason),
        other => panic!("unexpected event: {other:?}"),
    }
    // No extra events should be emitted after an abort.
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn abort_gracefully_emits_turn_aborted_only() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![UserInput::Text {
        text: "hello".to_string(),
        text_elements: Vec::new(),
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    // Even if tasks handle cancellation gracefully, interrupts still result in `TurnAborted`
    // being the only client-visible signal.
    let evt = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for event")
        .expect("event");
    match evt.msg {
        EventMsg::TurnAborted(e) => assert_eq!(TurnAbortReason::Interrupted, e.reason),
        other => panic!("unexpected event: {other:?}"),
    }
    // No extra events should be emitted after an abort.
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn steer_input_requires_active_turn() {
    let (sess, _tc, _rx) = make_session_and_context_with_rx().await;
    let input = vec![UserInput::Text {
        text: "steer".to_string(),
        text_elements: Vec::new(),
    }];

    let err = sess
        .steer_input(
            input, /*expected_turn_id*/ None, /*responsesapi_client_metadata*/ None,
        )
        .await
        .expect_err("steering without active turn should fail");

    assert!(matches!(err, SteerInputError::NoActiveTurn(_)));
}

#[tokio::test]
async fn steer_input_enforces_expected_turn_id() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let input = vec![UserInput::Text {
        text: "hello".to_string(),
        text_elements: Vec::new(),
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    let steer_input = vec![UserInput::Text {
        text: "steer".to_string(),
        text_elements: Vec::new(),
    }];
    let err = sess
        .steer_input(
            steer_input,
            Some("different-turn-id"),
            /*responsesapi_client_metadata*/ None,
        )
        .await
        .expect_err("mismatched expected turn id should fail");

    match err {
        SteerInputError::ExpectedTurnMismatch { expected, actual } => {
            assert_eq!(
                (expected, actual),
                ("different-turn-id".to_string(), tc.sub_id.clone())
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn steer_input_rejects_non_regular_turns() {
    for (task_kind, turn_kind) in [
        (TaskKind::Review, NonSteerableTurnKind::Review),
        (TaskKind::Compact, NonSteerableTurnKind::Compact),
    ] {
        let (sess, _tc, _rx) = make_session_and_context_with_rx().await;
        let input = vec![UserInput::Text {
            text: "hello".to_string(),
            text_elements: Vec::new(),
        }];
        let turn_context = sess.new_default_turn_with_sub_id("turn".to_string()).await;
        sess.spawn_task(
            turn_context,
            input,
            NeverEndingTask {
                kind: task_kind,
                listen_to_cancellation_token: true,
            },
        )
        .await;

        let steer_input = vec![UserInput::Text {
            text: "steer".to_string(),
            text_elements: Vec::new(),
        }];
        let err = sess
            .steer_input(
                steer_input,
                /*expected_turn_id*/ None,
                /*responsesapi_client_metadata*/ None,
            )
            .await
            .expect_err("steering a non-regular turn should fail");

        assert_eq!(err, SteerInputError::ActiveTurnNotSteerable { turn_kind });

        sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
    }
}

#[tokio::test]
async fn steer_input_returns_active_turn_id() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let input = vec![UserInput::Text {
        text: "hello".to_string(),
        text_elements: Vec::new(),
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    let steer_input = vec![UserInput::Text {
        text: "steer".to_string(),
        text_elements: Vec::new(),
    }];
    let turn_id = sess
        .steer_input(
            steer_input,
            Some(&tc.sub_id),
            /*responsesapi_client_metadata*/ None,
        )
        .await
        .expect("steering with matching expected turn id should succeed");

    assert_eq!(turn_id, tc.sub_id);
    assert!(sess.has_pending_input().await);
}

#[tokio::test]
async fn prepend_pending_input_keeps_older_tail_ahead_of_newer_input() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let input = vec![UserInput::Text {
        text: "hello".to_string(),
        text_elements: Vec::new(),
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    let blocked = ResponseInputItem::Message {
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "blocked queued prompt".to_string(),
        }],
        phase: None,
    };
    let later = ResponseInputItem::Message {
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "later queued prompt".to_string(),
        }],
        phase: None,
    };
    let newer = ResponseInputItem::Message {
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "newer queued prompt".to_string(),
        }],
        phase: None,
    };

    sess.inject_response_items(vec![blocked.clone(), later.clone()])
        .await
        .expect("inject initial pending input into active turn");

    let drained = sess.get_pending_input().await;
    assert_eq!(
        drained,
        vec![TurnInput::from(blocked), TurnInput::from(later.clone())]
    );

    sess.inject_response_items(vec![newer.clone()])
        .await
        .expect("inject newer pending input into active turn");

    let mut drained_iter = drained.into_iter();
    let _blocked = drained_iter.next().expect("blocked prompt should exist");
    sess.prepend_pending_input(drained_iter.collect())
        .await
        .expect("requeue later pending input at the front of the queue");

    assert_eq!(
        sess.get_pending_input().await,
        vec![TurnInput::from(later), TurnInput::from(newer)]
    );
}

#[tokio::test]
async fn queued_response_items_for_next_turn_move_into_next_active_turn() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let queued_item = ResponseInputItem::Message {
        role: "assistant".to_string(),
        content: vec![ContentItem::InputText {
            text: "queued before wake".to_string(),
        }],
        phase: None,
    };

    sess.queue_response_items_for_next_turn(vec![queued_item.clone()])
        .await;

    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    assert_eq!(
        sess.get_pending_input().await,
        vec![TurnInput::from(queued_item)]
    );
}

#[tokio::test]
async fn idle_interrupt_does_not_wake_queued_next_turn_items() {
    let (sess, _tc, _rx) = make_session_and_context_with_rx().await;
    let queued_item = ResponseInputItem::Message {
        role: "assistant".to_string(),
        content: vec![ContentItem::InputText {
            text: "queued before interrupt".to_string(),
        }],
        phase: None,
    };

    sess.queue_response_items_for_next_turn(vec![queued_item])
        .await;

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    assert!(sess.active_turn.lock().await.is_none());
    assert!(sess.has_queued_response_items_for_next_turn().await);
}

#[tokio::test]
async fn abort_empty_active_turn_preserves_pending_input() {
    let (sess, _tc, _rx) = make_session_and_context_with_rx().await;
    let pending_item = ResponseInputItem::Message {
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "late pending input".to_string(),
        }],
        phase: None,
    };
    let turn_state = {
        let mut active = sess.active_turn.lock().await;
        let active_turn = active.get_or_insert_with(ActiveTurn::default);
        Arc::clone(&active_turn.turn_state)
    };
    turn_state
        .lock()
        .await
        .push_pending_input(pending_item.clone());

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    assert!(sess.active_turn.lock().await.is_none());
    assert_eq!(
        turn_state.lock().await.take_pending_input(),
        vec![TurnInput::from(pending_item)]
    );
}

#[tokio::test]
async fn interrupt_accounts_active_goal_before_pausing() -> anyhow::Result<()> {
    let (sess, tc, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    sess.set_thread_goal(
        tc.as_ref(),
        SetGoalRequest {
            objective: Some("Keep improving the benchmark".to_string()),
            status: None,
            token_budget: None,
        },
    )
    .await?;

    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;
    set_total_token_usage(&sess, post_goal_token_usage()).await;

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    let goal = sess
        .get_thread_goal()
        .await?
        .expect("goal should remain persisted after interrupt");
    assert_eq!(
        codex_protocol::protocol::ThreadGoalStatus::Paused,
        goal.status
    );
    assert_eq!(70, goal.tokens_used);

    assert!(sess.active_turn.lock().await.is_none());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_goal_continuation_runs_again_after_no_tool_turn() -> anyhow::Result<()> {
    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::Goals)
            .expect("goal mode should be enableable in tests");
    });
    let test = builder.build(&server).await?;
    let _responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(
                    "call-create-goal",
                    "create_goal",
                    r#"{"objective":"write a benchmark note"}"#,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_assistant_message("msg-1", "Draft ready."),
                ev_completed("resp-2"),
            ]),
            sse(vec![
                ev_assistant_message("msg-2", "I am still working on the benchmark note."),
                ev_completed("resp-3"),
            ]),
            sse(vec![
                ev_response_created("resp-4"),
                ev_function_call(
                    "call-complete-goal",
                    "update_goal",
                    r#"{"status":"complete"}"#,
                ),
                ev_completed("resp-4"),
            ]),
            sse(vec![
                ev_assistant_message("msg-3", "Goal complete."),
                ev_completed("resp-5"),
            ]),
        ],
    )
    .await;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "write a benchmark note".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let mut completed_turns = 0;
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        loop {
            let event = test.codex.next_event().await?;
            if matches!(event.msg, EventMsg::TurnComplete(_)) {
                completed_turns += 1;
                if completed_turns == 3 {
                    return anyhow::Ok(());
                }
            }
        }
    })
    .await??;

    Ok(())
}

#[tokio::test]
async fn budget_limited_accounting_steers_active_turn_without_aborting() -> anyhow::Result<()> {
    let (sess, tc, rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    sess.set_thread_goal(
        tc.as_ref(),
        SetGoalRequest {
            objective: Some("Keep improving the benchmark".to_string()),
            status: None,
            token_budget: Some(Some(10)),
        },
    )
    .await?;
    sess.goal_runtime_apply(GoalRuntimeEvent::TurnStarted {
        turn_context: tc.as_ref(),
        token_usage: TokenUsage::default(),
    })
    .await?;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;
    while rx.try_recv().is_ok() {}

    set_total_token_usage(
        &sess,
        TokenUsage {
            input_tokens: 20,
            cached_input_tokens: 0,
            output_tokens: 5,
            reasoning_output_tokens: 0,
            total_tokens: 25,
        },
    )
    .await;

    sess.goal_runtime_apply(GoalRuntimeEvent::ToolCompleted {
        turn_context: tc.as_ref(),
        tool_name: "shell",
    })
    .await?;

    let pending_input = sess.get_pending_input().await;
    let [TurnInput::ResponseInputItem(ResponseInputItem::Message { role, content, .. })] =
        pending_input.as_slice()
    else {
        panic!("expected one budget-limit steering message, got {pending_input:#?}");
    };
    assert_eq!("developer", role);
    let [ContentItem::InputText { text }] = content.as_slice() else {
        panic!("expected one text span in budget-limit steering message, got {content:#?}");
    };
    assert!(text.contains("budget_limited"));
    assert!(text.to_lowercase().contains("wrap up this turn soon"));
    assert!(sess.active_turn.lock().await.is_some());
    while let Ok(event) = rx.try_recv() {
        assert!(
            !matches!(event.msg, EventMsg::TurnAborted(_)),
            "budget limit should steer the active turn instead of aborting it"
        );
    }

    let state_db = goal_test_state_db(sess.as_ref()).await?;
    let goal = state_db
        .thread_goals()
        .get_thread_goal(sess.conversation_id)
        .await?
        .expect("goal should remain persisted after accounting");
    assert_eq!(codex_state::ThreadGoalStatus::BudgetLimited, goal.status);
    assert_eq!(25, goal.tokens_used);

    set_total_token_usage(
        &sess,
        TokenUsage {
            input_tokens: 30,
            cached_input_tokens: 0,
            output_tokens: 10,
            reasoning_output_tokens: 0,
            total_tokens: 40,
        },
    )
    .await;
    sess.goal_runtime_apply(GoalRuntimeEvent::ToolCompletedGoal {
        turn_context: tc.as_ref(),
    })
    .await?;

    let goal = state_db
        .thread_goals()
        .get_thread_goal(sess.conversation_id)
        .await?
        .expect("goal should remain persisted after follow-up accounting");
    assert_eq!(codex_state::ThreadGoalStatus::BudgetLimited, goal.status);
    assert_eq!(40, goal.tokens_used);

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_goal_mutation_accounts_active_turn_before_status_change() -> anyhow::Result<()> {
    let (sess, tc, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    sess.set_thread_goal(
        tc.as_ref(),
        SetGoalRequest {
            objective: Some("Keep improving the benchmark".to_string()),
            status: None,
            token_budget: None,
        },
    )
    .await?;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;
    set_total_token_usage(&sess, post_goal_token_usage()).await;

    sess.goal_runtime_apply(GoalRuntimeEvent::ExternalMutationStarting)
        .await?;

    let state_db = goal_test_state_db(sess.as_ref()).await?;
    let goal = state_db
        .thread_goals()
        .get_thread_goal(sess.conversation_id)
        .await?
        .expect("goal should remain persisted");
    assert_eq!(70, goal.tokens_used);

    let previous_status = ExternalGoalPreviousStatus::from(&goal);
    let goal_id = goal.goal_id.clone();
    let updated_goal = state_db
        .thread_goals()
        .update_thread_goal(
            sess.conversation_id,
            codex_state::ThreadGoalUpdate {
                objective: None,
                status: Some(codex_state::ThreadGoalStatus::Complete),
                token_budget: None,
                expected_goal_id: Some(goal_id),
            },
        )
        .await?
        .expect("goal status update should succeed");
    sess.goal_runtime_apply(GoalRuntimeEvent::ExternalSet {
        external_set: ExternalGoalSet {
            goal: updated_goal,
            previous_status,
        },
    })
    .await?;

    assert!(sess.active_turn.lock().await.is_some());
    let goal = state_db
        .thread_goals()
        .get_thread_goal(sess.conversation_id)
        .await?
        .expect("goal should remain persisted");
    assert_eq!(codex_state::ThreadGoalStatus::Complete, goal.status);
    assert_eq!(70, goal.tokens_used);

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_active_goal_set_marks_current_turn_for_accounting() -> anyhow::Result<()> {
    let (sess, tc, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;
    set_total_token_usage(&sess, post_goal_token_usage()).await;

    let state_db = goal_test_state_db(sess.as_ref()).await?;
    let goal = state_db
        .thread_goals()
        .replace_thread_goal(
            sess.conversation_id,
            "Keep improving the benchmark",
            codex_state::ThreadGoalStatus::Active,
            /*token_budget*/ None,
        )
        .await?;
    sess.goal_runtime_apply(GoalRuntimeEvent::ExternalSet {
        external_set: ExternalGoalSet {
            goal,
            previous_status: ExternalGoalPreviousStatus::NewGoal,
        },
    })
    .await?;

    set_total_token_usage(
        &sess,
        TokenUsage {
            input_tokens: 65,
            cached_input_tokens: 10,
            output_tokens: 40,
            reasoning_output_tokens: 5,
            total_tokens: 110,
        },
    )
    .await;
    sess.goal_runtime_apply(GoalRuntimeEvent::ToolCompleted {
        turn_context: tc.as_ref(),
        tool_name: "shell",
    })
    .await?;

    let goal = state_db
        .thread_goals()
        .get_thread_goal(sess.conversation_id)
        .await?
        .expect("goal should remain persisted");
    assert_eq!(codex_state::ThreadGoalStatus::Active, goal.status);
    assert_eq!(25, goal.tokens_used);

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    Ok(())
}

#[tokio::test]
async fn queue_only_mailbox_mail_waits_for_next_turn_after_answer_boundary() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "late queue-only update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.defer_mailbox_delivery_to_next_turn(&tc.sub_id).await;
    sess.enqueue_mailbox_communication(communication.clone());

    assert!(
        !sess.has_pending_input().await,
        "queue-only mailbox mail should stay buffered once the current turn emitted its answer"
    );
    assert_eq!(sess.get_pending_input().await, Vec::new());

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    assert_eq!(
        sess.get_pending_input().await,
        vec![TurnInput::from(communication.to_response_input_item())],
    );
}

#[tokio::test]
async fn trigger_turn_mailbox_mail_waits_for_next_turn_after_answer_boundary() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.defer_mailbox_delivery_to_next_turn(&tc.sub_id).await;
    sess.enqueue_mailbox_communication(InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "late trigger update".to_string(),
        /*trigger_turn*/ true,
    ));

    assert!(
        !sess.has_pending_input().await,
        "trigger-turn mailbox mail should not extend the current turn after its answer boundary"
    );

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    assert!(sess.has_trigger_turn_mailbox_items().await);
}

#[tokio::test]
async fn steered_input_reopens_mailbox_delivery_for_current_turn() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "queued child update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.defer_mailbox_delivery_to_next_turn(&tc.sub_id).await;
    sess.enqueue_mailbox_communication(communication.clone());
    sess.steer_input(
        vec![UserInput::Text {
            text: "follow up".to_string(),
            text_elements: Vec::new(),
        }],
        Some(&tc.sub_id),
        /*responsesapi_client_metadata*/ None,
    )
    .await
    .expect("steered input should be accepted");

    assert_eq!(
        sess.get_pending_input().await,
        vec![
            TurnInput::from(ResponseInputItem::from(vec![UserInput::Text {
                text: "follow up".to_string(),
                text_elements: Vec::new(),
            }])),
            TurnInput::from(communication.to_response_input_item()),
        ],
    );
}

#[tokio::test]
async fn stale_defer_mailbox_delivery_does_not_override_steered_input() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "queued child update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.defer_mailbox_delivery_to_next_turn(&tc.sub_id).await;
    sess.enqueue_mailbox_communication(communication.clone());
    sess.steer_input(
        vec![UserInput::Text {
            text: "follow up".to_string(),
            text_elements: Vec::new(),
        }],
        Some(&tc.sub_id),
        /*responsesapi_client_metadata*/ None,
    )
    .await
    .expect("steered input should be accepted");

    sess.defer_mailbox_delivery_to_next_turn(&tc.sub_id).await;

    assert_eq!(
        sess.get_pending_input().await,
        vec![
            TurnInput::from(ResponseInputItem::from(vec![UserInput::Text {
                text: "follow up".to_string(),
                text_elements: Vec::new(),
            }])),
            TurnInput::from(communication.to_response_input_item()),
        ],
    );
}

#[tokio::test]
async fn tool_calls_reopen_mailbox_delivery_for_current_turn() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "queued child update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.defer_mailbox_delivery_to_next_turn(&tc.sub_id).await;
    sess.enqueue_mailbox_communication(communication.clone());

    let item = ResponseItem::FunctionCall {
        id: None,
        name: "test_tool".to_string(),
        namespace: None,
        arguments: "{}".to_string(),
        call_id: "call-1".to_string(),
    };
    let mut ctx = HandleOutputCtx {
        sess: Arc::clone(&sess),
        turn_context: Arc::clone(&tc),
        turn_store: Arc::clone(&tc.extension_data),
        tool_runtime: test_tool_runtime(Arc::clone(&sess), Arc::clone(&tc)),
        cancellation_token: CancellationToken::new(),
    };

    let output = handle_output_item_done(&mut ctx, item, /*previously_active_item*/ None)
        .await
        .expect("tool call should be handled");

    assert!(output.needs_follow_up);
    assert!(output.tool_future.is_some());
    assert_eq!(
        sess.get_pending_input().await,
        vec![TurnInput::from(communication.to_response_input_item())],
    );
}

#[tokio::test]
#[expect(
    clippy::await_holding_invalid_type,
    reason = "test builds a router from session-owned MCP manager state"
)]
async fn fatal_tool_error_stops_turn_and_reports_error() {
    let (session, turn_context, _rx) = make_session_and_context_with_rx().await;
    let tools = {
        session
            .services
            .mcp_connection_manager
            .read()
            .await
            .list_all_tools()
            .await
    };
    let deferred_mcp_tools = Some(tools.clone());
    let router = ToolRouter::from_turn_context(
        &turn_context,
        crate::tools::router::ToolRouterParams {
            deferred_mcp_tools,
            mcp_tools: Some(tools),
            discoverable_tools: None,
            dynamic_tools: turn_context.dynamic_tools.as_slice(),
            extension_tool_executors: Vec::new(),
        },
    );
    let item = ResponseItem::CustomToolCall {
        id: None,
        status: None,
        call_id: "call-1".to_string(),
        name: "shell".to_string(),
        input: "{}".to_string(),
    };

    let call = ToolRouter::build_tool_call(item.clone())
        .expect("build tool call")
        .expect("tool call present");
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
    let err = router
        .dispatch_tool_call_with_code_mode_result(
            Arc::clone(&session),
            Arc::clone(&turn_context),
            CancellationToken::new(),
            tracker,
            call,
            ToolCallSource::Direct,
        )
        .await
        .err()
        .expect("expected fatal error");

    match err {
        FunctionCallError::Fatal(message) => {
            assert_eq!(message, "tool shell invoked with incompatible payload");
        }
        other => panic!("expected FunctionCallError::Fatal, got {other:?}"),
    }
}

#[tokio::test]
async fn create_goal_tool_rejects_existing_goal() {
    let (session, turn_context, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
    let handler = CreateGoalHandler;

    handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker: Arc::clone(&tracker),
            call_id: "create-goal-1".to_string(),
            tool_name: plain_tool_name("create_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "objective": "Keep the watcher alive",
                    "token_budget": 123,
                })
                .to_string(),
            },
        })
        .await
        .expect("initial create_goal should succeed");

    let response = handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker,
            call_id: "create-goal-2".to_string(),
            tool_name: plain_tool_name("create_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "objective": "Replace the watcher",
                    "token_budget": 456,
                })
                .to_string(),
            },
        })
        .await;

    let Err(FunctionCallError::RespondToModel(output)) = response else {
        panic!("expected create_goal to reject an existing goal");
    };
    assert_eq!(
        output,
        "cannot create a new goal because this thread already has a goal; use update_goal only when the existing goal is complete"
    );

    let goal = session
        .get_thread_goal()
        .await
        .expect("read thread goal")
        .expect("goal should still exist");
    assert_eq!(goal.objective, "Keep the watcher alive");
    assert_eq!(goal.token_budget, Some(123));
}

#[tokio::test]
async fn update_goal_tool_rejects_pausing_goal() {
    let (session, turn_context, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
    let create_handler = CreateGoalHandler;
    let update_handler = UpdateGoalHandler;

    create_handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker: Arc::clone(&tracker),
            call_id: "create-goal".to_string(),
            tool_name: plain_tool_name("create_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "objective": "Keep the watcher alive",
                    "token_budget": 123,
                })
                .to_string(),
            },
        })
        .await
        .expect("initial create_goal should succeed");

    let response = update_handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker,
            call_id: "pause-goal".to_string(),
            tool_name: plain_tool_name("update_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "status": "paused",
                })
                .to_string(),
            },
        })
        .await;

    let Err(FunctionCallError::RespondToModel(output)) = response else {
        panic!("expected update_goal to reject pausing a goal");
    };
    assert_eq!(
        output,
        "update_goal can only mark the existing goal complete; pause, resume, and budget-limited status changes are controlled by the user or system"
    );

    let goal = session
        .get_thread_goal()
        .await
        .expect("read thread goal")
        .expect("goal should still exist");
    assert_eq!(goal.status, ThreadGoalStatus::Active);
}

#[tokio::test]
async fn update_goal_tool_marks_goal_complete() {
    let (session, turn_context, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
    let create_handler = CreateGoalHandler;
    let update_handler = UpdateGoalHandler;

    create_handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker: Arc::clone(&tracker),
            call_id: "create-goal".to_string(),
            tool_name: plain_tool_name("create_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "objective": "Keep the watcher alive",
                    "token_budget": 123,
                })
                .to_string(),
            },
        })
        .await
        .expect("initial create_goal should succeed");

    update_handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker,
            call_id: "complete-goal".to_string(),
            tool_name: plain_tool_name("update_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "status": "complete",
                })
                .to_string(),
            },
        })
        .await
        .expect("update_goal should mark the goal complete");

    let goal = session
        .get_thread_goal()
        .await
        .expect("read thread goal")
        .expect("goal should still exist");
    assert_eq!(goal.status, ThreadGoalStatus::Complete);
}
