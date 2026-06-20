#![allow(clippy::expect_used)]

use crate::compact_remote_support::*;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_compact_runs_automatically() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing()),
    )
    .await?;
    let codex = harness.test().codex.clone();
    let session_id = harness.test().session_configured.session_id.to_string();
    let thread_id = harness.test().session_configured.thread_id.to_string();

    mount_sse_once(
        harness.server(),
        sse(vec![
            responses::ev_shell_command_call("m1", "echo 'hi'"),
            responses::ev_completed_with_tokens("resp-1", /*total_tokens*/ 100000000), // over token limit
        ]),
    )
    .await;
    let responses_mock = mount_sse_once(
        harness.server(),
        responses::sse(vec![
            responses::ev_assistant_message("m2", "AFTER_COMPACT_REPLY"),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    let compact_mock = responses::mount_compact_user_history_with_summary_once(
        harness.server(),
        "REMOTE_COMPACTED_SUMMARY",
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello remote compact".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let message = wait_for_event_match(&codex, |event| match event {
        EventMsg::ContextCompacted(_) => Some(true),
        _ => None,
    })
    .await;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;
    assert!(message);
    assert_eq!(compact_mock.requests().len(), 1);
    assert_eq!(
        compact_mock
            .single_request()
            .header("session-id")
            .as_deref(),
        Some(session_id.as_str())
    );
    assert_eq!(
        compact_mock.single_request().header("thread-id").as_deref(),
        Some(thread_id.as_str())
    );
    let follow_up_request = responses_mock.single_request();
    let follow_up_body = follow_up_request.body_json().to_string();
    assert!(follow_up_body.contains("REMOTE_COMPACTED_SUMMARY"));

    Ok(())
}

#[cfg_attr(target_os = "windows", ignore)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_compact_trims_function_call_history_to_fit_context_window() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let first_user_message = "turn with retained shell call";
    let second_user_message = "turn with trimmed shell call";
    let retained_call_id = "retained-call";
    let trimmed_call_id = "trimmed-call";
    let retained_command = "echo retained-shell-output";
    let trimmed_command = "yes x | head -n 3000";

    let harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config(|config| {
                config.model_context_window = Some(2_000);
                config.model_auto_compact_token_limit = Some(200_000);
            }),
    )
    .await?;
    let codex = harness.test().codex.clone();

    responses::mount_sse_sequence(
        harness.server(),
        vec![
            sse(vec![
                responses::ev_shell_command_call(retained_call_id, retained_command),
                responses::ev_completed("retained-call-response"),
            ]),
            sse(vec![
                responses::ev_assistant_message("retained-assistant", "retained complete"),
                responses::ev_completed("retained-final-response"),
            ]),
            sse(vec![
                responses::ev_shell_command_call(trimmed_call_id, trimmed_command),
                responses::ev_completed("trimmed-call-response"),
            ]),
        ],
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: first_user_message.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: second_user_message.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let compact_mock = responses::mount_compact_user_history_with_summary_once(
        harness.server(),
        "REMOTE_COMPACT_SUMMARY",
    )
    .await;

    codex.submit(Op::Compact).await?;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let compact_request = compact_mock.single_request();
    let user_messages = compact_request.message_input_texts("user");
    assert!(
        user_messages
            .iter()
            .any(|message| message == first_user_message),
        "expected compact request to retain earlier user history"
    );
    assert!(
        user_messages
            .iter()
            .any(|message| message == second_user_message),
        "expected compact request to retain the user boundary message"
    );

    assert!(
        compact_request.has_function_call(retained_call_id)
            && compact_request
                .function_call_output_text(retained_call_id)
                .is_some(),
        "expected compact request to keep the older function call/result pair"
    );
    assert!(
        !compact_request.has_function_call(trimmed_call_id)
            && compact_request
                .function_call_output_text(trimmed_call_id)
                .is_none(),
        "expected compact request to drop the trailing function call/result pair past the boundary"
    );

    assert_eq!(
        compact_request.inputs_of_type("function_call").len(),
        1,
        "expected exactly one function call after trimming"
    );
    assert_eq!(
        compact_request.inputs_of_type("function_call_output").len(),
        1,
        "expected exactly one function call output after trimming"
    );

    Ok(())
}

