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
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_compact_counts_encrypted_reasoning_before_last_user() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let first_user = "COUNT_PRE_LAST_REASONING";
    let second_user = "TRIGGER_COMPACT_AT_LIMIT";
    let third_user = "AFTER_REMOTE_COMPACT";

    let pre_last_reasoning_content = "a".repeat(2_400);
    let post_last_reasoning_content = "b".repeat(4_000);

    let first_turn = sse(vec![
        ev_reasoning_item("pre-reasoning", &["pre"], &[&pre_last_reasoning_content]),
        ev_completed_with_tokens("r1", /*total_tokens*/ 10),
    ]);
    let second_turn = sse(vec![
        ev_reasoning_item("post-reasoning", &["post"], &[&post_last_reasoning_content]),
        ev_completed_with_tokens("r2", /*total_tokens*/ 80),
    ]);
    let third_turn = sse(vec![
        ev_assistant_message("m4", FINAL_REPLY),
        ev_completed_with_tokens("r4", /*total_tokens*/ 1),
    ]);

    let request_log = mount_sse_sequence(
        &server,
        vec![
            // Turn 1: reasoning before last user (should count).
            first_turn,
            // Turn 2: reasoning after last user (should be ignored for compaction).
            second_turn,
            // Turn 3: next user turn after remote compaction.
            third_turn,
        ],
    )
    .await;

    let compacted_history = vec![
        codex_protocol::models::ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![codex_protocol::models::ContentItem::OutputText {
                text: "REMOTE_COMPACT_SUMMARY".to_string(),
            }],
            phase: None,
        },
        codex_protocol::models::ResponseItem::Compaction {
            encrypted_content: "ENCRYPTED_COMPACTION_SUMMARY".to_string(),
        },
    ];
    let compact_mock =
        mount_compact_json_once(&server, serde_json::json!({ "output": compacted_history })).await;
    let chatgpt_base_url = format!("{}/backend-api", server.uri());

    let codex = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(move |config| {
            config.chatgpt_base_url = chatgpt_base_url;
            set_test_compact_prompt(config);
            config.model_auto_compact_token_limit = Some(300);
        })
        .build(&server)
        .await
        .expect("build codex")
        .codex;

    for (idx, user) in [first_user, second_user, third_user]
        .into_iter()
        .enumerate()
    {
        codex
            .submit(Op::UserInput {
                environments: None,
                items: vec![UserInput::Text {
                    text: user.into(),
                    text_elements: Vec::new(),
                }],
                final_output_json_schema: None,
                responsesapi_client_metadata: None,
            })
            .await
            .unwrap();
        wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

        if idx < 2 {
            assert!(
                compact_mock.requests().is_empty(),
                "remote compaction should not run before the next user turn"
            );
        }
    }

    let compact_requests = compact_mock.requests();
    assert_eq!(
        compact_requests.len(),
        1,
        "remote compaction should run once after the second turn"
    );
    assert_eq!(
        compact_requests[0].path(),
        "/v1/responses/compact",
        "remote compaction should hit the compact endpoint"
    );

    let requests = request_log.requests();
    assert_eq!(
        requests.len(),
        3,
        "conversation should include three user turns"
    );
    let second_request_body = requests[1].body_json().to_string();
    assert!(
        !second_request_body.contains("REMOTE_COMPACT_SUMMARY"),
        "second turn should not include compacted history"
    );
    let third_request_body = requests[2].body_json().to_string();
    assert!(
        third_request_body.contains("REMOTE_COMPACT_SUMMARY")
            || third_request_body.contains(FINAL_REPLY),
        "third turn should include compacted history"
    );
    assert!(
        third_request_body.contains("ENCRYPTED_COMPACTION_SUMMARY"),
        "third turn should include compaction summary item"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_compact_runs_when_reasoning_header_clears_between_turns() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let first_user = "SERVER_INCLUDED_FIRST";
    let second_user = "SERVER_INCLUDED_SECOND";
    let third_user = "SERVER_INCLUDED_THIRD";

    let pre_last_reasoning_content = "a".repeat(2_400);
    let post_last_reasoning_content = "b".repeat(4_000);

    let first_turn = sse(vec![
        ev_reasoning_item("pre-reasoning", &["pre"], &[&pre_last_reasoning_content]),
        ev_completed_with_tokens("r1", /*total_tokens*/ 10),
    ]);
    let second_turn = sse(vec![
        ev_reasoning_item("post-reasoning", &["post"], &[&post_last_reasoning_content]),
        ev_completed_with_tokens("r2", /*total_tokens*/ 80),
    ]);
    let third_turn = sse(vec![
        ev_assistant_message("m4", FINAL_REPLY),
        ev_completed_with_tokens("r4", /*total_tokens*/ 1),
    ]);

    let responses = vec![
        sse_response(first_turn).insert_header("X-Reasoning-Included", "true"),
        sse_response(second_turn),
        sse_response(third_turn),
    ];
    mount_response_sequence(&server, responses).await;

    let compacted_history = vec![
        codex_protocol::models::ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![codex_protocol::models::ContentItem::OutputText {
                text: "REMOTE_COMPACT_SUMMARY".to_string(),
            }],
            phase: None,
        },
        codex_protocol::models::ResponseItem::Compaction {
            encrypted_content: "ENCRYPTED_COMPACTION_SUMMARY".to_string(),
        },
    ];
    let compact_mock =
        mount_compact_json_once(&server, serde_json::json!({ "output": compacted_history })).await;

    let codex = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(|config| {
            set_test_compact_prompt(config);
            config.model_auto_compact_token_limit = Some(300);
        })
        .build(&server)
        .await
        .expect("build codex")
        .codex;

    for user in [first_user, second_user, third_user] {
        codex
            .submit(Op::UserInput {
                environments: None,
                items: vec![UserInput::Text {
                    text: user.into(),
                    text_elements: Vec::new(),
                }],
                final_output_json_schema: None,
                responsesapi_client_metadata: None,
            })
            .await
            .unwrap();
        wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
    }

    let compact_requests = compact_mock.requests();
    assert_eq!(
        compact_requests.len(),
        1,
        "remote compaction should run once after the reasoning header clears"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// TODO(ccunningham): Update once pre-turn compaction includes incoming user input.
