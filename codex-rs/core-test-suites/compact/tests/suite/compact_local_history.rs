#![allow(clippy::expect_used)]
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

fn ev_shell_command_call(call_id: &str, command: &str) -> serde_json::Value {
    ev_function_call(
        call_id,
        "shell_command",
        &json!({ "command": command }).to_string(),
    )
}

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
#[cfg_attr(windows, ignore)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn post_turn_auto_compaction_incomplete_max_output_tokens_does_not_retry_or_replace_history()
{
    skip_if_no_network!();

    let server = start_mock_server().await;
    let first_turn = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", /*total_tokens*/ 200_001),
    ]);
    let truncated_compaction = sse(vec![
        ev_assistant_message("m2", "TRUNCATED_SUMMARY"),
        ev_incomplete_with_tokens("r2", "max_output_tokens", /*total_tokens*/ 25),
    ]);
    let next_turn = sse(vec![
        ev_assistant_message("m3", "after failed post-turn compact"),
        ev_completed_with_tokens("r3", /*total_tokens*/ 100),
    ]);
    let request_log =
        mount_sse_sequence(&server, vec![first_turn, truncated_compaction, next_turn]).await;

    let model_provider = non_openai_model_provider(&server);
    let codex = test_codex()
        .with_config(move |config| {
            config.model_provider = model_provider;
            set_test_compact_prompt(config);
            config.model_auto_compact_token_limit = Some(200_000);
        })
        .build(&server)
        .await
        .expect("build codex")
        .codex;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_ONE".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submit first user");

    let error_message = wait_for_event_match(&codex, |event| match event {
        EventMsg::Error(err)
            if err
                .message
                .contains("context compaction response incomplete: max_output_tokens") =>
        {
            Some(err.message.clone())
        }
        _ => None,
    })
    .await;
    assert!(
        error_message.contains("context compaction response incomplete: max_output_tokens"),
        "expected post-turn compaction incomplete error, got {error_message}"
    );

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "USER_TWO".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submit second user");
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert_eq!(
        requests.len(),
        3,
        "post-turn max-output compaction should not retry immediately"
    );
    let next_turn_body = requests[2].body_json().to_string();
    assert!(
        body_contains_text(&next_turn_body, "USER_ONE"),
        "failed compaction must keep original user history"
    );
    assert!(
        body_contains_text(&next_turn_body, FIRST_REPLY),
        "failed compaction must keep original assistant history"
    );
    assert!(
        body_contains_text(&next_turn_body, "USER_TWO"),
        "later user turn should still be sent"
    );
    assert!(
        !body_contains_text(&next_turn_body, "TRUNCATED_SUMMARY"),
        "truncated post-turn compaction output must not be recorded"
    );
    assert!(
        !body_contains_compaction_summary_prefix(&next_turn_body),
        "failed post-turn compaction must not replace history with a summary"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_compact_twice_preserves_latest_user_messages() {
    skip_if_no_network!();

    let first_user_message = "first manual turn";
    let second_user_message = "second manual turn";
    let final_user_message = "post compact follow-up";
    let first_summary = "FIRST_MANUAL_SUMMARY";
    let second_summary = "SECOND_MANUAL_SUMMARY";
    let expected_second_summary = summary_with_prefix(second_summary);

    let server = start_mock_server().await;

    let first_turn = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed("r1"),
    ]);
    let first_compact_summary = auto_summary(first_summary);
    let first_compact = sse(vec![
        ev_assistant_message("m2", &first_compact_summary),
        ev_completed("r2"),
    ]);
    let second_turn = sse(vec![
        ev_assistant_message("m3", SECOND_LARGE_REPLY),
        ev_completed("r3"),
    ]);
    let second_compact_summary = auto_summary(second_summary);
    let second_compact = sse(vec![
        ev_assistant_message("m4", &second_compact_summary),
        ev_completed("r4"),
    ]);
    let final_turn = sse(vec![
        ev_assistant_message("m5", FINAL_REPLY),
        ev_completed("r5"),
    ]);

    let responses_mock = mount_sse_sequence(
        &server,
        vec![
            first_turn,
            first_compact,
            second_turn,
            second_compact,
            final_turn,
        ],
    )
    .await;

    let model_provider = non_openai_model_provider(&server);

    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        set_test_compact_prompt(config);
    });
    let codex = builder.build(&server).await.unwrap().codex;

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
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await.unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

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
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await.unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: final_user_message.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = responses_mock.requests();
    assert_eq!(
        requests.len(),
        5,
        "expected exactly 5 requests (user turn, compact, user turn, compact, final turn)"
    );
    let contains_user_text = |request: &codex_core_test_runtime::responses::ResponsesRequest,
                              expected: &str| {
        request
            .message_input_texts("user")
            .iter()
            .any(|text| text == expected)
    };

    assert!(
        contains_user_text(&requests[0], first_user_message),
        "first turn request missing first user message"
    );
    assert!(
        !contains_user_text(&requests[0], compact_prompt()),
        "first turn request should not include summarization prompt"
    );

    assert!(
        contains_user_text(&requests[1], first_user_message),
        "first compact request should include history before compaction"
    );

    assert!(
        contains_user_text(&requests[2], second_user_message),
        "second turn request missing second user message"
    );
    assert!(
        contains_user_text(&requests[2], first_user_message),
        "second turn request should include the compacted user history"
    );

    assert!(
        contains_user_text(&requests[3], second_user_message),
        "second compact request should include latest history"
    );

    insta::assert_snapshot!(
        "manual_compact_with_history_shapes",
        format_labeled_requests_snapshot(
            "Manual /compact with prior user history compacts existing history and the follow-up turn includes the compact summary plus new user message.",
            &[
                ("Local Compaction Request", &requests[1]),
                ("Local Post-Compaction History Layout", &requests[2]),
            ]
        )
    );

    let first_compact_has_prompt = contains_user_text(&requests[1], compact_prompt());
    let second_compact_has_prompt = contains_user_text(&requests[3], compact_prompt());
    assert_eq!(
        first_compact_has_prompt, second_compact_has_prompt,
        "compact requests should consistently include or omit the summarization prompt"
    );

    let first_request_user_texts = requests[0].message_input_texts("user");
    let first_turn_user_index = first_request_user_texts
        .len()
        .checked_sub(1)
        .unwrap_or_else(|| panic!("first turn request missing user messages"));
    assert_eq!(
        first_request_user_texts[first_turn_user_index], first_user_message,
        "first turn request should end with the submitted user message"
    );
    let initial_seeded_user_prefix = &first_request_user_texts[..first_turn_user_index];

    let final_request_user_texts = requests
        .last()
        .unwrap_or_else(|| panic!("final turn request missing for {final_user_message}"))
        .message_input_texts("user");
    assert!(
        !initial_seeded_user_prefix.is_empty(),
        "first turn should include seeded user prefix before the submitted user message"
    );
    let (final_request_last_user_text, final_request_before_last_user) = final_request_user_texts
        .split_last()
        .unwrap_or_else(|| panic!("final turn request missing user messages"));
    assert_eq!(
        final_request_last_user_text, final_user_message,
        "final turn request should end with the submitted user message"
    );
    let history_before_seeded_prefix = final_request_before_last_user
        .strip_suffix(initial_seeded_user_prefix)
        .unwrap_or_else(|| {
            panic!(
                "final request should end with the seeded user prefix from the first request: {initial_seeded_user_prefix:?}"
            )
        });
    let expected_history = vec![
        first_user_message.to_string(),
        second_user_message.to_string(),
        expected_second_summary,
    ];
    assert_eq!(history_before_seeded_prefix, expected_history.as_slice());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_compaction_incomplete_max_output_tokens_does_not_replace_history() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let first_turn = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", /*total_tokens*/ 100),
    ]);
    let truncated_compaction = sse(vec![
        ev_assistant_message("m2", "TRUNCATED_SUMMARY"),
        ev_incomplete_with_tokens("r2", "max_output_tokens", /*total_tokens*/ 25),
    ]);
    let next_turn = sse(vec![
        ev_assistant_message("m3", "after failed compact"),
        ev_completed_with_tokens("r3", /*total_tokens*/ 100),
    ]);
    let request_log =
        mount_sse_sequence(&server, vec![first_turn, truncated_compaction, next_turn]).await;

    let mut model_provider = non_openai_model_provider(&server);
    model_provider.stream_max_retries = Some(0);
    let codex = test_codex()
        .with_config(move |config| {
            config.model_provider = model_provider;
            set_test_compact_prompt(config);
            config.model_auto_compact_token_limit = Some(200_000);
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

    codex.submit(Op::Compact).await.expect("run /compact");
    let error_message = wait_for_event_match(&codex, |event| match event {
        EventMsg::Error(err) => Some(err.message.clone()),
        _ => None,
    })
    .await;
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
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert_eq!(requests.len(), 3);
    assert!(
        error_message.contains("context compaction response incomplete: max_output_tokens"),
        "expected compaction incomplete error, got {error_message}"
    );

    let next_turn_body = requests[2].body_json().to_string();
    assert!(
        body_contains_text(&next_turn_body, "USER_ONE"),
        "failed compaction should leave pre-compact user history intact"
    );
    assert!(
        body_contains_text(&next_turn_body, FIRST_REPLY),
        "failed compaction should leave pre-compact assistant history intact"
    );
    assert!(
        body_contains_text(&next_turn_body, "USER_TWO"),
        "next user message should still be sent after failed compaction"
    );
    assert!(
        !body_contains_text(&next_turn_body, "TRUNCATED_SUMMARY"),
        "truncated compaction output must not be recorded as valid history"
    );
    assert!(
        !body_contains_compaction_summary_prefix(&next_turn_body),
        "failed compaction must not replace history with a summary"
    );
}
