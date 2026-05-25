#![allow(clippy::expect_used)]
use codex_features::Feature;
use codex_login::CodexAuth;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::built_in_model_providers;
use codex_models_manager::bundled_models_response;
use codex_protocol::config_types::ContextBudgetMode;
use codex_protocol::items::TurnItem;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::HookEventName;
use codex_protocol::protocol::HookRunStatus;
use codex_protocol::protocol::ItemCompletedEvent;
use codex_protocol::protocol::ItemStartedEvent;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::RolloutLine;
use codex_protocol::protocol::WarningEvent;
use codex_protocol::user_input::UserInput;
use codex_core_test_runtime::compact_fixtures::COMPACT_WARNING_MESSAGE;
use codex_core_test_runtime::compact_fixtures::FIRST_REPLY;
use codex_core_test_runtime::compact_fixtures::SUMMARY_TEXT;
use codex_core_test_runtime::compact_fixtures::assert_compaction_uses_turn_lifecycle_id;
use codex_core_test_runtime::compact_fixtures::assert_pre_sampling_switch_compaction_requests;
use codex_core_test_runtime::compact_fixtures::auto_summary;
use codex_core_test_runtime::compact_fixtures::body_contains_compaction_prompt;
use codex_core_test_runtime::compact_fixtures::body_contains_compaction_summary_prefix;
use codex_core_test_runtime::compact_fixtures::body_contains_text;
use codex_core_test_runtime::compact_fixtures::compact_prompt;
use codex_core_test_runtime::compact_fixtures::disabled_permission_plan_turn;
use codex_core_test_runtime::compact_fixtures::disabled_permission_user_turn;
use codex_core_test_runtime::compact_fixtures::read_hook_inputs;
use codex_core_test_runtime::compact_fixtures::set_test_compact_prompt;
use codex_core_test_runtime::compact_fixtures::summary_with_prefix;
use codex_core_test_runtime::compact_fixtures::write_matching_compact_hooks;
use codex_core_test_runtime::compact_fixtures::write_unsupported_blocking_pre_compact_hook;
use codex_test_support_responses::context_snapshot;
use codex_test_support_responses::context_snapshot::ContextSnapshotOptions;
use codex_test_support_responses::context_snapshot::ContextSnapshotRenderMode;
use codex_core_test_runtime::hooks::trust_discovered_hooks;
use codex_core_test_runtime::responses::ev_reasoning_item;
use codex_core_test_runtime::responses::mount_models_once;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::wait_for_event;
use codex_core_test_runtime::wait_for_event_match;

use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_completed_with_tokens;
use codex_core_test_runtime::responses::ev_function_call;
use codex_core_test_runtime::responses::ev_incomplete_with_tokens;
use codex_core_test_runtime::responses::mount_compact_json_once;
use codex_core_test_runtime::responses::mount_response_sequence;
use codex_core_test_runtime::responses::mount_sse_once;
use codex_core_test_runtime::responses::mount_sse_once_match;
use codex_core_test_runtime::responses::mount_sse_sequence;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::responses::sse_failed;
use codex_core_test_runtime::responses::sse_response;
use codex_core_test_runtime::responses::start_mock_server;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::path::PathBuf;
use wiremock::MockServer;
// --- Test helpers -----------------------------------------------------------

const THIRD_USER_MSG: &str = "next turn";
const AUTO_SUMMARY_TEXT: &str = "AUTO_SUMMARY";
const FIRST_AUTO_MSG: &str = "token limit start";
const SECOND_AUTO_MSG: &str = "token limit push";
const MULTI_AUTO_MSG: &str = "multi auto";
const SECOND_LARGE_REPLY: &str = "SECOND_LARGE_REPLY";
const FIRST_AUTO_SUMMARY: &str = "FIRST_AUTO_SUMMARY";
const SECOND_AUTO_SUMMARY: &str = "SECOND_AUTO_SUMMARY";
const FINAL_REPLY: &str = "FINAL_REPLY";
const CONTEXT_LIMIT_MESSAGE: &str =
    "Your input exceeds the context window of this model. Please adjust your input and try again.";
const DUMMY_FUNCTION_NAME: &str = "test_tool";
const DUMMY_CALL_ID: &str = "call-multi-auto";
const FUNCTION_CALL_LIMIT_MSG: &str = "function call limit push";
const POST_AUTO_USER_MSG: &str = "post auto follow-up";
const PRETURN_CONTEXT_DIFF_CWD: &str = "/tmp/PRETURN_CONTEXT_DIFF_CWD";

