#![cfg(not(target_os = "windows"))]
#![allow(clippy::expect_used)]
#[path = "remote_models/support.rs"]
mod support;

use support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_get_model_info_uses_longest_matching_prefix() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let generic = test_remote_model_with_policy(
        "gpt-5.3",
        ModelVisibility::List,
        /*priority*/ 1_000,
        TruncationPolicyConfig::bytes(/*limit*/ 10_000),
    );
    let specific = test_remote_model_with_policy(
        "gpt-5.3-codex",
        ModelVisibility::List,
        /*priority*/ 1_000,
        TruncationPolicyConfig::bytes(/*limit*/ 10_000),
    );
    let specific = ModelInfo {
        display_name: "GPT 5.3 Codex".to_string(),
        base_instructions: "use specific prefix".to_string(),
        ..specific
    };
    let generic = ModelInfo {
        display_name: "GPT 5.3".to_string(),
        base_instructions: "use generic prefix".to_string(),
        ..generic
    };
    mount_models_once(
        &server,
        ModelsResponse {
            models: vec![generic.clone(), specific.clone()],
        },
    )
    .await;

    let codex_home = TempDir::new()?;
    let config = load_default_config_for_test(&codex_home).await;

    let auth = CodexAuth::create_dummy_chatgpt_auth_for_testing();
    let provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers(/* openai_base_url */ /*openai_base_url*/ None)["openai"].clone()
    };
    let manager = codex_core::test_support::models_manager_with_provider(
        codex_home.path().to_path_buf(),
        codex_core::test_support::auth_manager_from_auth(auth),
        provider,
    );

    manager.list_models(RefreshStrategy::OnlineIfUncached).await;

    let model_info = manager
        .get_model_info("gpt-5.3-codex-test", &config.to_models_manager_config())
        .await;

    assert_eq!(model_info.slug, "gpt-5.3-codex-test");
    assert_eq!(model_info.base_instructions, specific.base_instructions);

    Ok(())
}

