#![allow(clippy::expect_used)]

use crate::compact_remote_support::*;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_shape_remote_pre_turn_compaction_restates_realtime_start() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = wiremock::MockServer::start().await;
    let realtime_server = start_remote_realtime_server().await;
    let mut builder = remote_realtime_test_codex_builder(&realtime_server).with_config(|config| {
        config.model_auto_compact_token_limit = Some(200);
    });
    let test = builder.build(&server).await?;

    let responses_mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "REMOTE_FIRST_REPLY"),
                responses::ev_completed_with_tokens("r1", /*total_tokens*/ 500),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "REMOTE_SECOND_REPLY"),
                responses::ev_completed_with_tokens("r2", /*total_tokens*/ 80),
            ]),
        ],
    )
    .await;
    let compact_mock = responses::mount_compact_json_once(
        &server,
        serde_json::json!({
            "output": compacted_summary_only_output(
                "REMOTE_PRETURN_REALTIME_STILL_ACTIVE_SUMMARY"
            )
        }),
    )
    .await;

    start_realtime_conversation(test.codex.as_ref()).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_ONE".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_TWO".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 1);
    let requests = responses_mock.requests();
    assert_eq!(requests.len(), 2, "expected two model requests");

    let compact_request = compact_mock.single_request();
    let post_compact_request = &requests[1];
    assert_request_contains_realtime_start(post_compact_request);

    insta::assert_snapshot!(
        "remote_pre_turn_compaction_restates_realtime_start_shapes",
        format_labeled_requests_snapshot(
            "Remote pre-turn auto-compaction while realtime remains active: compaction clears the reference baseline, so the follow-up request restates realtime-start instructions.",
            &[
                ("Remote Compaction Request", &compact_request),
                (
                    "Remote Post-Compaction History Layout",
                    post_compact_request
                ),
            ]
        )
    );

    close_realtime_conversation(test.codex.as_ref()).await?;
    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_request_uses_custom_experimental_realtime_start_instructions() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = wiremock::MockServer::start().await;
    let realtime_server = start_remote_realtime_server().await;
    let custom_instructions = "custom realtime start instructions";
    let mut builder = remote_realtime_test_codex_builder(&realtime_server).with_config({
        let custom_instructions = custom_instructions.to_string();
        move |config| {
            config.experimental_realtime_start_instructions = Some(custom_instructions);
        }
    });
    let test = builder.build(&server).await?;

    let responses_mock = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("m1", "REMOTE_FIRST_REPLY"),
            responses::ev_completed("r1"),
        ]),
    )
    .await;

    start_realtime_conversation(test.codex.as_ref()).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_ONE".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_request_contains_custom_realtime_start(
        &responses_mock.single_request(),
        custom_instructions,
    );

    close_realtime_conversation(test.codex.as_ref()).await?;
    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_shape_remote_pre_turn_compaction_restates_realtime_end() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = wiremock::MockServer::start().await;
    let realtime_server = start_remote_realtime_server().await;
    let mut builder = remote_realtime_test_codex_builder(&realtime_server).with_config(|config| {
        config.model_auto_compact_token_limit = Some(200);
    });
    let test = builder.build(&server).await?;

    let responses_mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "REMOTE_FIRST_REPLY"),
                responses::ev_completed_with_tokens("r1", /*total_tokens*/ 500),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "REMOTE_SECOND_REPLY"),
                responses::ev_completed_with_tokens("r2", /*total_tokens*/ 80),
            ]),
        ],
    )
    .await;
    let compact_mock = responses::mount_compact_json_once(
        &server,
        serde_json::json!({
            "output": compacted_summary_only_output(
                "REMOTE_PRETURN_REALTIME_CLOSED_SUMMARY"
            )
        }),
    )
    .await;

    start_realtime_conversation(test.codex.as_ref()).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_ONE".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    close_realtime_conversation(test.codex.as_ref()).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_TWO".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 1);
    let requests = responses_mock.requests();
    assert_eq!(requests.len(), 2, "expected two model requests");

    let compact_request = compact_mock.single_request();
    let post_compact_request = &requests[1];
    assert_request_contains_realtime_end(post_compact_request);

    insta::assert_snapshot!(
        "remote_pre_turn_compaction_restates_realtime_end_shapes",
        format_labeled_requests_snapshot(
            "Remote pre-turn auto-compaction after realtime was closed between turns: the follow-up request emits realtime-end instructions from previous-turn settings even though compaction cleared the reference baseline.",
            &[
                ("Remote Compaction Request", &compact_request),
                (
                    "Remote Post-Compaction History Layout",
                    post_compact_request
                ),
            ]
        )
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_shape_remote_manual_compact_restates_realtime_start() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = wiremock::MockServer::start().await;
    let realtime_server = start_remote_realtime_server().await;
    let mut builder = remote_realtime_test_codex_builder(&realtime_server);
    let test = builder.build(&server).await?;

    let responses_mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "REMOTE_FIRST_REPLY"),
                responses::ev_completed_with_tokens("r1", /*total_tokens*/ 60),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "REMOTE_SECOND_REPLY"),
                responses::ev_completed_with_tokens("r2", /*total_tokens*/ 80),
            ]),
        ],
    )
    .await;
    let compact_mock = responses::mount_compact_json_once(
        &server,
        serde_json::json!({
            "output": compacted_summary_only_output(
                "REMOTE_MANUAL_REALTIME_STILL_ACTIVE_SUMMARY"
            )
        }),
    )
    .await;

    start_realtime_conversation(test.codex.as_ref()).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_ONE".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    test.codex.submit(Op::Compact).await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_TWO".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 1);
    let requests = responses_mock.requests();
    assert_eq!(requests.len(), 2, "expected two model requests");

    let compact_request = compact_mock.single_request();
    let post_compact_request = &requests[1];
    assert_request_contains_realtime_start(post_compact_request);

    insta::assert_snapshot!(
        "remote_manual_compact_restates_realtime_start_shapes",
        format_labeled_requests_snapshot(
            "Remote manual /compact while realtime remains active: the next regular turn restates realtime-start instructions after compaction clears the baseline.",
            &[
                ("Remote Compaction Request", &compact_request),
                (
                    "Remote Post-Compaction History Layout",
                    post_compact_request
                ),
            ]
        )
    );

    close_realtime_conversation(test.codex.as_ref()).await?;
    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_shape_remote_mid_turn_compaction_does_not_restate_realtime_end()
-> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = wiremock::MockServer::start().await;
    let realtime_server = start_remote_realtime_server().await;
    let mut builder = remote_realtime_test_codex_builder(&realtime_server).with_config(|config| {
        config.model_auto_compact_token_limit = Some(200);
    });
    let test = builder.build(&server).await?;

    let responses_mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("setup", "REMOTE_SETUP_REPLY"),
                responses::ev_completed_with_tokens("setup-response", /*total_tokens*/ 60),
            ]),
            responses::sse(vec![
                responses::ev_function_call("call-remote-mid-turn", DUMMY_FUNCTION_NAME, "{}"),
                responses::ev_completed_with_tokens("r1", /*total_tokens*/ 500),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "REMOTE_MID_TURN_FINAL_REPLY"),
                responses::ev_completed_with_tokens("r2", /*total_tokens*/ 80),
            ]),
        ],
    )
    .await;
    let compact_mock = responses::mount_compact_json_once(
        &server,
        serde_json::json!({
            "output": compacted_summary_only_output(
                "REMOTE_MID_TURN_REALTIME_CLOSED_SUMMARY"
            )
        }),
    )
    .await;

    start_realtime_conversation(test.codex.as_ref()).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "SETUP_USER".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    close_realtime_conversation(test.codex.as_ref()).await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_TWO".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 1);
    let requests = responses_mock.requests();
    assert_eq!(requests.len(), 3, "expected three model requests");

    let second_turn_request = &requests[1];
    let compact_request = compact_mock.single_request();
    let post_compact_request = &requests[2];
    assert_request_contains_realtime_end(second_turn_request);
    assert!(
        !post_compact_request
            .body_json()
            .to_string()
            .contains("<realtime_conversation>"),
        "did not expect post-compaction history to restate realtime instructions once the current turn had already established an inactive baseline"
    );

    insta::assert_snapshot!(
        "remote_mid_turn_compaction_does_not_restate_realtime_end_shapes",
        format_labeled_requests_snapshot(
            "Remote mid-turn continuation compaction after realtime was closed before the turn: the initial second-turn request emits realtime-end instructions, but the continuation request does not restate them after compaction because the current turn already established the inactive baseline.",
            &[
                ("Second Turn Initial Request", second_turn_request),
                ("Remote Compaction Request", &compact_request),
                (
                    "Remote Post-Compaction History Layout",
                    post_compact_request
                ),
            ]
        )
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_shape_remote_compact_resume_restates_realtime_end() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = wiremock::MockServer::start().await;
    let realtime_server = start_remote_realtime_server().await;
    let mut builder = remote_realtime_test_codex_builder(&realtime_server);
    let initial = builder.build(&server).await?;
    let home = initial.home.clone();
    let rollout_path = initial
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");

    let responses_mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "REMOTE_FIRST_REPLY"),
                responses::ev_completed_with_tokens("r1", /*total_tokens*/ 60),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "REMOTE_AFTER_RESUME_REPLY"),
                responses::ev_completed_with_tokens("r2", /*total_tokens*/ 80),
            ]),
        ],
    )
    .await;
    let compact_mock = responses::mount_compact_json_once(
        &server,
        serde_json::json!({
            "output": compacted_summary_only_output(
                "REMOTE_RESUME_REALTIME_CLOSED_SUMMARY"
            )
        }),
    )
    .await;

    start_realtime_conversation(initial.codex.as_ref()).await?;

    initial
        .codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_ONE".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&initial.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    close_realtime_conversation(initial.codex.as_ref()).await?;

    initial.codex.submit(Op::Compact).await?;
    wait_for_event(&initial.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    initial.codex.submit(Op::Shutdown).await?;
    wait_for_event(&initial.codex, |ev| {
        matches!(ev, EventMsg::ShutdownComplete)
    })
    .await;

    let mut resume_builder =
        test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing());
    let resumed = resume_builder.resume(&server, home, rollout_path).await?;

    resumed
        .codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_TWO".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&resumed.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 1);
    let requests = responses_mock.requests();
    assert_eq!(requests.len(), 2, "expected two model requests");

    let compact_request = compact_mock.single_request();
    let after_resume_request = &requests[1];
    assert_request_contains_realtime_end(after_resume_request);

    insta::assert_snapshot!(
        "remote_compact_resume_restates_realtime_end_shapes",
        format_labeled_requests_snapshot(
            "After remote manual /compact and resume, the first resumed turn rebuilds history from the compaction item and restates realtime-end instructions from reconstructed previous-turn settings.",
            &[
                ("Remote Compaction Request", &compact_request),
                ("Remote Post-Resume History Layout", after_resume_request),
            ]
        )
    );

    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// TODO(ccunningham): Update once remote pre-turn compaction includes incoming user input.