fn non_openai_model_provider(server: &MockServer) -> ModelProviderInfo {
    let mut provider =
        built_in_model_providers(/* openai_base_url */ /*openai_base_url*/ None)["openai"].clone();
    provider.name = "OpenAI (test)".into();
    provider.base_url = Some(format!("{}/v1", server.uri()));
    provider.supports_websockets = false;
    provider
}

fn model_info_with_context_window(slug: &str, context_window: i64) -> ModelInfo {
    let models_response = bundled_models_response()
        .unwrap_or_else(|err| panic!("bundled models.json should parse: {err}"));
    let mut model_info = models_response
        .models
        .into_iter()
        .find(|model| model.slug == slug)
        .unwrap_or_else(|| panic!("model `{slug}` missing from models.json"));
    model_info.context_window = Some(context_window);
    model_info
}

fn context_snapshot_options() -> ContextSnapshotOptions {
    ContextSnapshotOptions::default()
        .strip_capability_instructions()
        .render_mode(ContextSnapshotRenderMode::KindWithTextPrefix { max_chars: 64 })
}

fn format_labeled_requests_snapshot(
    scenario: &str,
    sections: &[(&str, &codex_core_test_runtime::responses::ResponsesRequest)],
) -> String {
    context_snapshot::format_labeled_requests_snapshot(
        scenario,
        sections,
        &context_snapshot_options(),
    )
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]

async fn snapshot_request_shape_pre_turn_compaction_including_incoming_user_message() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let sse1 = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", /*total_tokens*/ 60),
    ]);
    let sse2 = sse(vec![
        ev_assistant_message("m2", "SECOND_REPLY"),
        ev_completed_with_tokens("r2", /*total_tokens*/ 500),
    ]);
    let sse3 = sse(vec![
        ev_assistant_message("m3", "PRE_TURN_SUMMARY"),
        ev_completed_with_tokens("r3", /*total_tokens*/ 100),
    ]);
    let sse4 = sse(vec![
        ev_assistant_message("m4", FINAL_REPLY),
        ev_completed_with_tokens("r4", /*total_tokens*/ 80),
    ]);
    let request_log = mount_sse_sequence(&server, vec![sse1, sse2, sse3, sse4]).await;

    let model_provider = non_openai_model_provider(&server);
    let codex = test_codex()
        .with_config(move |config| {
            config.model_provider = model_provider;
            set_test_compact_prompt(config);
            config.model_auto_compact_token_limit = Some(200);
        })
        .build(&server)
        .await
        .expect("build codex")
        .codex;

    for user in ["USER_ONE", "USER_TWO"] {
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
            .await
            .expect("submit user input");
        wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
    }
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
        .await
        .expect("override turn context");
    let image_url = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR4nGNgYAAAAAMAASsJTYQAAAAASUVORK5CYII="
        .to_string();
    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![
                UserInput::Image {
                    image_url: image_url.clone(),
                    detail: None,
                },
                UserInput::Text {
                    text: "USER_THREE".to_string(),
                    text_elements: Vec::new(),
                },
            ],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submit user input");
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert_eq!(requests.len(), 4, "expected user, user, compact, follow-up");

    insta::assert_snapshot!(
        "pre_turn_compaction_including_incoming_shapes",
        format_labeled_requests_snapshot(
            "Pre-turn auto-compaction with a context override emits the context diff in the compact request while the incoming user message is still excluded.",
            &[
                ("Local Compaction Request", &requests[2]),
                ("Local Post-Compaction History Layout", &requests[3]),
            ]
        )
    );
    let compact_request_user_texts = requests[2].message_input_texts("user");
    assert!(
        !compact_request_user_texts
            .iter()
            .any(|text| text == "USER_THREE"),
        "current behavior excludes incoming user message from pre-turn compaction input"
    );
    let follow_up_user_texts = requests[3].message_input_texts("user");
    assert!(
        follow_up_user_texts.iter().any(|text| text == "USER_THREE"),
        "expected post-compaction follow-up request to keep incoming user text"
    );
    let follow_up_user_images = requests[3].message_input_image_urls("user");
    assert!(
        follow_up_user_images
            .iter()
            .any(|url| url == image_url.as_str()),
        "expected post-compaction follow-up request to keep incoming user image content"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// TODO(ccunningham): Update once pre-turn compaction context-overflow handling includes incoming
// user input and emits richer oversized-input messaging.
async fn snapshot_request_shape_pre_turn_compaction_strips_incoming_model_switch() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let previous_model = "gpt-5.4";
    let next_model = "gpt-5.3-codex";

    let request_log = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_assistant_message("m1", "BEFORE_SWITCH_REPLY"),
                ev_completed_with_tokens("r1", /*total_tokens*/ 500),
            ]),
            sse(vec![
                ev_assistant_message("m2", "PRETURN_SWITCH_SUMMARY"),
                ev_completed_with_tokens("r2", /*total_tokens*/ 100),
            ]),
            sse(vec![
                ev_assistant_message("m3", "AFTER_SWITCH_REPLY"),
                ev_completed_with_tokens("r3", /*total_tokens*/ 100),
            ]),
        ],
    )
    .await;

    let model_provider = non_openai_model_provider(&server);
    let test = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_model(previous_model)
        .with_config(move |config| {
            config.model_provider = model_provider;
            set_test_compact_prompt(config);
            let _ = config.features.enable(Feature::RemoteModels);
            config.model_auto_compact_token_limit = Some(200);
        })
        .build(&server)
        .await
        .expect("build codex");

    test.codex
        .submit(disabled_permission_user_turn(
            "BEFORE_SWITCH_USER",
            test.cwd.path().to_path_buf(),
            previous_model.to_string(),
        ))
        .await
        .expect("submit first user turn");
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    test.codex
        .submit(disabled_permission_user_turn(
            "AFTER_SWITCH_USER",
            test.cwd.path().to_path_buf(),
            next_model.to_string(),
        ))
        .await
        .expect("submit second user turn");
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = request_log.requests();
    assert_eq!(
        requests.len(),
        3,
        "expected first turn, pre-turn compact, and post-compact follow-up requests"
    );

    let compact_body = requests[1].body_json().to_string();
    assert!(
        body_contains_compaction_prompt(&compact_body),
        "pre-turn compaction request should include summarization prompt"
    );
    assert!(
        !compact_body.contains("<model_switch>"),
        "pre-turn compaction request should strip incoming model-switch update item"
    );

    let follow_up_body = requests[2].body_json().to_string();
    assert!(
        follow_up_body.contains("<model_switch>"),
        "post-compaction follow-up should include model-switch update item"
    );

    insta::assert_snapshot!(
        "pre_turn_compaction_strips_incoming_model_switch_shapes",
        format_labeled_requests_snapshot(
            "Pre-turn compaction during model switch (without pre-sampling model-switch compaction): current behavior strips incoming <model_switch> from the compact request and restores it in the post-compaction follow-up request.",
            &[
                ("Initial Request (Previous Model)", &requests[0]),
                ("Local Compaction Request", &requests[1]),
                ("Local Post-Compaction History Layout", &requests[2]),
            ]
        )
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]