/// Scenario: the model advertises a default 273k context window and a 400k max
/// context window, and the user explicitly configures 1M. This verifies the
/// runtime turn clamps the override to the advertised max window.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_config_context_window_override_clamps_to_max_context_window() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let requested_model = "gpt-5.4-test";
    let mut remote_model =
        test_remote_model("gpt-5.4", ModelVisibility::List, /*priority*/ 1_000);
    remote_model.context_window = Some(273_000);
    remote_model.max_context_window = Some(400_000);
    remote_model.effective_context_window_percent = 100;
    mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model],
        },
    )
    .await;
    mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;

    let TestCodex {
        codex, cwd, config, ..
    } = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(|config| {
            config.model = Some(requested_model.to_string());
            config.model_context_window = Some(1_000_000);
        })
        .build(&server)
        .await?;

    codex
        .submit(Op::UserTurn {
            items: vec![UserInput::Text {
                text: "check context window".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd.path().to_path_buf(),
            approval_policy: config.permissions.approval_policy.value(),
            approvals_reviewer: None,
            sandbox_policy: config.legacy_sandbox_policy(),
            model: requested_model.to_string(),
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            permission_profile: None,
            personality: None,
            environments: None,
        })
        .await?;

    let turn_started_event = wait_for_event(&codex, |event| {
        matches!(
            event,
            EventMsg::TurnStarted(started)
                if started.model_context_window == Some(400_000)
        )
    })
    .await;
    let EventMsg::TurnStarted(turn_started) = turn_started_event else {
        unreachable!("wait_for_event returned unexpected event");
    };

    assert_eq!(turn_started.model_context_window, Some(400_000));

    Ok(())
}

/// Scenario: the user explicitly configures a context window above the model's
/// max_context_window. This verifies the runtime window is clamped to the max
/// instead of using the oversized config value.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_config_override_above_max_uses_max_context_window() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let requested_model = "gpt-5.4-test";
    let mut remote_model =
        test_remote_model("gpt-5.4", ModelVisibility::List, /*priority*/ 1_000);
    remote_model.context_window = Some(273_000);
    remote_model.max_context_window = Some(400_000);
    remote_model.effective_context_window_percent = 100;
    mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model],
        },
    )
    .await;
    mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;

    let TestCodex {
        codex, cwd, config, ..
    } = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(|config| {
            config.model = Some(requested_model.to_string());
            config.model_context_window = Some(500_000);
        })
        .build(&server)
        .await?;

    codex
        .submit(Op::UserTurn {
            items: vec![UserInput::Text {
                text: "check context window".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd.path().to_path_buf(),
            approval_policy: config.permissions.approval_policy.value(),
            approvals_reviewer: None,
            sandbox_policy: config.legacy_sandbox_policy(),
            model: requested_model.to_string(),
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            permission_profile: None,
            personality: None,
            environments: None,
        })
        .await?;

    let turn_started_event = wait_for_event(&codex, |event| {
        matches!(
            event,
            EventMsg::TurnStarted(started)
                if started.model_context_window == Some(400_000)
        )
    })
    .await;
    let EventMsg::TurnStarted(turn_started) = turn_started_event else {
        unreachable!("wait_for_event returned unexpected event");
    };

    assert_eq!(turn_started.model_context_window, Some(400_000));

    Ok(())
}

/// Scenario: model metadata includes both context_window and max_context_window,
/// but the user did not configure an override. This verifies the runtime keeps
/// using the model's default context_window in the no-override path.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_use_context_window_when_config_override_is_absent() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let requested_model = "gpt-5.4-test";
    let mut remote_model =
        test_remote_model("gpt-5.4", ModelVisibility::List, /*priority*/ 1_000);
    remote_model.context_window = Some(273_000);
    remote_model.max_context_window = Some(400_000);
    remote_model.effective_context_window_percent = 100;
    mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model],
        },
    )
    .await;
    mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;

    let TestCodex {
        codex, cwd, config, ..
    } = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(|config| {
            config.model = Some(requested_model.to_string());
        })
        .build(&server)
        .await?;

    codex
        .submit(Op::UserTurn {
            items: vec![UserInput::Text {
                text: "check context window".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd.path().to_path_buf(),
            approval_policy: config.permissions.approval_policy.value(),
            approvals_reviewer: None,
            sandbox_policy: config.legacy_sandbox_policy(),
            model: requested_model.to_string(),
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            permission_profile: None,
            personality: None,
            environments: None,
        })
        .await?;

    let turn_started_event = wait_for_event(&codex, |event| {
        matches!(
            event,
            EventMsg::TurnStarted(started)
                if started.model_context_window == Some(273_000)
        )
    })
    .await;
    let EventMsg::TurnStarted(turn_started) = turn_started_event else {
        unreachable!("wait_for_event returned unexpected event");
    };

    assert_eq!(turn_started.model_context_window, Some(273_000));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_long_model_slug_is_sent_with_high_reasoning() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let requested_model = "gpt-5.3-codex-test";
    let prefix_model = "gpt-5.3-codex";
    let mut remote_model = test_remote_model_with_policy(
        prefix_model,
        ModelVisibility::List,
        /*priority*/ 1_000,
        TruncationPolicyConfig::bytes(/*limit*/ 10_000),
    );
    remote_model.default_reasoning_level = Some(ReasoningEffort::High);
    remote_model.supported_reasoning_levels = vec![
        ReasoningEffortPreset {
            effort: ReasoningEffort::Medium,
            description: ReasoningEffort::Medium.to_string(),
        },
        ReasoningEffortPreset {
            effort: ReasoningEffort::High,
            description: ReasoningEffort::High.to_string(),
        },
    ];
    remote_model.supports_reasoning_summaries = true;
    remote_model.default_reasoning_summary = ReasoningSummary::Detailed;
    mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model],
        },
    )
    .await;

    let response_mock = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;

    let TestCodex {
        codex, cwd, config, ..
    } = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(|config| {
            config.model = Some(requested_model.to_string());
        })
        .build(&server)
        .await?;

    codex
        .submit(Op::UserTurn {
            items: vec![UserInput::Text {
                text: "check model slug".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd.path().to_path_buf(),
            approval_policy: config.permissions.approval_policy.value(),
            approvals_reviewer: None,
            sandbox_policy: config.legacy_sandbox_policy(),
            permission_profile: None,
            model: requested_model.to_string(),
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
            environments: None,
        })
        .await?;

    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let request = response_mock.single_request();
    let body = request.body_json();
    let reasoning_effort = body
        .get("reasoning")
        .and_then(|reasoning| reasoning.get("effort"))
        .and_then(|value| value.as_str());
    let reasoning_summary = body
        .get("reasoning")
        .and_then(|reasoning| reasoning.get("summary"))
        .and_then(|value| value.as_str());
    assert_eq!(body["model"].as_str(), Some(requested_model));
    assert_eq!(reasoning_effort, Some("high"));
    assert_eq!(reasoning_summary, Some("detailed"));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn namespaced_model_slug_uses_catalog_metadata_without_fallback_warning() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let requested_model = "custom/gpt-5.2-codex";
    let response_mock = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;

    let TestCodex {
        codex, cwd, config, ..
    } = test_codex()
        .with_model(requested_model)
        .build(&server)
        .await?;

    codex
        .submit(Op::UserTurn {
            items: vec![UserInput::Text {
                text: "check namespaced model metadata".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd.path().to_path_buf(),
            approval_policy: config.permissions.approval_policy.value(),
            approvals_reviewer: None,
            sandbox_policy: config.legacy_sandbox_policy(),
            permission_profile: None,
            model: requested_model.to_string(),
            effort: None,
            summary: Some(
                config
                    .model_reasoning_summary
                    .unwrap_or(ReasoningSummary::Auto),
            ),
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
            environments: None,
        })
        .await?;

    let mut fallback_warning_count = 0;
    loop {
        let event = wait_for_event(&codex, |_| true).await;
        match event {
            EventMsg::Warning(warning)
                if warning.message.contains("Defaulting to fallback metadata") =>
            {
                fallback_warning_count += 1;
            }
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    let body = response_mock.single_request().body_json();
    assert_eq!(body["model"].as_str(), Some(requested_model));
    assert_eq!(fallback_warning_count, 0);

    Ok(())
}
