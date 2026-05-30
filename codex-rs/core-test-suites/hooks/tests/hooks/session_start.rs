use super::common::*;

#[tokio::test]
async fn session_start_hook_sees_materialized_transcript_path() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let _response = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "hello from the reef"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) = write_session_start_hook_recording_transcript(home) {
                panic!("failed to write session start hook test fixture: {error}");
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    test.submit_turn("hello").await?;

    let hook_inputs = read_session_start_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(
        hook_inputs[0]
            .get("transcript_path")
            .and_then(Value::as_str)
            .map(str::is_empty),
        Some(false)
    );
    assert_eq!(hook_inputs[0].get("exists"), Some(&Value::Bool(true)));

    Ok(())
}

#[tokio::test]
async fn session_start_runs_before_user_prompt_submit_on_first_turn() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let _response = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "hello after hooks"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) = write_session_start_and_user_prompt_submit_order_hooks(home) {
                panic!("failed to write hook ordering fixtures: {error}");
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    test.submit_turn("hello").await?;

    let hook_inputs = read_hook_order_inputs(test.codex_home_path())?;
    assert_eq!(
        hook_inputs
            .iter()
            .map(|input| input["hook_event_name"]
                .as_str()
                .expect("hook input event name"))
            .collect::<Vec<_>>(),
        vec!["SessionStart", "UserPromptSubmit"],
    );
    assert_eq!(
        hook_inputs[0].get("source").and_then(Value::as_str),
        Some("startup")
    );
    assert_eq!(
        hook_inputs[1].get("prompt").and_then(Value::as_str),
        Some("hello")
    );

    Ok(())
}

#[tokio::test]
async fn session_start_hook_spills_large_additional_context() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "hello from the reef"),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let additional_context = "remember the reef ".repeat(800);

    let mut builder = test_codex()
        .with_pre_build_hook({
            let additional_context = additional_context.clone();
            move |home| {
                if let Err(error) = write_session_start_hook_with_context(home, &additional_context)
                {
                    panic!("failed to write session start hook test fixture: {error}");
                }
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    test.submit_turn("hello").await?;

    let request = response.single_request();
    let developer_messages = request.message_input_texts("developer");
    let developer_message = developer_messages
        .iter()
        .find(|message| spilled_hook_output_path(message).is_some())
        .context("spilled developer hook message")?;
    assert!(developer_message.contains("tokens truncated"));
    let path = spilled_hook_output_path(developer_message).context("spill path")?;
    assert_eq!(fs::read_to_string(path)?, additional_context);

    Ok(())
}

#[tokio::test]
async fn compact_session_start_hook_records_additional_context_for_next_turn() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let request_log = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_assistant_message("msg-1", "hello before compact"),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-2", "summary after compact"),
                ev_completed("resp-2"),
            ]),
            sse(vec![
                ev_response_created("resp-3"),
                ev_assistant_message("msg-3", "hello after compact"),
                ev_completed("resp-3"),
            ]),
        ],
    )
    .await;
    let additional_context = "remember the compacted reef";
    let model_provider = non_openai_model_provider(&server);

    let mut builder = test_codex()
        .with_pre_build_hook(move |home| {
            if let Err(error) =
                write_compact_session_start_hook_with_context(home, additional_context)
            {
                panic!("failed to write compact session start hook fixture: {error}");
            }
        })
        .with_config(move |config| {
            config.model_provider = model_provider;
            trust_discovered_hooks(config);
        });
    let test = builder.build(&server).await?;

    test.submit_turn("hello before compact").await?;
    test.codex.submit(Op::Compact).await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    test.submit_turn("hello after compact").await?;

    let requests = request_log.requests();
    assert_eq!(requests.len(), 3);
    assert!(
        !requests[0]
            .message_input_texts("developer")
            .iter()
            .any(|message| message == additional_context),
        "compact matcher should not run for initial startup",
    );
    assert!(
        requests[2]
            .message_input_texts("developer")
            .iter()
            .any(|message| message == additional_context),
        "compact matcher should inject additional context before the next model turn",
    );

    let hook_inputs = read_session_start_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(
        hook_inputs[0].get("source").and_then(Value::as_str),
        Some("compact")
    );

    Ok(())
}

#[tokio::test]
async fn resumed_thread_runs_resume_then_compact_session_start_hooks() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let limit = 200_000;
    let over_limit_tokens = 250_000;
    let remote_summary = "remote compact summary";
    let resume_context = "remember the resumed reef";
    let compact_context = "remember the compacted reef";
    let compacted_history = vec![
        ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: remote_summary.to_string(),
            }],
            phase: None,
        },
        ResponseItem::Compaction {
            encrypted_content: "encrypted compact summary".to_string(),
        },
    ];
    let compact_mock =
        mount_compact_json_once(&server, serde_json::json!({ "output": compacted_history })).await;

    let mut builder = test_codex()
        .with_pre_build_hook(move |home| {
            if let Err(error) = write_resume_and_compact_session_start_hook_with_context(
                home,
                resume_context,
                compact_context,
            ) {
                panic!("failed to write resume/compact session start hook fixture: {error}");
            }
        })
        .with_config(move |config| {
            config.model_auto_compact_token_limit = Some(limit);
            trust_discovered_hooks(config);
        });
    let initial = builder.build(&server).await?;
    let home = initial.home.clone();
    let rollout_path = initial
        .session_configured
        .rollout_path
        .clone()
        .context("rollout path")?;

    mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "hello before resume"),
            ev_completed_with_tokens("resp-1", over_limit_tokens),
        ]),
    )
    .await;
    initial.submit_turn("hello before resume").await?;
    assert!(compact_mock.requests().is_empty());

    let mut resume_builder = test_codex().with_config(move |config| {
        config.model_auto_compact_token_limit = Some(limit);
        trust_discovered_hooks(config);
    });
    let resumed = resume_builder.resume(&server, home, rollout_path).await?;
    let follow_up = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-2"),
            ev_assistant_message("msg-2", "hello after resume"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    resumed.submit_turn("hello after resume").await?;

    assert_eq!(compact_mock.requests().len(), 1);
    let developer_messages = follow_up.single_request().message_input_texts("developer");
    assert!(
        developer_messages
            .iter()
            .any(|message| message == resume_context),
        "resume matcher should inject additional context before the next model turn",
    );
    assert!(
        developer_messages
            .iter()
            .any(|message| message == compact_context),
        "compact matcher should inject additional context before the next model turn",
    );

    let hook_inputs = read_session_start_hook_inputs(resumed.codex_home_path())?;
    assert_eq!(
        hook_inputs
            .iter()
            .filter_map(|input| input.get("source").and_then(Value::as_str))
            .collect::<Vec<_>>(),
        vec!["resume", "compact"],
    );

    Ok(())
}