async fn snapshot_request_shape_mid_turn_continuation_compaction() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let context_window = 100;
    let limit = context_window * 90 / 100;
    let over_limit_tokens = context_window * 95 / 100 + 1;

    let first_turn = sse(vec![
        ev_function_call(DUMMY_CALL_ID, DUMMY_FUNCTION_NAME, "{}"),
        ev_completed_with_tokens("r1", over_limit_tokens),
    ]);
    let auto_summary_payload = auto_summary(AUTO_SUMMARY_TEXT);
    let auto_compact_turn = sse(vec![
        ev_assistant_message("m2", &auto_summary_payload),
        ev_completed_with_tokens("r3", /*total_tokens*/ 10),
    ]);
    let post_auto_compact_turn = sse(vec![
        ev_assistant_message("m3", FINAL_REPLY),
        ev_completed_with_tokens("r4", /*total_tokens*/ 10),
    ]);

    // Mount responses in order and keep mocks only for the ones we assert on.
    let first_turn_mock = mount_sse_once(&server, first_turn).await;
    let auto_compact_mock = mount_sse_once(&server, auto_compact_turn).await;
    let post_auto_compact_mock = mount_sse_once(&server, post_auto_compact_turn).await;

    let model_provider = non_openai_model_provider(&server);

    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        set_test_compact_prompt(config);
        config.model_context_window = Some(context_window);
        config.model_auto_compact_token_limit = Some(limit);
    });
    let codex = builder.build(&server).await.unwrap().codex;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: FUNCTION_CALL_LIMIT_MSG.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |msg| matches!(msg, EventMsg::TurnComplete(_))).await;

    // Assert first request captured expected user message that triggers function call.
    let first_request = first_turn_mock.single_request().input();
    assert!(
        first_request.iter().any(|item| {
            item.get("type").and_then(|value| value.as_str()) == Some("message")
                && item
                    .get("content")
                    .and_then(|content| content.as_array())
                    .and_then(|entries| entries.first())
                    .and_then(|entry| entry.get("text"))
                    .and_then(|value| value.as_str())
                    == Some(FUNCTION_CALL_LIMIT_MSG)
        }),
        "first request should include the user message that triggers the function call"
    );

    let function_call_output = auto_compact_mock
        .single_request()
        .function_call_output(DUMMY_CALL_ID);
    let output_text = function_call_output
        .get("output")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert!(
        output_text.contains(DUMMY_FUNCTION_NAME),
        "function call output should be sent before auto compact"
    );

    let auto_compact_body = auto_compact_mock.single_request().body_json().to_string();
    assert!(
        body_contains_compaction_prompt(&auto_compact_body),
        "mid-turn auto compact request should include the summarization prompt after exceeding 95% (limit {limit})"
    );

    insta::assert_snapshot!(
        "mid_turn_compaction_shapes",
        format_labeled_requests_snapshot(
            "True mid-turn continuation compaction after tool output: compact request includes tool artifacts, and the continuation request includes the summary in the same turn.",
            &[
                (
                    "Local Compaction Request",
                    &auto_compact_mock.single_request()
                ),
                (
                    "Local Post-Compaction History Layout",
                    &post_auto_compact_mock.single_request()
                ),
            ]
        )
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]

