#![allow(clippy::expect_used)]

use crate::compact_remote_support::*;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_compact_replaces_history_for_followups() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing()),
    )
    .await?;
    let codex = harness.test().codex.clone();
    let session_id = harness.test().session_configured.session_id.to_string();
    let thread_id = harness.test().session_configured.thread_id.to_string();

    let responses_mock = responses::mount_sse_sequence(
        harness.server(),
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "FIRST_REMOTE_REPLY"),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "AFTER_COMPACT_REPLY"),
                responses::ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let compacted_history = vec![ResponseItem::Compaction {
        encrypted_content: "ENCRYPTED_COMPACTION_SUMMARY".to_string(),
    }];
    let compact_mock = responses::mount_compact_json_once(
        harness.server(),
        serde_json::json!({ "output": compacted_history.clone() }),
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
    wait_for_turn_complete(&codex).await;

    codex.submit(Op::Compact).await?;
    wait_for_turn_complete(&codex).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "after compact".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_turn_complete(&codex).await;

    let compact_request = compact_mock.single_request();
    assert_eq!(compact_request.path(), "/v1/responses/compact");
    assert_eq!(
        compact_request.header("chatgpt-account-id").as_deref(),
        Some("account_id")
    );
    assert_eq!(
        compact_request.header("authorization").as_deref(),
        Some("Bearer Access Token")
    );
    assert_eq!(
        compact_request.header("session-id").as_deref(),
        Some(session_id.as_str())
    );
    assert_eq!(
        compact_request.header("thread-id").as_deref(),
        Some(thread_id.as_str())
    );
    let compact_body = compact_request.body_json();
    assert_eq!(
        compact_body.get("model").and_then(|v| v.as_str()),
        Some(harness.test().session_configured.model.as_str())
    );
    let response_requests = responses_mock.requests();
    let first_response_request = response_requests.first().expect("initial request missing");
    assert_eq!(
        compact_body["tools"],
        first_response_request.body_json()["tools"],
        "compact requests should send the same tools payload as /v1/responses"
    );
    assert_eq!(
        compact_body["parallel_tool_calls"],
        first_response_request.body_json()["parallel_tool_calls"],
        "compact requests should match /v1/responses parallel_tool_calls"
    );
    assert_eq!(
        compact_body["reasoning"],
        first_response_request.body_json()["reasoning"],
        "compact requests should match /v1/responses reasoning"
    );
    assert_eq!(
        compact_body["text"],
        first_response_request.body_json()["text"],
        "compact requests should match /v1/responses text controls"
    );
    let compact_body_text = compact_body.to_string();
    assert!(
        compact_body_text.contains("hello remote compact"),
        "expected compact request to include user history"
    );
    assert!(
        compact_body_text.contains("FIRST_REMOTE_REPLY"),
        "expected compact request to include assistant history"
    );

    let response_requests = responses_mock.requests();
    let follow_up_request = response_requests.last().expect("follow-up request missing");
    let follow_up_body = follow_up_request.body_json().to_string();
    assert!(
        follow_up_body.contains("\"type\":\"compaction\""),
        "expected follow-up request to use compacted history"
    );
    assert!(
        follow_up_body.contains("ENCRYPTED_COMPACTION_SUMMARY"),
        "expected follow-up request to include compaction summary item"
    );
    assert!(
        !follow_up_body.contains("FIRST_REMOTE_REPLY"),
        "expected follow-up request to drop pre-compaction assistant messages"
    );
    assert!(
        !follow_up_body.contains("hello remote compact"),
        "expected follow-up request to drop compacted-away user turns when remote output omits them"
    );

    insta::assert_snapshot!(
        "remote_manual_compact_with_history_shapes",
        format_labeled_requests_snapshot(
            "Remote manual /compact where remote compact output is compaction-only: follow-up layout uses the returned compaction item plus new user message.",
            &[
                ("Remote Compaction Request", &compact_request),
                ("Remote Post-Compaction History Layout", follow_up_request),
            ]
        )
    );

    Ok(())
}