async fn snapshot_request_shape_remote_pre_turn_compaction_including_incoming_user_message()
-> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config(|config| {
                config.model_auto_compact_token_limit = Some(200);
            }),
    )
    .await?;
    let codex = harness.test().codex.clone();

    let responses_mock = responses::mount_sse_sequence(
        harness.server(),
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "REMOTE_FIRST_REPLY"),
                responses::ev_completed_with_tokens("r1", /*total_tokens*/ 60),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "REMOTE_SECOND_REPLY"),
                responses::ev_completed_with_tokens("r2", /*total_tokens*/ 500),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m3", "REMOTE_FINAL_REPLY"),
                responses::ev_completed_with_tokens("r3", /*total_tokens*/ 80),
            ]),
        ],
    )
    .await;

    let compact_mock = responses::mount_compact_user_history_with_summary_once(
        harness.server(),
        &summary_with_prefix("REMOTE_PRE_TURN_SUMMARY"),
    )
    .await;

    for user in ["USER_ONE", "USER_TWO", "USER_THREE"] {
        if user == "USER_THREE" {
            codex
                .submit(Op::OverrideTurnContext {
                    cwd: Some(PathBuf::from(PRETURN_CONTEXT_DIFF_CWD)),
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
                })
                .await?;
        }
        codex
            .submit(Op::UserInput {
                environments: None,
                items: vec![UserInput::Text {
                    text: user.to_string(),
                    text_elements: Vec::new(),
                }],
                final_output_json_schema: None,
                responsesapi_client_metadata: None,
            })
            .await?;
        wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
    }

    assert_eq!(compact_mock.requests().len(), 1);
    let requests = responses_mock.requests();
    assert_eq!(
        requests.len(),
        3,
        "expected user, user, and post-compact turn"
    );

    let compact_request = compact_mock.single_request();
    insta::assert_snapshot!(
        "remote_pre_turn_compaction_including_incoming_shapes",
        format_labeled_requests_snapshot(
            "Remote pre-turn auto-compaction with a context override emits the context diff in the compact request while excluding the incoming user message.",
            &[
                ("Remote Compaction Request", &compact_request),
                ("Remote Post-Compaction History Layout", &requests[2]),
            ]
        )
    );
    assert_eq!(
        requests[2]
            .message_input_texts("user")
            .iter()
            .filter(|text| text.as_str() == "USER_THREE")
            .count(),
        1,
        "post-compaction request should contain incoming user exactly once from runtime append"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_shape_remote_pre_turn_compaction_strips_incoming_model_switch()
-> Result<()> {
    skip_if_no_network!(Ok(()));

    let previous_model = "gpt-5.4";
    let next_model = "gpt-5.3-codex";
    let harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_model(previous_model)
            .with_config(|config| {
                config.model_auto_compact_token_limit = Some(200);
            }),
    )
    .await?;
    let codex = harness.test().codex.clone();

    let initial_turn_request_mock = responses::mount_sse_once(
        harness.server(),
        responses::sse(vec![
            responses::ev_assistant_message("m1", "BEFORE_SWITCH_REPLY"),
            responses::ev_completed_with_tokens("r1", /*total_tokens*/ 500),
        ]),
    )
    .await;
    let post_compact_turn_request_mock = responses::mount_sse_once(
        harness.server(),
        responses::sse(vec![
            responses::ev_assistant_message("m2", "AFTER_SWITCH_REPLY"),
            responses::ev_completed_with_tokens("r2", /*total_tokens*/ 80),
        ]),
    )
    .await;
    let compact_mock = responses::mount_compact_user_history_with_summary_once(
        harness.server(),
        &summary_with_prefix("REMOTE_SWITCH_SUMMARY"),
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "BEFORE_SWITCH_USER".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::OverrideTurnContext {
            cwd: None,
            approval_policy: None,
            approvals_reviewer: None,
            sandbox_policy: None,
            permission_profile: None,
            windows_sandbox_level: None,
            model: Some(next_model.to_string()),
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
        .await?;
    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "AFTER_SWITCH_USER".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(
        compact_mock.requests().len(),
        1,
        "expected a single remote pre-turn compaction request"
    );
    assert_eq!(
        initial_turn_request_mock.requests().len(),
        1,
        "expected initial turn request"
    );
    assert_eq!(
        post_compact_turn_request_mock.requests().len(),
        1,
        "expected post-compaction follow-up request"
    );

    let initial_turn_request = initial_turn_request_mock.single_request();
    let compact_request = compact_mock.single_request();
    let post_compact_turn_request = post_compact_turn_request_mock.single_request();
    let compact_body = compact_request.body_json().to_string();
    assert!(
        !compact_body.contains("AFTER_SWITCH_USER"),
        "current behavior excludes incoming user from the pre-turn remote compaction request"
    );
    assert!(
        !compact_body.contains("<model_switch>"),
        "pre-turn remote compaction request should strip incoming model-switch update item"
    );

    let follow_up_body = post_compact_turn_request.body_json().to_string();
    assert!(
        follow_up_body.contains("BEFORE_SWITCH_USER"),
        "post-compaction follow-up should preserve older user messages when they fit"
    );
    assert!(
        follow_up_body.contains("AFTER_SWITCH_USER"),
        "post-compaction follow-up should preserve incoming user message via runtime append"
    );
    assert!(
        follow_up_body.contains("<model_switch>"),
        "post-compaction follow-up should include the model-switch update item"
    );

    insta::assert_snapshot!(
        "remote_pre_turn_compaction_strips_incoming_model_switch_shapes",
        format_labeled_requests_snapshot(
            "Remote pre-turn compaction during model switch currently excludes incoming user input, strips incoming <model_switch> from the compact request payload, and restores it in the post-compaction follow-up request.",
            &[
                ("Initial Request (Previous Model)", &initial_turn_request),
                ("Remote Compaction Request", &compact_request),
                (
                    "Remote Post-Compaction History Layout",
                    &post_compact_turn_request
                ),
            ]
        )
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// TODO(ccunningham): Update once remote pre-turn compaction context-overflow handling includes
// incoming user input and emits richer oversized-input messaging.
async fn snapshot_request_shape_remote_pre_turn_compaction_context_window_exceeded() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config(|config| {
                config.model_auto_compact_token_limit = Some(200);
            }),
    )
    .await?;
    let codex = harness.test().codex.clone();

    let responses_mock = responses::mount_sse_sequence(
        harness.server(),
        vec![responses::sse(vec![
            responses::ev_assistant_message("m1", "REMOTE_FIRST_REPLY"),
            responses::ev_completed_with_tokens("r1", /*total_tokens*/ 500),
        ])],
    )
    .await;

    let compact_mock = responses::mount_compact_response_once(
        harness.server(),
        ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": {
                "code": "context_length_exceeded",
                "message": "Your input exceeds the context window of this model. Please adjust your input and try again."
            }
        })),
    )
    .await;
    let post_compact_turn_mock = responses::mount_sse_once(
        harness.server(),
        responses::sse(vec![
            responses::ev_assistant_message("m2", "REMOTE_POST_COMPACT_SHOULD_NOT_RUN"),
            responses::ev_completed_with_tokens("r2", /*total_tokens*/ 80),
        ]),
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_ONE".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_TWO".to_string(),
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
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 1);
    let requests = responses_mock.requests();
    assert_eq!(
        requests.len(),
        1,
        "expected no post-compaction follow-up turn request after compact failure"
    );
    assert!(
        post_compact_turn_mock.requests().is_empty(),
        "expected turn to stop after compaction failure"
    );

    let include_attempt_request = compact_mock.single_request();
    insta::assert_snapshot!(
        "remote_pre_turn_compaction_context_window_exceeded_shapes",
        format_labeled_requests_snapshot(
            "Remote pre-turn auto-compaction context-window failure: compaction request excludes the incoming user message and the turn errors.",
            &[(
                "Remote Compaction Request (Incoming User Excluded)",
                &include_attempt_request
            ),]
        )
    );
    assert!(
        error_message.to_lowercase().contains("context window"),
        "expected context window failure to surface, got {error_message}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_shape_remote_mid_turn_continuation_compaction() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config(|config| {
                config.model_auto_compact_token_limit = Some(200);
            }),
    )
    .await?;
    let codex = harness.test().codex.clone();

    let responses_mock = responses::mount_sse_sequence(
        harness.server(),
        vec![
            responses::sse(vec![
                responses::ev_function_call("call-remote-mid-turn", DUMMY_FUNCTION_NAME, "{}"),
                responses::ev_completed_with_tokens("r1", /*total_tokens*/ 500),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "REMOTE_MID_TURN_FINAL_REPLY"),
                responses::ev_completed_with_tokens("r2", /*total_tokens*/ 80),
            ]),
        ],
    )
    .await;

    let compact_mock = responses::mount_compact_user_history_with_summary_once(
        harness.server(),
        &summary_with_prefix("REMOTE_MID_TURN_SUMMARY"),
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_ONE".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 1);
    let requests = responses_mock.requests();
    assert_eq!(
        requests.len(),
        2,
        "expected initial and post-compact requests"
    );

    let compact_request = compact_mock.single_request();
    insta::assert_snapshot!(
        "remote_mid_turn_compaction_shapes",
        format_labeled_requests_snapshot(
            "Remote mid-turn continuation compaction after tool output: compact request includes tool artifacts and the follow-up request includes the returned compaction item.",
            &[
                ("Remote Compaction Request", &compact_request),
                ("Remote Post-Compaction History Layout", &requests[1]),
            ]
        )
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_shape_remote_mid_turn_compaction_summary_only_reinjects_context()
-> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config(|config| {
                config.model_auto_compact_token_limit = Some(200);
            }),
    )
    .await?;
    let codex = harness.test().codex.clone();

    let initial_turn_request_mock = responses::mount_sse_once(
        harness.server(),
        responses::sse(vec![
            responses::ev_function_call("call-remote-summary-only", DUMMY_FUNCTION_NAME, "{}"),
            responses::ev_completed_with_tokens("r1", /*total_tokens*/ 500),
        ]),
    )
    .await;
    let post_compact_turn_request_mock = responses::mount_sse_once(
        harness.server(),
        responses::sse(vec![
            responses::ev_assistant_message("m2", "REMOTE_SUMMARY_ONLY_FINAL_REPLY"),
            responses::ev_completed_with_tokens("r2", /*total_tokens*/ 80),
        ]),
    )
    .await;

    let compacted_history = vec![ResponseItem::Compaction {
        encrypted_content: summary_with_prefix("REMOTE_SUMMARY_ONLY"),
    }];
    let compact_mock = responses::mount_compact_json_once(
        harness.server(),
        serde_json::json!({ "output": compacted_history }),
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_ONE".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 1);
    assert_eq!(
        initial_turn_request_mock.requests().len(),
        1,
        "expected initial turn request"
    );
    assert_eq!(
        post_compact_turn_request_mock.requests().len(),
        1,
        "expected post-compaction request"
    );

    let compact_request = compact_mock.single_request();
    let post_compact_turn_request = post_compact_turn_request_mock.single_request();
    insta::assert_snapshot!(
        "remote_mid_turn_compaction_summary_only_reinjects_context_shapes",
        format_labeled_requests_snapshot(
            "Remote mid-turn compaction where compact output has only a compaction item: continuation layout reinjects context before that compaction item.",
            &[
                ("Remote Compaction Request", &compact_request),
                (
                    "Remote Post-Compaction History Layout",
                    &post_compact_turn_request
                ),
            ]
        )
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_shape_remote_mid_turn_compaction_multi_summary_reinjects_above_last_summary()
-> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config(|config| {
                config.model_auto_compact_token_limit = Some(200);
            }),
    )
    .await?;
    let codex = harness.test().codex.clone();

    let setup_turn_request_mock = responses::mount_sse_once(
        harness.server(),
        responses::sse(vec![
            responses::ev_assistant_message("setup", "REMOTE_SETUP_REPLY"),
            responses::ev_completed_with_tokens("setup-response", /*total_tokens*/ 60),
        ]),
    )
    .await;
    let second_turn_request_mock = responses::mount_sse_once(
        harness.server(),
        responses::sse(vec![
            responses::ev_shell_command_call("call-remote-multi-summary", "echo multi-summary"),
            responses::ev_completed_with_tokens("r1", /*total_tokens*/ 1_000),
        ]),
    )
    .await;

    let compact_mock = responses::mount_compact_user_history_with_summary_sequence(
        harness.server(),
        vec![
            summary_with_prefix("REMOTE_OLDER_SUMMARY"),
            summary_with_prefix("REMOTE_LATEST_SUMMARY"),
        ],
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_ONE".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_TWO".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 2);
    assert_eq!(
        setup_turn_request_mock.requests().len(),
        1,
        "expected setup turn request"
    );
    assert_eq!(
        second_turn_request_mock.requests().len(),
        1,
        "expected second-turn pre-compaction request"
    );

    let compact_requests = compact_mock.requests();
    assert_eq!(
        compact_requests.len(),
        2,
        "expected one setup compact and one mid-turn compact request"
    );
    let compact_request = compact_requests[1].clone();
    let second_turn_request = second_turn_request_mock.single_request();
    assert!(
        compact_request.body_contains_text("REMOTE_OLDER_SUMMARY"),
        "older summary should round-trip from conversation history into the next compact request"
    );
    insta::assert_snapshot!(
        "remote_mid_turn_compaction_multi_summary_reinjects_above_last_summary_shapes",
        format_labeled_requests_snapshot(
            "After a prior manual /compact produced an older remote compaction item, the next turn hits remote auto-compaction before the next sampling request. The compact request carries forward that earlier compaction item, and the next sampling request shows the latest compaction item with context reinjected before USER_TWO.",
            &[
                ("Remote Compaction Request", &compact_request),
                (
                    "Second Turn Request (After Compaction)",
                    &second_turn_request
                ),
            ]
        )
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_shape_remote_manual_compact_without_previous_user_messages() -> Result<()>
{
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing()),
    )
    .await?;
    let codex = harness.test().codex.clone();

    let responses_mock = responses::mount_sse_once(
        harness.server(),
        responses::sse(vec![
            responses::ev_assistant_message("m1", "REMOTE_MANUAL_EMPTY_FOLLOW_UP_REPLY"),
            responses::ev_completed_with_tokens("r1", /*total_tokens*/ 80),
        ]),
    )
    .await;

    let compact_mock =
        responses::mount_compact_json_once(harness.server(), serde_json::json!({ "output": [] }))
            .await;

    codex.submit(Op::Compact).await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_ONE".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(
        compact_mock.requests().len(),
        0,
        "manual /compact without prior user should not issue a remote compaction request"
    );
    let follow_up_request = responses_mock.single_request();
    insta::assert_snapshot!(
        "remote_manual_compact_without_prev_user_shapes",
        format_labeled_requests_snapshot(
            "Remote manual /compact with no prior user turn skips the remote compact request; the follow-up turn carries canonical context and new user message.",
            &[("Remote Post-Compaction History Layout", &follow_up_request)]
        )
    );

    Ok(())
}