async fn snapshot_request_shape_pre_turn_compaction_context_window_exceeded() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let first_turn = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", /*total_tokens*/ 500),
    ]);
    let mut responses = vec![first_turn];
    responses.extend(
        (0..5).map(|_| {
            sse_failed(
                "compact-failed",
                "context_length_exceeded",
                "Your input exceeds the context window of this model. Please adjust your input and try again.",
            )
        }),
    );
    let request_log = mount_sse_sequence(&server, responses).await;

    let mut model_provider = non_openai_model_provider(&server);
    model_provider.stream_max_retries = Some(0);
    let codex = test_codex()
        .with_config(move |config| {
            config.model_provider = model_provider;
            set_test_compact_prompt(config);
            config.model_auto_compact_token_limit = Some(200);
        })
        .build(&server)
        .await
        .expect("build codex")
        .codex;

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
        .await
        .expect("submit first user");
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
        .await
        .expect("submit second user");
    let error_message = wait_for_event_match(&codex, |event| match event {
        EventMsg::Error(err) => Some(err.message.clone()),
        _ => None,
    })
    .await;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert!(
        requests.len() >= 2,
        "expected first turn and at least one compaction request"
    );

    insta::assert_snapshot!(
        "pre_turn_compaction_context_window_exceeded_shapes",
        format_labeled_requests_snapshot(
            "Pre-turn auto-compaction context-window failure: compaction request excludes the incoming user message and the turn errors.",
            &[(
                "Local Compaction Request (Incoming User Excluded)",
                &requests[1]
            ),]
        )
    );

    assert!(
        error_message.contains("ran out of room in the model's context window"),
        "expected context window exceeded message, got {error_message}"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]

async fn snapshot_request_shape_manual_compact_without_previous_user_messages() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let compact_turn = sse(vec![
        ev_assistant_message("m1", "MANUAL_EMPTY_SUMMARY"),
        ev_completed_with_tokens("r1", /*total_tokens*/ 90),
    ]);
    let follow_up_turn = sse(vec![
        ev_assistant_message("m2", FINAL_REPLY),
        ev_completed_with_tokens("r2", /*total_tokens*/ 80),
    ]);
    let request_log = mount_sse_sequence(&server, vec![compact_turn, follow_up_turn]).await;

    let model_provider = non_openai_model_provider(&server);
    let codex = test_codex()
        .with_config(move |config| {
            config.model_provider = model_provider;
            set_test_compact_prompt(config);
        })
        .build(&server)
        .await
        .expect("build codex")
        .codex;

    codex.submit(Op::Compact).await.expect("run /compact");
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "AFTER_MANUAL_EMPTY_COMPACT".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submit follow-up user input");
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert_eq!(
        requests.len(),
        2,
        "expected manual /compact request and follow-up turn request"
    );

    insta::assert_snapshot!(
        "manual_compact_without_prev_user_shapes",
        format_labeled_requests_snapshot(
            "Manual /compact with no prior user turn currently still issues a compaction request; follow-up turn carries canonical context and the new user message.",
            &[
                ("Local Compaction Request", &requests[0]),
                ("Local Post-Compaction History Layout", &requests[1]),
            ]
        )
    );
}
