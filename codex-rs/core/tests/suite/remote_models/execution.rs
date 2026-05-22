#![cfg(not(target_os = "windows"))]
#![allow(clippy::expect_used)]
#[path = "support.rs"]
mod support;

use support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_remote_model_uses_unified_exec() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::builder()
        .body_print_limit(BodyPrintLimit::Limited(80_000))
        .start()
        .await;

    let remote_model = ModelInfo {
        slug: REMOTE_MODEL_SLUG.to_string(),
        display_name: "Remote Test".to_string(),
        description: Some("A remote model that requires the test shell".to_string()),
        default_reasoning_level: Some(ReasoningEffort::Medium),
        supported_reasoning_levels: vec![ReasoningEffortPreset {
            effort: ReasoningEffort::Medium,
            description: ReasoningEffort::Medium.to_string(),
        }],
        shell_type: ConfigShellToolType::UnifiedExec,
        visibility: ModelVisibility::List,
        supported_in_api: true,
        input_modalities: default_input_modalities(),
        used_fallback_model_metadata: false,
        supports_search_tool: false,
        priority: 1,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        upgrade: None,
        base_instructions: "base instructions".to_string(),
        model_messages: None,
        supports_reasoning_summaries: false,
        default_reasoning_summary: ReasoningSummary::Auto,
        support_verbosity: false,
        default_verbosity: None,
        availability_nux: None,
        apply_patch_tool_type: None,
        web_search_tool_type: Default::default(),
        truncation_policy: TruncationPolicyConfig::bytes(/*limit*/ 10_000),
        supports_parallel_tool_calls: false,
        supports_image_detail_original: false,
        context_window: Some(272_000),
        max_context_window: None,
        auto_compact_token_limit: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
    };

    let models_mock = mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model],
        },
    )
    .await;

    let mut builder = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(|config| {
            config.model = Some("gpt-5.4".to_string());
        });
    let TestCodex {
        codex,
        cwd,
        config,
        thread_manager,
        ..
    } = builder.build(&server).await?;

    let models_manager = thread_manager.get_models_manager();
    let available_model = wait_for_model_available(&models_manager, REMOTE_MODEL_SLUG).await;

    assert_eq!(available_model.model, REMOTE_MODEL_SLUG);

    let requests = models_mock.requests();
    assert_eq!(
        requests.len(),
        1,
        "expected a single /models refresh request for the remote models feature"
    );
    assert_eq!(requests[0].url.path(), "/v1/models");

    let model_info = models_manager
        .get_model_info(REMOTE_MODEL_SLUG, &config.to_models_manager_config())
        .await;
    assert_eq!(model_info.shell_type, ConfigShellToolType::UnifiedExec);

    codex
        .submit(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: None,
            approvals_reviewer: None,
            sandbox_policy: None,
            permission_profile: None,
            windows_sandbox_level: None,
            model: Some(REMOTE_MODEL_SLUG.to_string()),
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    let call_id = "call";
    let args = json!({
        "cmd": "/bin/echo call",
        "yield_time_ms": 250,
    });
    let responses = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_response_created("resp-2"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    ];
    mount_sse_sequence(&server, responses).await;

    let cwd_path = cwd.path().to_path_buf();
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::Disabled, cwd_path.as_path());
    codex
        .submit(Op::UserTurn {
            items: vec![UserInput::Text {
                text: "run call".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd_path,
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy,
            permission_profile,
            model: REMOTE_MODEL_SLUG.to_string(),
            effort: None,
            summary: Some(ReasoningSummary::Auto),
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
            environments: None,
        })
        .await?;

    let begin_event = wait_for_event_match(&codex, |msg| match msg {
        EventMsg::ExecCommandBegin(event) if event.call_id == call_id => Some(event.clone()),
        _ => None,
    })
    .await;

    assert_eq!(begin_event.source, ExecCommandSource::UnifiedExecStartup);

    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_truncation_policy_without_override_preserves_remote() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::builder()
        .body_print_limit(BodyPrintLimit::Limited(80_000))
        .start()
        .await;

    let slug = "codex-test-truncation-policy";
    let remote_model = test_remote_model_with_policy(
        slug,
        ModelVisibility::List,
        /*priority*/ 1,
        TruncationPolicyConfig::bytes(/*limit*/ 12_000),
    );
    mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model],
        },
    )
    .await;

    let mut builder = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(|config| {
            config.model = Some("gpt-5.4".to_string());
        });
    let test = builder.build(&server).await?;

    let models_manager = test.thread_manager.get_models_manager();
    wait_for_model_available(&models_manager, slug).await;

    let model_info = models_manager
        .get_model_info(slug, &test.config.to_models_manager_config())
        .await;
    assert_eq!(
        model_info.truncation_policy,
        TruncationPolicyConfig::bytes(/*limit*/ 12_000)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_truncation_policy_with_tool_output_override() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::builder()
        .body_print_limit(BodyPrintLimit::Limited(80_000))
        .start()
        .await;

    let slug = "codex-test-truncation-override";
    let remote_model = test_remote_model_with_policy(
        slug,
        ModelVisibility::List,
        /*priority*/ 1,
        TruncationPolicyConfig::bytes(/*limit*/ 10_000),
    );
    mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model],
        },
    )
    .await;

    let mut builder = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(|config| {
            config.model = Some("gpt-5.4".to_string());
            config.tool_output_token_limit = Some(50);
        });
    let test = builder.build(&server).await?;

    let models_manager = test.thread_manager.get_models_manager();
    wait_for_model_available(&models_manager, slug).await;

    let model_info = models_manager
        .get_model_info(slug, &test.config.to_models_manager_config())
        .await;
    assert_eq!(
        model_info.truncation_policy,
        TruncationPolicyConfig::bytes(/*limit*/ 200)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_apply_remote_base_instructions() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::builder()
        .body_print_limit(BodyPrintLimit::Limited(80_000))
        .start()
        .await;

    let model = "test-gpt-5-remote";

    let remote_base = "Use the remote base instructions only.";
    let remote_model = ModelInfo {
        slug: model.to_string(),
        display_name: "Parallel Remote".to_string(),
        description: Some("A remote model with custom instructions".to_string()),
        default_reasoning_level: Some(ReasoningEffort::Medium),
        supported_reasoning_levels: vec![ReasoningEffortPreset {
            effort: ReasoningEffort::Medium,
            description: ReasoningEffort::Medium.to_string(),
        }],
        shell_type: ConfigShellToolType::ShellCommand,
        visibility: ModelVisibility::List,
        supported_in_api: true,
        input_modalities: default_input_modalities(),
        used_fallback_model_metadata: false,
        supports_search_tool: false,
        priority: 1,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        upgrade: None,
        base_instructions: remote_base.to_string(),
        model_messages: None,
        supports_reasoning_summaries: false,
        default_reasoning_summary: ReasoningSummary::Auto,
        support_verbosity: false,
        default_verbosity: None,
        availability_nux: None,
        apply_patch_tool_type: None,
        web_search_tool_type: Default::default(),
        truncation_policy: TruncationPolicyConfig::bytes(/*limit*/ 10_000),
        supports_parallel_tool_calls: false,
        supports_image_detail_original: false,
        context_window: Some(272_000),
        max_context_window: None,
        auto_compact_token_limit: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
    };
    mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model],
        },
    )
    .await;

    let response_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(|config| {
            config.model = Some("gpt-5.2".to_string());
        });
    let TestCodex {
        codex,
        cwd,
        config,
        thread_manager,
        ..
    } = builder.build(&server).await?;

    let models_manager = thread_manager.get_models_manager();
    wait_for_model_available(&models_manager, model).await;

    codex
        .submit(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: None,
            approvals_reviewer: None,
            sandbox_policy: None,
            permission_profile: None,
            windows_sandbox_level: None,
            model: Some(model.to_string()),
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    let cwd_path = cwd.path().to_path_buf();
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::Disabled, cwd_path.as_path());
    codex
        .submit(Op::UserTurn {
            items: vec![UserInput::Text {
                text: "hello remote".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: cwd_path,
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy,
            permission_profile,
            model: model.to_string(),
            effort: None,
            summary: Some(ReasoningSummary::Auto),
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
            environments: None,
        })
        .await?;

    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let base_model_info = models_manager
        .get_model_info("gpt-5.2", &config.to_models_manager_config())
        .await;
    let body = response_mock.single_request().body_json();
    let instructions = body["instructions"].as_str().unwrap();
    assert_eq!(instructions, base_model_info.base_instructions);

    Ok(())
}