#[cfg_attr(target_os = "windows", ignore)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_remote_compact_trims_function_call_history_to_fit_context_window() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let first_user_message = "turn with retained shell call";
    let second_user_message = "turn with trimmed shell call";
    let retained_call_id = "retained-call";
    let trimmed_call_id = "trimmed-call";
    let retained_command = "echo retained-shell-output";
    let trimmed_command = "yes x | head -n 3000";
    let harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config(|config| {
                config.model_context_window = Some(2_000);
                config.model_auto_compact_token_limit = Some(200_000);
            }),
    )
    .await?;
    let codex = harness.test().codex.clone();

    responses::mount_sse_sequence(
        harness.server(),
        vec![
            sse(vec![
                responses::ev_shell_command_call(retained_call_id, retained_command),
                responses::ev_completed_with_tokens(
                    "retained-call-response",
                    /*total_tokens*/ 100,
                ),
            ]),
            sse(vec![
                responses::ev_assistant_message("retained-assistant", "retained complete"),
                responses::ev_completed("retained-final-response"),
            ]),
            sse(vec![
                responses::ev_shell_command_call(trimmed_call_id, trimmed_command),
                responses::ev_completed_with_tokens(
                    "trimmed-call-response",
                    /*total_tokens*/ 100,
                ),
            ]),
            sse(vec![responses::ev_completed_with_tokens(
                "trimmed-final-response",
                /*total_tokens*/ 500_000,
            )]),
        ],
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: first_user_message.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: second_user_message.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let compact_mock = responses::mount_compact_user_history_with_summary_once(
        harness.server(),
        "REMOTE_AUTO_COMPACT_SUMMARY",
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "turn that triggers auto compact".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;
    assert_eq!(
        compact_mock.requests().len(),
        1,
        "expected exactly one remote compact request"
    );

    let compact_request = compact_mock.single_request();
    let user_messages = compact_request.message_input_texts("user");
    assert!(
        user_messages
            .iter()
            .any(|message| message == first_user_message),
        "expected compact request to retain earlier user history"
    );
    assert!(
        user_messages
            .iter()
            .any(|message| message == second_user_message),
        "expected compact request to retain the user boundary message"
    );

    assert!(
        compact_request.has_function_call(retained_call_id)
            && compact_request
                .function_call_output_text(retained_call_id)
                .is_some(),
        "expected compact request to keep the older function call/result pair"
    );
    assert!(
        !compact_request.has_function_call(trimmed_call_id)
            && compact_request
                .function_call_output_text(trimmed_call_id)
                .is_none(),
        "expected compact request to drop the trailing function call/result pair past the boundary"
    );

    assert_eq!(
        compact_request.inputs_of_type("function_call").len(),
        1,
        "expected exactly one function call after trimming"
    );
    assert_eq!(
        compact_request.inputs_of_type("function_call_output").len(),
        1,
        "expected exactly one function call output after trimming"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_remote_compact_failure_stops_agent_loop() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config(|config| {
                config.model_auto_compact_token_limit = Some(120);
            }),
    )
    .await?;
    let codex = harness.test().codex.clone();

    mount_sse_once(
        harness.server(),
        sse(vec![
            responses::ev_assistant_message("initial-assistant", "initial turn complete"),
            responses::ev_completed_with_tokens("initial-response", /*total_tokens*/ 500_000),
        ]),
    )
    .await;

    let first_compact_mock = responses::mount_compact_json_once(
        harness.server(),
        serde_json::json!({ "output": "invalid compact payload shape" }),
    )
    .await;
    let post_compact_turn_mock = mount_sse_once(
        harness.server(),
        sse(vec![
            responses::ev_assistant_message("post-compact-assistant", "should not run"),
            responses::ev_completed("post-compact-response"),
        ]),
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "turn that exceeds token threshold".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "turn that triggers auto compact".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let error_message = wait_for_event_match(&codex, |event| match event {
        EventMsg::Error(err) => Some(err.message.clone()),
        _ => None,
    })
    .await;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    assert!(
        error_message.contains("Error running remote compact task"),
        "expected remote compact task error prefix, got {error_message}"
    );
    assert_eq!(
        first_compact_mock.requests().len(),
        1,
        "expected first remote compact attempt with incoming items"
    );
    assert!(
        post_compact_turn_mock.requests().is_empty(),
        "expected agent loop to stop after compaction failure"
    );

    insta::assert_snapshot!(
        "remote_pre_turn_compaction_failure_shapes",
        format_labeled_requests_snapshot(
            "Remote pre-turn auto-compaction parse failure: compaction request excludes the incoming user message and the turn stops.",
            &[(
                "Remote Compaction Request (Incoming User Excluded)",
                &first_compact_mock.single_request()
            ),]
        )
    );

    Ok(())
}

#[cfg_attr(target_os = "windows", ignore)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_compact_trim_estimate_uses_session_base_instructions() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let first_user_message = "turn with baseline shell call";
    let second_user_message = "turn with trailing shell call";
    let baseline_retained_call_id = "baseline-retained-call";
    let baseline_trailing_call_id = "baseline-trailing-call";
    let override_retained_call_id = "override-retained-call";
    let override_trailing_call_id = "override-trailing-call";
    let retained_command = "printf retained-shell-output";
    let trailing_command = "printf '%020000d' 0";

    let baseline_harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config(|config| {
                config.model_context_window = Some(200_000);
            }),
    )
    .await?;
    let baseline_codex = baseline_harness.test().codex.clone();

    responses::mount_sse_sequence(
        baseline_harness.server(),
        vec![
            sse(vec![
                responses::ev_shell_command_call(baseline_retained_call_id, retained_command),
                responses::ev_completed("baseline-retained-call-response"),
            ]),
            sse(vec![
                responses::ev_assistant_message("baseline-retained-assistant", "retained complete"),
                responses::ev_completed("baseline-retained-final-response"),
            ]),
            sse(vec![
                responses::ev_shell_command_call(baseline_trailing_call_id, trailing_command),
                responses::ev_completed("baseline-trailing-call-response"),
            ]),
            sse(vec![responses::ev_completed(
                "baseline-trailing-final-response",
            )]),
        ],
    )
    .await;

    baseline_codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: first_user_message.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&baseline_codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    baseline_codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: second_user_message.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&baseline_codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let baseline_compact_mock = responses::mount_compact_user_history_with_summary_once(
        baseline_harness.server(),
        "REMOTE_BASELINE_SUMMARY",
    )
    .await;

    baseline_codex.submit(Op::Compact).await?;
    wait_for_event(&baseline_codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let baseline_compact_request = baseline_compact_mock.single_request();
    assert!(
        baseline_compact_request.has_function_call(baseline_retained_call_id),
        "expected baseline compact request to retain older function call history"
    );
    assert!(
        baseline_compact_request.has_function_call(baseline_trailing_call_id),
        "expected baseline compact request to retain trailing function call history"
    );

    let baseline_input_tokens = estimate_compact_input_tokens(&baseline_compact_request);
    let baseline_payload_tokens = estimate_compact_payload_tokens(&baseline_compact_request);

    let override_base_instructions = format!(
        "{}\nREMOTE_BASE_INSTRUCTIONS_OVERRIDE {}",
        baseline_compact_request.instructions_text(),
        "x".repeat(4_000)
    );
    let override_context_window = baseline_payload_tokens.saturating_add(500);
    let pretrim_override_estimate =
        baseline_input_tokens.saturating_add(approx_token_count(&override_base_instructions));
    assert!(
        pretrim_override_estimate > override_context_window,
        "expected override instructions to push pre-trim estimate past the context window"
    );

    let override_harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config({
                let override_base_instructions = override_base_instructions.clone();
                move |config| {
                    config.model_context_window = Some(override_context_window);
                    config.base_instructions = Some(override_base_instructions);
                }
            }),
    )
    .await?;
    let override_codex = override_harness.test().codex.clone();

    responses::mount_sse_sequence(
        override_harness.server(),
        vec![
            sse(vec![
                responses::ev_shell_command_call(override_retained_call_id, retained_command),
                responses::ev_completed("override-retained-call-response"),
            ]),
            sse(vec![
                responses::ev_assistant_message("override-retained-assistant", "retained complete"),
                responses::ev_completed("override-retained-final-response"),
            ]),
            sse(vec![
                responses::ev_shell_command_call(override_trailing_call_id, trailing_command),
                responses::ev_completed("override-trailing-call-response"),
            ]),
            sse(vec![responses::ev_completed(
                "override-trailing-final-response",
            )]),
        ],
    )
    .await;

    override_codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: first_user_message.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&override_codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    override_codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: second_user_message.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&override_codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let override_compact_mock = responses::mount_compact_user_history_with_summary_once(
        override_harness.server(),
        "REMOTE_OVERRIDE_SUMMARY",
    )
    .await;

    override_codex.submit(Op::Compact).await?;
    wait_for_event(&override_codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let override_compact_request = override_compact_mock.single_request();
    assert_eq!(
        override_compact_request.instructions_text(),
        override_base_instructions
    );
    assert!(
        override_compact_request.has_function_call(override_retained_call_id),
        "expected remote compact request to preserve older function call history"
    );
    assert!(
        !override_compact_request.has_function_call(override_trailing_call_id),
        "expected remote compact request to trim trailing function call history with override instructions"
    );

    Ok(())
}
