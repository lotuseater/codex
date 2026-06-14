use super::*;

const ACTION_OPTIMIZATION_OPEN_TAG: &str = "<action_optimization_instructions>";

fn enable_action_optimization(
    turn_context: &mut TurnContext,
    mode: crate::config::ActionOptimizationInstructionsMode,
    max_tokens: usize,
) {
    let mut config = turn_context.config.as_ref().clone();
    config.action_optimization_instructions = crate::config::ActionOptimizationInstructionsConfig {
        mode,
        variant: crate::config::ActionOptimizationInstructionsVariant::ActionRouteSelection,
        max_tokens,
    };
    turn_context.config = std::sync::Arc::new(config);
}

fn disable_action_optimization(turn_context: &mut TurnContext) {
    let mut config = turn_context.config.as_ref().clone();
    config.action_optimization_instructions.mode =
        crate::config::ActionOptimizationInstructionsMode::Off;
    turn_context.config = std::sync::Arc::new(config);
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
async fn build_initial_context_includes_action_optimization_by_default_on_first_turn() {
    let (session, turn_context) = make_session_and_context().await;

    let context = session.build_initial_context(&turn_context).await;
    let developer_text = developer_input_texts(&context).join("\n");

    assert!(developer_text.contains(ACTION_OPTIMIZATION_OPEN_TAG));
    assert!(developer_text.contains("Keep simple tasks simple"));
    assert!(developer_text.contains("Select the lightest route"));
}

#[tokio::test]
async fn build_initial_context_omits_action_optimization_when_explicitly_off() {
    let (session, mut turn_context) = make_session_and_context().await;
    disable_action_optimization(&mut turn_context);

    let context = session.build_initial_context(&turn_context).await;
    let developer_text = developer_input_texts(&context).join("\n");

    assert!(!developer_text.contains(ACTION_OPTIMIZATION_OPEN_TAG));
    assert!(!developer_text.contains("Keep simple tasks simple"));
}

#[tokio::test]
async fn build_initial_context_includes_action_optimization_always_before_batch() {
    let (session, mut turn_context) = make_session_and_context().await;
    enable_action_optimization(
        &mut turn_context,
        crate::config::ActionOptimizationInstructionsMode::Always,
        120,
    );
    turn_context.collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model: turn_context.model_info.slug.clone(),
            reasoning_effort: None,
            developer_instructions: Some("COLLABORATION MODE INSTRUCTIONS".to_string()),
        },
    };
    turn_context.tools_config.workflow_batch_enabled = true;
    turn_context.tools_config.environment_mode = ToolEnvironmentMode::Single;

    let context = session.build_initial_context(&turn_context).await;
    let developer_text = developer_input_texts(&context).join("\n");

    let collaboration_index = developer_text
        .find("COLLABORATION MODE INSTRUCTIONS")
        .expect("collaboration instructions should render");
    let action_index = developer_text
        .find(ACTION_OPTIMIZATION_OPEN_TAG)
        .expect("action optimization instructions should render");
    let batch_index = developer_text
        .find("<batch_mini_programming_instructions>")
        .expect("batch mini-programming instructions should render");
    assert!(collaboration_index < action_index);
    assert!(action_index < batch_index);
    assert!(developer_text.contains("Keep simple tasks simple"));
    assert!(developer_text.contains("workflow_batch for repetitive deterministic"));
    assert!(!developer_text.contains("exactly one of `spec` or `spec_path`"));
}

#[tokio::test]
async fn build_initial_context_respects_action_optimization_plan_mode() {
    let (session, mut turn_context) = make_session_and_context().await;
    enable_action_optimization(
        &mut turn_context,
        crate::config::ActionOptimizationInstructionsMode::Plan,
        120,
    );

    let default_context = session.build_initial_context(&turn_context).await;
    let default_developer_text = developer_input_texts(&default_context).join("\n");
    assert!(!default_developer_text.contains(ACTION_OPTIMIZATION_OPEN_TAG));

    turn_context.collaboration_mode = CollaborationMode {
        mode: ModeKind::Plan,
        settings: Settings {
            model: turn_context.model_info.slug.clone(),
            reasoning_effort: None,
            developer_instructions: None,
        },
    };
    let plan_context = session.build_initial_context(&turn_context).await;
    let plan_developer_text = developer_input_texts(&plan_context).join("\n");
    assert!(plan_developer_text.contains(ACTION_OPTIMIZATION_OPEN_TAG));
}

#[tokio::test]
async fn build_initial_context_respects_action_optimization_first_turn_mode() {
    let (session, mut turn_context) = make_session_and_context().await;
    enable_action_optimization(
        &mut turn_context,
        crate::config::ActionOptimizationInstructionsMode::FirstTurn,
        120,
    );

    let initial_context = session.build_initial_context(&turn_context).await;
    let initial_developer_text = developer_input_texts(&initial_context).join("\n");
    assert!(initial_developer_text.contains(ACTION_OPTIMIZATION_OPEN_TAG));

    let previous_context_item = turn_context.to_turn_context_item();
    {
        let mut state = session.state.lock().await;
        state.set_reference_context_item(Some(previous_context_item));
    }
    let resumed_context = session.build_initial_context(&turn_context).await;
    let resumed_developer_text = developer_input_texts(&resumed_context).join("\n");
    assert!(!resumed_developer_text.contains(ACTION_OPTIMIZATION_OPEN_TAG));
}

#[tokio::test]
async fn build_initial_context_respects_action_optimization_max_tokens() {
    let (session, mut turn_context) = make_session_and_context().await;
    enable_action_optimization(
        &mut turn_context,
        crate::config::ActionOptimizationInstructionsMode::Always,
        8,
    );

    let context = session.build_initial_context(&turn_context).await;
    let developer_text = developer_input_texts(&context).join("\n");

    assert!(developer_text.contains("Keep simple tasks simple: answer directly when enough"));
    assert!(!developer_text.contains("evidence is present"));
}

#[tokio::test]
async fn build_initial_context_omits_reserved_action_optimization_tool_turn_mode() {
    let (session, mut turn_context) = make_session_and_context().await;
    enable_action_optimization(
        &mut turn_context,
        crate::config::ActionOptimizationInstructionsMode::ToolTurn,
        120,
    );

    let context = session.build_initial_context(&turn_context).await;
    let developer_text = developer_input_texts(&context).join("\n");

    assert!(!developer_text.contains(ACTION_OPTIMIZATION_OPEN_TAG));
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
    assert!(
        developer_text
            .contains("several deterministic local file/JSON reads, bounded scans, transforms")
    );
    assert!(developer_text.contains("one diagnosable batch beats repeated tool calls"));
    assert!(developer_text.contains("`spec` is `{\"steps\":[...]}`"));
    assert!(developer_text.contains("exactly one of `spec` or `spec_path`"));
    assert!(developer_text.contains("Python for richer algorithms or reusable prototypes"));
    assert!(developer_text.contains("Keep simple tasks simple"));
    assert!(developer_text.contains("would hide which file, query, or step failed"));
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
