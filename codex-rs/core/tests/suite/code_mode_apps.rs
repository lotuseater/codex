#![allow(dead_code, unused_imports, clippy::expect_used, clippy::unwrap_used)]

include!("code_mode_shared.rs");

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_only_guides_all_tools_search_and_calls_deferred_app_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let resp_mock = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call(
                "call-1",
                "exec",
                r#"
const tool = ALL_TOOLS.find(
  ({ name }) => name === "mcp__codex_apps__calendar_timezone_option_99"
);
if (!tool) {
  text(JSON.stringify({ found: false }));
} else {
  const result = await tools[tool.name]({ timezone: "UTC" });
  text(JSON.stringify({
    found: true,
    isError: Boolean(result.isError),
    text: result.content?.[0]?.text ?? "",
  }));
}
"#,
            ),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let follow_up_mock = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    let apps_base_url = apps_server.chatgpt_base_url.clone();
    let mut builder = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(move |config| {
            config
                .features
                .enable(Feature::Apps)
                .expect("test config should allow feature update");
            config
                .features
                .enable(Feature::ToolSearch)
                .expect("test config should allow feature update");
            config
                .features
                .enable(Feature::CodeMode)
                .expect("test config should allow feature update");
            config
                .features
                .enable(Feature::CodeModeOnly)
                .expect("test config should allow feature update");
            let mut model_catalog = bundled_models_response()
                .unwrap_or_else(|err| panic!("bundled models.json should parse: {err}"));
            let model = model_catalog
                .models
                .iter_mut()
                .find(|model| model.slug == "gpt-5.4")
                .expect("gpt-5.4 exists in bundled models.json");
            config.chatgpt_base_url = apps_base_url;
            config.model = Some("gpt-5.4".to_string());
            model.supports_search_tool = true;
            config.model_catalog = Some(model_catalog);
        });
    let test = builder.build(&server).await?;
    test.submit_turn("inspect tools in code mode only").await?;

    let first_body = resp_mock.single_request().body_json();
    assert_eq!(
        tool_names(&first_body),
        vec!["exec".to_string(), "wait".to_string()]
    );

    let exec_description = first_body
        .get("tools")
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter().find_map(|tool| {
                if tool
                    .get("name")
                    .or_else(|| tool.get("type"))
                    .and_then(Value::as_str)
                    == Some("exec")
                {
                    tool.get("description").and_then(Value::as_str)
                } else {
                    None
                }
            })
        })
        .expect("exec description should be present");
    assert!(exec_description.contains("filter `ALL_TOOLS` by `name` and `description`"));
    assert!(!exec_description.contains("calendar_timezone_option_99"));

    let request = follow_up_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&request, "call-1");
    assert_ne!(
        success,
        Some(false),
        "code_mode_only deferred app tool call failed unexpectedly: {output}"
    );
    let parsed: Value = serde_json::from_str(&output)?;
    assert_eq!(
        parsed,
        serde_json::json!({
            "found": true,
            "isError": false,
            "text": "called calendar_timezone_option_99 for  at  with ",
        })
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_call_hidden_dynamic_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex().with_config(move |config| {
        let _ = config.features.enable(Feature::CodeMode);
    });
    let base_test = builder.build(&server).await?;
    let new_thread = base_test
        .thread_manager
        .start_thread_with_tools(
            base_test.config.clone(),
            vec![DynamicToolSpec {
                namespace: Some("codex_app".to_string()),
                name: "hidden_dynamic_tool".to_string(),
                description: "A hidden dynamic tool.".to_string(),
                input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "city": { "type": "string" }
                        },
                    "required": ["city"],
                    "additionalProperties": false,
                }),
                defer_loading: true,
            }],
            /*persist_extended_history*/ false,
        )
        .await?;
    let mut test = base_test;
    test.codex = new_thread.thread;
    test.session_configured = new_thread.session_configured;

    let code = r#"
const tool = ALL_TOOLS.find(({ name }) => name === "codex_app_hidden_dynamic_tool");
const out = await tools.codex_app_hidden_dynamic_tool({ city: "Paris" });
text(
  JSON.stringify({
    name: tool?.name ?? null,
    description: tool?.description ?? null,
    out,
  })
);
"#;

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call("call-1", "exec", code),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let second_mock = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    let cwd = test.cwd.path().to_path_buf();
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::Disabled, cwd.as_path());

    test.codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "use exec to inspect and call hidden tools".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd,
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy,
            permission_profile,
            model: test.session_configured.model.clone(),
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    let turn_id = wait_for_event_match(&test.codex, |event| match event {
        EventMsg::TurnStarted(event) => Some(event.turn_id.clone()),
        _ => None,
    })
    .await;
    let request = wait_for_event_match(&test.codex, |event| match event {
        EventMsg::DynamicToolCallRequest(request) => Some(request.clone()),
        _ => None,
    })
    .await;
    assert_eq!(request.namespace.as_deref(), Some("codex_app"));
    assert_eq!(request.tool, "hidden_dynamic_tool");
    assert_eq!(request.arguments, serde_json::json!({ "city": "Paris" }));
    test.codex
        .submit(Op::DynamicToolResponse {
            id: request.call_id,
            response: DynamicToolResponse {
                content_items: vec![DynamicToolCallOutputContentItem::InputText {
                    text: "hidden-ok".to_string(),
                }],
                success: true,
            },
        })
        .await?;
    wait_for_event(&test.codex, |event| match event {
        EventMsg::TurnComplete(event) => event.turn_id == turn_id,
        _ => false,
    })
    .await;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec hidden dynamic tool call failed unexpectedly: {output}"
    );

    let parsed: Value = serde_json::from_str(
        &custom_tool_output_last_non_empty_text(&req, "call-1")
            .expect("exec hidden dynamic tool lookup should emit JSON"),
    )?;
    assert_eq!(
        parsed.get("name"),
        Some(&Value::String("codex_app_hidden_dynamic_tool".to_string()))
    );
    assert_eq!(
        parsed.get("out"),
        Some(&Value::String("hidden-ok".to_string()))
    );
    assert!(
        parsed
            .get("description")
            .and_then(Value::as_str)
            .is_some_and(|description| {
                description.contains("A hidden dynamic tool.")
                    && description.contains("declare const tools:")
                    && description.contains("codex_app_hidden_dynamic_tool(args:")
            })
    );

    Ok(())
}