async fn assert_remote_manual_compact_request_parity(
    auth: CodexAuth,
    configured_service_tier: Option<ServiceTier>,
    expected_service_tier: Option<&str>,
    snapshot_name: &str,
    scenario: &str,
) -> Result<()> {
    let mut builder = test_codex().with_auth(auth);
    if let Some(service_tier) = configured_service_tier {
        builder = builder.with_config(move |config| {
            config.service_tier = Some(service_tier.request_value().to_string());
        });
    }
    let harness = TestCodexHarness::with_builder(builder).await?;
    let codex = harness.test().codex.clone();
    let image_url =
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR4nGNgYAAAAAMAASsJTYQAAAAASUVORK5CYII="
            .to_string();

    let responses_mock = responses::mount_sse_sequence(
        harness.server(),
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("turn-one-assistant", "TURN_ONE_ASSISTANT"),
                responses::ev_completed("turn-one-response"),
            ]),
            responses::sse(vec![
                responses::ev_reasoning_item(
                    "turn-two-reasoning",
                    &["TURN_TWO_REASONING"],
                    &["turn two raw content"],
                ),
                responses::ev_assistant_message("turn-two-assistant", "TURN_TWO_ASSISTANT"),
                responses::ev_completed("turn-two-response"),
            ]),
            responses::sse(vec![
                responses::ev_function_call("turn-three-call", DUMMY_FUNCTION_NAME, "{}"),
                responses::ev_completed("turn-three-call-response"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("turn-three-assistant", "TURN_THREE_ASSISTANT"),
                responses::ev_completed("turn-three-final-response"),
            ]),
            responses::sse(vec![
                responses::ev_shell_command_call(
                    "turn-four-shell-command",
                    "echo TURN_FOUR_LOCAL_SHELL",
                ),
                responses::ev_completed("turn-four-local-shell-response"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("turn-four-assistant", "TURN_FOUR_ASSISTANT"),
                responses::ev_completed("turn-four-final-response"),
            ]),
            responses::sse(vec![
                responses::ev_reasoning_item(
                    "turn-five-reasoning",
                    &["TURN_FIVE_REASONING"],
                    &["turn five raw content"],
                ),
                responses::ev_assistant_message("turn-five-assistant", "TURN_FIVE_ASSISTANT"),
                responses::ev_completed("turn-five-response"),
            ]),
        ],
    )
    .await;
    let compact_mock = responses::mount_compact_user_history_with_summary_once(
        harness.server(),
        "REMOTE_CACHE_TIER_SUMMARY",
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "TURN_ONE_USER".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_turn_complete(&codex).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![
                UserInput::Text {
                    text: "TURN_TWO_PREFIX".to_string(),
                    text_elements: Vec::new(),
                },
                UserInput::Text {
                    text: "TURN_TWO_SUFFIX".to_string(),
                    text_elements: Vec::new(),
                },
            ],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_turn_complete(&codex).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "TURN_THREE_TOOL_USER".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_turn_complete(&codex).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![
                UserInput::Image {
                    image_url,
                    detail: None,
                },
                UserInput::Text {
                    text: "TURN_FOUR_IMAGE_USER".to_string(),
                    text_elements: Vec::new(),
                },
            ],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_turn_complete(&codex).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "TURN_FIVE_USER".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_turn_complete(&codex).await;

    codex.submit(Op::Compact).await?;
    wait_for_turn_complete(&codex).await;

    let response_requests = responses_mock.requests();
    assert_eq!(
        response_requests.len(),
        7,
        "expected five turns with one unsupported tool continuation and one shell command continuation"
    );
    assert_eq!(
        compact_mock.requests().len(),
        1,
        "expected exactly one remote compact request"
    );
    let normal_request = response_requests
        .last()
        .cloned()
        .expect("last turn request missing");
    let compact_request = compact_mock.single_request();
    let normal_body = normal_request.body_json();
    let compact_body = compact_request.body_json();

    let mut expected_compact_body_without_input = normal_body.clone();
    let expected_compact_object = expected_compact_body_without_input
        .as_object_mut()
        .expect("responses request body should be an object");
    for field in [
        "input",
        "client_metadata",
        "include",
        "store",
        "stream",
        "tool_choice",
    ] {
        expected_compact_object.remove(field);
    }
    if expected_service_tier.is_none() {
        expected_compact_object.remove("service_tier");
    }
    let mut compact_body_without_input = compact_body.clone();
    compact_body_without_input
        .as_object_mut()
        .expect("compact request body should be an object")
        .remove("input");
    let canonical_compact_body_without_input = canonical_json(&compact_body_without_input);
    let canonical_expected_compact_body_without_input =
        canonical_json(&expected_compact_body_without_input);

    assert_eq!(
        json!({
            "compact_body_without_input": canonical_compact_body_without_input,
            "expected_compact_body_without_input": canonical_expected_compact_body_without_input,
            "prompt_cache_key_matches_responses": compact_body["prompt_cache_key"] == normal_body["prompt_cache_key"],
            "prompt_cache_key_present": compact_body["prompt_cache_key"].is_string(),
            "service_tier": compact_body.get("service_tier").and_then(Value::as_str),
        }),
        json!({
            "compact_body_without_input": canonical_expected_compact_body_without_input,
            "expected_compact_body_without_input": canonical_expected_compact_body_without_input,
            "prompt_cache_key_matches_responses": true,
            "prompt_cache_key_present": true,
            "service_tier": expected_service_tier,
        }),
        "compact requests should carry the same shared request fields as /responses"
    );

    insta::assert_snapshot!(
        snapshot_name,
        context_snapshot::format_request_body_diff_snapshot(
            scenario,
            "Last Normal /responses Request",
            &normal_request,
            "Remote /responses/compact Request",
            &compact_request,
            &ContextSnapshotOptions::default(),
        )
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_manual_compact_api_auth_omits_service_tier_and_reuses_prompt_cache_key()
-> Result<()> {
    skip_if_no_network!(Ok(()));

    assert_remote_manual_compact_request_parity(
        CodexAuth::from_api_key("dummy"),
        Some(ServiceTier::Fast),
        /*expected_service_tier*/ None,
        "remote_manual_compact_api_auth_prompt_cache_key_request_diff",
        "After five varied API-key-auth turns, remote manual compaction omits service_tier, reuses prompt_cache_key, and still omits responses-only fields.",
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_manual_compact_chatgpt_auth_reuses_service_tier_and_prompt_cache_key() -> Result<()>
{
    skip_if_no_network!(Ok(()));

    assert_remote_manual_compact_request_parity(
        CodexAuth::create_dummy_chatgpt_auth_for_testing(),
        Some(ServiceTier::Fast),
        Some("priority"),
        "remote_manual_compact_chatgpt_auth_service_tier_prompt_cache_key_request_diff",
        "After five varied ChatGPT-auth turns, remote manual compaction reuses service_tier and prompt_cache_key while omitting responses-only fields.",
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_compact_v2_reuses_context_compaction_for_followups() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config(|config| {
                let _ = config.features.enable(Feature::RemoteCompactionV2);
            }),
    )
    .await?;
    let codex = harness.test().codex.clone();

    let responses_mock = responses::mount_sse_sequence(
        harness.server(),
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "FIRST_REMOTE_REPLY"),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                serde_json::json!({
                    "type": "response.output_item.done",
                    "item": {
                        "type": "context_compaction",
                        "encrypted_content": "ENCRYPTED_CONTEXT_COMPACTION_SUMMARY",
                    }
                }),
                responses::ev_completed("resp-compact"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "AFTER_COMPACT_REPLY"),
                responses::ev_completed("resp-2"),
            ]),
        ],
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
    wait_for_turn_complete(&codex).await;

    codex.submit(Op::Compact).await?;
    wait_for_turn_complete(&codex).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "after compact".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_turn_complete(&codex).await;

    let response_requests = responses_mock.requests();
    let compact_request = &response_requests[1];
    assert!(
        compact_request
            .header("x-codex-beta-features")
            .as_deref()
            .is_some_and(|value| value
                .split(',')
                .any(|feature| feature == "remote_compaction_v2")),
        "expected compact request to advertise the remote_compaction_v2 beta feature"
    );
    assert_eq!(compact_request.path(), "/v1/responses");
    let compact_body = compact_request.body_json().to_string();
    assert!(
        compact_body.contains("\"type\":\"context_compaction\""),
        "expected v2 compaction request to include the context_compaction trigger item"
    );
    assert!(
        !compact_body.contains("ENCRYPTED_CONTEXT_COMPACTION_SUMMARY"),
        "expected v2 compaction trigger item to omit encrypted_content"
    );

    let follow_up_request = response_requests.last().expect("follow-up request missing");
    let follow_up_body = follow_up_request.body_json().to_string();
    assert!(
        follow_up_body.contains("\"type\":\"context_compaction\""),
        "expected follow-up request to preserve the v2 context_compaction item"
    );
    assert!(
        follow_up_body.contains("ENCRYPTED_CONTEXT_COMPACTION_SUMMARY"),
        "expected follow-up request to include the context compaction payload"
    );
    assert!(
        follow_up_body.contains("hello remote compact"),
        "expected v2 follow-up request to preserve retained original user messages"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_compact_v2_accepts_additional_output_items_before_context_compaction() -> Result<()>
{
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex()
            .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
            .with_config(|config| {
                let _ = config.features.enable(Feature::RemoteCompactionV2);
            }),
    )
    .await?;
    let codex = harness.test().codex.clone();

    let responses_mock = responses::mount_sse_sequence(
        harness.server(),
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "FIRST_REMOTE_REPLY"),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m-compact-noise", "IGNORED_COMPACT_REPLY"),
                serde_json::json!({
                    "type": "response.output_item.done",
                    "item": {
                        "type": "context_compaction",
                        "encrypted_content": "ENCRYPTED_CONTEXT_COMPACTION_SUMMARY",
                    }
                }),
                responses::ev_completed("resp-compact"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "AFTER_COMPACT_REPLY"),
                responses::ev_completed("resp-2"),
            ]),
        ],
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
    wait_for_turn_complete(&codex).await;

    codex.submit(Op::Compact).await?;
    wait_for_turn_complete(&codex).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "after compact".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_turn_complete(&codex).await;

    let response_requests = responses_mock.requests();
    let follow_up_request = response_requests.last().expect("follow-up request missing");
    let follow_up_body = follow_up_request.body_json().to_string();
    assert!(
        follow_up_body.contains("\"type\":\"context_compaction\""),
        "expected follow-up request to preserve the v2 context_compaction item"
    );
    assert!(
        follow_up_body.contains("ENCRYPTED_CONTEXT_COMPACTION_SUMMARY"),
        "expected follow-up request to include the context compaction payload"
    );
    assert!(
        !follow_up_body.contains("IGNORED_COMPACT_REPLY"),
        "expected follow-up request to ignore unrelated output items from the compaction stream"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_compact_filters_deferred_dynamic_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing());
    let mut test = builder.build(&server).await?;
    let hidden_tool = "hidden_dynamic_tool";
    let visible_tool = "visible_dynamic_tool";
    let input_schema = json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false,
    });
    let dynamic_tools = vec![
        DynamicToolSpec {
            namespace: Some("codex_app".to_string()),
            name: hidden_tool.to_string(),
            description: "Hidden until discovered.".to_string(),
            input_schema: input_schema.clone(),
            defer_loading: true,
        },
        DynamicToolSpec {
            namespace: Some("codex_app".to_string()),
            name: visible_tool.to_string(),
            description: "Visible immediately.".to_string(),
            input_schema,
            defer_loading: false,
        },
    ];
    let new_thread = test
        .thread_manager
        .start_thread_with_tools(
            test.config.clone(),
            dynamic_tools,
            /*persist_extended_history*/ false,
        )
        .await?;
    test.codex = new_thread.thread;
    test.session_configured = new_thread.session_configured;
    let codex = test.codex.clone();

    let responses_mock = mount_sse_once(
        &server,
        sse(vec![
            responses::ev_assistant_message("m1", "FIRST_REMOTE_REPLY"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    let compact_mock = responses::mount_compact_json_once(
        &server,
        serde_json::json!({
            "output": compacted_summary_only_output("compact summary"),
        }),
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
    wait_for_turn_complete(&codex).await;

    codex.submit(Op::Compact).await?;
    wait_for_turn_complete(&codex).await;

    let first_response_body = responses_mock.single_request().body_json();
    let compact_body = compact_mock.single_request().body_json();
    assert_eq!(
        compact_body["tools"], first_response_body["tools"],
        "compact requests should send the same model-visible tools payload as /v1/responses"
    );
    assert_tools_payload_does_not_defer(&first_response_body);
    assert_tools_payload_does_not_defer(&compact_body);
    assert_eq!(
        namespace_child_tool_names(&first_response_body, "codex_app"),
        vec![visible_tool.to_string()]
    );
    assert_eq!(
        namespace_child_tool_names(&compact_body, "codex_app"),
        vec![visible_tool.to_string()]
    );

    Ok(())
}
