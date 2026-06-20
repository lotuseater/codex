use super::*;

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
async fn build_initial_context_adds_default_multi_agent_v2_root_usage_hint() {
    let (session, turn_context, _rx_event) = make_session_and_context_with_auth_and_config_and_rx(
        CodexAuth::from_api_key("Test API Key"),
        Vec::new(),
        |config| {
            let _ = config.features.enable(Feature::MultiAgentV2);
        },
    )
    .await;

    let initial_context = session.build_initial_context(turn_context.as_ref()).await;

    let developer_messages = developer_message_texts(&initial_context);
    assert!(
        developer_messages.iter().flatten().any(|text| text
            .contains(codex_agent_policy::DEFAULT_PLAN_TOKEN_ECONOMY_DELEGATION_K_PROMPT_TEXT)),
        "expected default root usage hint to include literal {}, got {developer_messages:?}",
        codex_agent_policy::DEFAULT_PLAN_TOKEN_ECONOMY_DELEGATION_K_PROMPT_TEXT
    );
}

#[tokio::test]
async fn build_initial_context_adds_default_multi_agent_v2_subagent_usage_hint() {
    let (session, mut turn_context, _rx_event) =
        make_session_and_context_with_auth_and_config_and_rx(
            CodexAuth::from_api_key("Test API Key"),
            Vec::new(),
            |config| {
                let _ = config.features.enable(Feature::MultiAgentV2);
            },
        )
        .await;
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
        developer_messages.iter().flatten().any(|text| text
            .contains(codex_agent_policy::DEFAULT_PLAN_TOKEN_ECONOMY_DELEGATION_K_PROMPT_TEXT)),
        "expected default subagent usage hint to include literal {}, got {developer_messages:?}",
        codex_agent_policy::DEFAULT_PLAN_TOKEN_ECONOMY_DELEGATION_K_PROMPT_TEXT
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
