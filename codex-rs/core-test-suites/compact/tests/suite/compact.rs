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
async fn manual_pre_compact_block_decision_does_not_block_compaction() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let first_turn = sse(vec![
        ev_assistant_message("m0", FIRST_REPLY),
        ev_completed_with_tokens("r0", /*total_tokens*/ 80),
    ]);
    let compact_turn = sse(vec![
        ev_assistant_message("m1", SUMMARY_TEXT),
        ev_completed_with_tokens("r1", /*total_tokens*/ 100),
    ]);
    let request_log = mount_sse_sequence(&server, vec![first_turn, compact_turn]).await;

    let model_provider = non_openai_model_provider(&server);
    let mut builder = test_codex()
        .with_pre_build_hook(write_unsupported_blocking_pre_compact_hook)
        .with_config(move |config| {
            config.model_provider = model_provider;
            trust_discovered_hooks(config);
            set_test_compact_prompt(config);
        });
    let test = builder.build(&server).await.expect("create conversation");
    let codex = test.codex.clone();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello before blocked compact".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submit first user turn");
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await.expect("trigger compact");

    let completed = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::HookCompleted(completed)
            if completed.run.event_name == HookEventName::PreCompact =>
        {
            Some(completed.clone())
        }
        _ => None,
    })
    .await;
    assert_eq!(completed.run.status, HookRunStatus::Failed);
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::Warning(_))).await;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert_eq!(
        requests.len(),
        2,
        "unsupported PreCompact block output should not prevent the compact request"
    );

    let hook_inputs = read_hook_inputs(&test.codex_home_path().join("pre_compact_block_log.jsonl"));
    assert_eq!(hook_inputs.len(), 1);
    let input = &hook_inputs[0];
    assert_eq!(input["hook_event_name"], "PreCompact");
    assert_eq!(input["trigger"], "manual");
    assert!(input.get("reason").is_none());
    assert!(input.get("phase").is_none());
    assert!(input.get("implementation").is_none());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_hooks_respect_matchers_and_post_runs_after_compaction() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let first_turn = sse(vec![
        ev_assistant_message("m0", FIRST_REPLY),
        ev_completed_with_tokens("r0", /*total_tokens*/ 80),
    ]);
    let compact_turn = sse(vec![
        ev_assistant_message("m1", SUMMARY_TEXT),
        ev_completed_with_tokens("r1", /*total_tokens*/ 100),
    ]);
    let request_log = mount_sse_sequence(&server, vec![first_turn, compact_turn]).await;

    let model_provider = non_openai_model_provider(&server);
    let mut builder = test_codex()
        .with_pre_build_hook(write_matching_compact_hooks)
        .with_config(move |config| {
            config.model_provider = model_provider;
            trust_discovered_hooks(config);
            set_test_compact_prompt(config);
        });
    let test = builder.build(&server).await.expect("create conversation");
    let codex = test.codex.clone();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello before matched compact".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submit first user turn");
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await.expect("trigger compact");
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::Warning(_))).await;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(request_log.requests().len(), 2);
    assert!(
        !test
            .codex_home_path()
            .join("pre_compact_auto_log.jsonl")
            .exists(),
        "auto matcher should not run for manual compaction"
    );

    let hook_inputs =
        read_hook_inputs(&test.codex_home_path().join("post_compact_manual_log.jsonl"));
    assert_eq!(hook_inputs.len(), 1);
    let input = &hook_inputs[0];
    assert_eq!(input["hook_event_name"], "PostCompact");
    assert_eq!(input["trigger"], "manual");
    assert!(input.get("compact_summary").is_none());
    assert!(input.get("status").is_none());
    assert!(input.get("error").is_none());
    assert!(input.get("reason").is_none());
    assert!(input.get("phase").is_none());
    assert!(input.get("implementation").is_none());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_compact_uses_custom_prompt() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let first_turn = sse(vec![
        ev_assistant_message("m0", FIRST_REPLY),
        ev_completed_with_tokens("r0", /*total_tokens*/ 80),
    ]);
    let compact_turn = sse(vec![
        ev_assistant_message("m1", SUMMARY_TEXT),
        ev_completed_with_tokens("r1", /*total_tokens*/ 100),
    ]);
    let request_log = mount_sse_sequence(&server, vec![first_turn, compact_turn]).await;

    let custom_prompt = "Use this compact prompt instead";

    let model_provider = non_openai_model_provider(&server);
    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        config.compact_prompt = Some(custom_prompt.to_string());
    });
    let codex = builder
        .build(&server)
        .await
        .expect("create conversation")
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
        .expect("submit first user turn");
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await.expect("trigger compact");
    let warning_event = wait_for_event(&codex, |ev| matches!(ev, EventMsg::Warning(_))).await;
    let EventMsg::Warning(WarningEvent { message }) = warning_event else {
        panic!("expected warning event after compact");
    };
    assert_eq!(message, COMPACT_WARNING_MESSAGE);
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert_eq!(
        requests.len(),
        2,
        "expected first turn and compact requests"
    );
    let body = requests[1].body_json();

    let input = body
        .get("input")
        .and_then(|v| v.as_array())
        .expect("input array");
    let mut found_custom_prompt = false;
    let mut found_default_prompt = false;

    for item in input {
        if item["type"].as_str() != Some("message") {
            continue;
        }
        let text = item["content"][0]["text"].as_str().unwrap_or_default();
        if text == custom_prompt {
            found_custom_prompt = true;
        }
        if text == compact_prompt() {
            found_default_prompt = true;
        }
    }

    let used_prompt = found_custom_prompt || found_default_prompt;
    if used_prompt {
        assert!(found_custom_prompt, "custom prompt should be injected");
        assert!(
            !found_default_prompt,
            "default prompt should be replaced when a compact prompt is used"
        );
    } else {
        assert!(
            !found_default_prompt,
            "summarization prompt should not appear if compaction omits a prompt"
        );
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_compact_emits_api_and_local_token_usage_events() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    // Compact run where the API reports zero tokens in usage. Our local
    // estimator should still compute a non-zero context size for the compacted
    // history.
    let sse_compact = sse(vec![
        ev_assistant_message("m1", SUMMARY_TEXT),
        ev_completed_with_tokens("r1", /*total_tokens*/ 0),
    ]);
    mount_sse_once(&server, sse_compact).await;

    let model_provider = non_openai_model_provider(&server);
    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        set_test_compact_prompt(config);
    });
    let codex = builder.build(&server).await.unwrap().codex;

    // Trigger manual compact and collect TokenCount events for the compact turn.
    codex.submit(Op::Compact).await.unwrap();

    // First TokenCount: from the compact API call (usage.total_tokens = 0).
    let first = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::TokenCount(tc) => tc
            .info
            .as_ref()
            .map(|info| info.last_token_usage.total_tokens),
        _ => None,
    })
    .await;

    // Second TokenCount: from the local post-compaction estimate.
    let last = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::TokenCount(tc) => tc
            .info
            .as_ref()
            .map(|info| info.last_token_usage.total_tokens),
        _ => None,
    })
    .await;

    // Ensure the compact task itself completes.
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(
        first, 0,
        "expected first TokenCount from compact API usage to be zero"
    );
    assert!(
        last > 0,
        "second TokenCount should reflect a non-zero estimated context size after compaction"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_compact_emits_context_compaction_items() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let sse1 = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed("r1"),
    ]);
    let sse2 = sse(vec![
        ev_assistant_message("m2", SUMMARY_TEXT),
        ev_completed("r2"),
    ]);
    mount_sse_sequence(&server, vec![sse1, sse2]).await;

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
                text: "manual compact".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await.unwrap();

    let mut started_item = None;
    let mut completed_item = None;
    let mut legacy_event = false;
    let mut saw_turn_complete = false;

    while !saw_turn_complete || started_item.is_none() || completed_item.is_none() || !legacy_event
    {
        let event = codex.next_event().await.unwrap();
        match event.msg {
            EventMsg::ItemStarted(ItemStartedEvent {
                item: TurnItem::ContextCompaction(item),
                ..
            }) => {
                started_item = Some(item);
            }
            EventMsg::ItemCompleted(ItemCompletedEvent {
                item: TurnItem::ContextCompaction(item),
                ..
            }) => {
                completed_item = Some(item);
            }
            EventMsg::ContextCompacted(_) => {
                legacy_event = true;
            }
            EventMsg::TurnComplete(_) => {
                saw_turn_complete = true;
            }
            _ => {}
        }
    }

    let started_item = started_item.expect("context compaction item started");
    let completed_item = completed_item.expect("context compaction item completed");
    assert_eq!(started_item.id, completed_item.id);
    assert!(legacy_event);
}
// Windows CI only: bump to 4 workers to prevent SSE/event starvation and test timeouts.
#[cfg_attr(windows, tokio::test(flavor = "multi_thread", worker_threads = 4))]
#[cfg_attr(not(windows), tokio::test(flavor = "multi_thread", worker_threads = 2))]
async fn auto_compact_emits_context_compaction_items() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let sse1 = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", /*total_tokens*/ 70_000),
    ]);
    let sse2 = sse(vec![
        ev_assistant_message("m2", "SECOND_REPLY"),
        ev_completed_with_tokens("r2", /*total_tokens*/ 330_000),
    ]);
    let sse3 = sse(vec![
        ev_assistant_message("m3", AUTO_SUMMARY_TEXT),
        ev_completed_with_tokens("r3", /*total_tokens*/ 200),
    ]);
    let sse4 = sse(vec![
        ev_assistant_message("m4", FINAL_REPLY),
        ev_completed_with_tokens("r4", /*total_tokens*/ 120),
    ]);

    mount_sse_sequence(&server, vec![sse1, sse2, sse3, sse4]).await;

    let model_provider = non_openai_model_provider(&server);
    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        set_test_compact_prompt(config);
        config.model_auto_compact_token_limit = Some(200_000);
    });
    let codex = builder.build(&server).await.unwrap().codex;

    let mut started_item = None;
    let mut completed_item = None;
    let mut legacy_event = false;

    for user in [FIRST_AUTO_MSG, SECOND_AUTO_MSG, POST_AUTO_USER_MSG] {
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

        loop {
            let event = codex.next_event().await.unwrap();
            match event.msg {
                EventMsg::ItemStarted(ItemStartedEvent {
                    item: TurnItem::ContextCompaction(item),
                    ..
                }) => {
                    started_item = Some(item);
                }
                EventMsg::ItemCompleted(ItemCompletedEvent {
                    item: TurnItem::ContextCompaction(item),
                    ..
                }) => {
                    completed_item = Some(item);
                }
                EventMsg::ContextCompacted(_) => {
                    legacy_event = true;
                }
                EventMsg::TurnComplete(_) if !event.id.starts_with("auto-compact-") => {
                    break;
                }
                _ => {}
            }
        }
    }

    let started_item = started_item.expect("context compaction item started");
    let completed_item = completed_item.expect("context compaction item completed");
    assert_eq!(started_item.id, completed_item.id);
    assert!(legacy_event);
}
// Windows CI only: bump to 4 workers to prevent SSE/event starvation and test timeouts.
#[cfg_attr(windows, tokio::test(flavor = "multi_thread", worker_threads = 4))]
#[cfg_attr(not(windows), tokio::test(flavor = "multi_thread", worker_threads = 2))]
async fn auto_compact_starts_after_turn_started() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let sse1 = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", /*total_tokens*/ 70_000),
    ]);
    let sse2 = sse(vec![
        ev_assistant_message("m2", "SECOND_REPLY"),
        ev_completed_with_tokens("r2", /*total_tokens*/ 330_000),
    ]);
    let sse3 = sse(vec![
        ev_assistant_message("m3", AUTO_SUMMARY_TEXT),
        ev_completed_with_tokens("r3", /*total_tokens*/ 200),
    ]);
    let sse4 = sse(vec![
        ev_assistant_message("m4", FINAL_REPLY),
        ev_completed_with_tokens("r4", /*total_tokens*/ 120),
    ]);

    mount_sse_sequence(&server, vec![sse1, sse2, sse3, sse4]).await;

    let model_provider = non_openai_model_provider(&server);
    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        set_test_compact_prompt(config);
        config.model_auto_compact_token_limit = Some(200_000);
    });
    let codex = builder.build(&server).await.unwrap().codex;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: FIRST_AUTO_MSG.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: SECOND_AUTO_MSG.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: POST_AUTO_USER_MSG.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    let first = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::TurnStarted(_) => Some("turn"),
        EventMsg::ItemStarted(ItemStartedEvent {
            item: TurnItem::ContextCompaction(_),
            ..
        }) => Some("compaction"),
        _ => None,
    })
    .await;
    assert_eq!(first, "turn", "compaction started before turn started");

    wait_for_event(&codex, |ev| {
        matches!(
            ev,
            EventMsg::ItemStarted(ItemStartedEvent {
                item: TurnItem::ContextCompaction(_),
                ..
            })
        )
    })
    .await;

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn semantic_auto_compact_defers_during_plan_mode() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let first_plan_user = "PLAN_MODE_FIRST_ITERATION";
    let second_plan_user = "PLAN_MODE_REVIEW_ITERATION";
    let implementation_user = "DEFAULT_MODE_IMPLEMENTATION";

    let request_log = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_assistant_message("m1", "FIRST_PLAN"),
                ev_completed_with_tokens("r1", /*total_tokens*/ 800),
            ]),
            sse(vec![
                ev_assistant_message("m2", "REVISED_PLAN"),
                ev_completed_with_tokens("r2", /*total_tokens*/ 20),
            ]),
            sse(vec![
                ev_assistant_message("m3", AUTO_SUMMARY_TEXT),
                ev_completed_with_tokens("r3", /*total_tokens*/ 20),
            ]),
            sse(vec![
                ev_assistant_message("m4", FINAL_REPLY),
                ev_completed_with_tokens("r4", /*total_tokens*/ 20),
            ]),
        ],
    )
    .await;

    let model_provider = non_openai_model_provider(&server);
    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        set_test_compact_prompt(config);
        config.model_auto_compact_token_limit = Some(1_000);
    });
    let test = builder.build(&server).await.unwrap();
    let model = test.session_configured.model.clone();

    for user in [first_plan_user, second_plan_user] {
        test.codex
            .submit(disabled_permission_plan_turn(
                user,
                test.cwd.path().to_path_buf(),
                model.clone(),
            ))
            .await
            .unwrap();
        wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
    }

    let plan_phase_requests: Vec<String> = request_log
        .requests()
        .into_iter()
        .map(|request| request.body_json().to_string())
        .collect();
    assert_eq!(
        plan_phase_requests.len(),
        2,
        "semantic auto-compact should not run between Plan-mode iterations"
    );
    assert!(plan_phase_requests[0].contains(first_plan_user));
    assert!(plan_phase_requests[1].contains(second_plan_user));
    assert!(
        plan_phase_requests
            .iter()
            .all(|body| !body_contains_text(body, compact_prompt())),
        "Plan-mode requests should not be semantic compaction requests"
    );

    test.codex
        .submit(disabled_permission_user_turn(
            implementation_user,
            test.cwd.path().to_path_buf(),
            model,
        ))
        .await
        .unwrap();
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests: Vec<String> = request_log
        .requests()
        .into_iter()
        .map(|request| request.body_json().to_string())
        .collect();
    assert_eq!(
        requests.len(),
        4,
        "semantic auto-compact should run once the next non-Plan turn starts"
    );
    assert!(
        body_contains_text(&requests[2], compact_prompt()),
        "third request should be deferred semantic compaction"
    );
    assert!(requests[3].contains(implementation_user));
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pre_sampling_compact_runs_on_switch_to_smaller_context_model() {
    skip_if_no_network!();

    let server = MockServer::start().await;
    let previous_model = "gpt-5.3-codex";
    let next_model = "gpt-5.2";

    let models_mock = mount_models_once(
        &server,
        ModelsResponse {
            models: vec![
                model_info_with_context_window(previous_model, /*context_window*/ 273_000),
                model_info_with_context_window(next_model, /*context_window*/ 125_000),
            ],
        },
    )
    .await;

    let request_log = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_assistant_message("m1", "before switch"),
                ev_completed_with_tokens("r1", /*total_tokens*/ 120_000),
            ]),
            sse(vec![
                ev_assistant_message("m2", "PRE_SAMPLING_SUMMARY"),
                ev_completed_with_tokens("r2", /*total_tokens*/ 10),
            ]),
            sse(vec![
                ev_assistant_message("m3", "after switch"),
                ev_completed_with_tokens("r3", /*total_tokens*/ 100),
            ]),
        ],
    )
    .await;

    let model_provider = non_openai_model_provider(&server);
    let mut builder = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_model(previous_model)
        .with_config(move |config| {
            config.model_provider = model_provider;
            set_test_compact_prompt(config);
        });
    let test = builder.build(&server).await.expect("build test codex");

    test.codex
        .submit(disabled_permission_user_turn(
            "before switch",
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
            "after switch",
            test.cwd.path().to_path_buf(),
            next_model.to_string(),
        ))
        .await
        .expect("submit second user turn");
    assert_compaction_uses_turn_lifecycle_id(&test.codex).await;

    let requests = request_log.requests();
    assert_eq!(models_mock.requests().len(), 1);
    assert_eq!(
        requests.len(),
        3,
        "expected user, compact, and follow-up requests"
    );
    assert_pre_sampling_switch_compaction_requests(
        &requests[0].body_json(),
        &requests[1].body_json(),
        &requests[2].body_json(),
        previous_model,
        next_model,
    );

    insta::assert_snapshot!(
        "pre_sampling_model_switch_compaction_shapes",
        format_labeled_requests_snapshot(
            "Pre-sampling compaction on model switch to a smaller context window: current behavior compacts using prior-turn history only (incoming user message excluded), and the follow-up request carries compacted history plus the new user message.",
            &[
                ("Initial Request (Previous Model)", &requests[0]),
                ("Pre-sampling Compaction Request", &requests[1]),
                (
                    "Post-Compaction Follow-up Request (Next Model)",
                    &requests[2]
                ),
            ]
        )
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_compact_persists_rollout_entries() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let sse1 = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", /*total_tokens*/ 70_000),
    ]);

    let sse2 = sse(vec![
        ev_assistant_message("m2", "SECOND_REPLY"),
        ev_completed_with_tokens("r2", /*total_tokens*/ 330_000),
    ]);

    let auto_summary_payload = auto_summary(AUTO_SUMMARY_TEXT);
    let sse3 = sse(vec![
        ev_assistant_message("m3", &auto_summary_payload),
        ev_completed_with_tokens("r3", /*total_tokens*/ 200),
    ]);
    let sse4 = sse(vec![
        ev_assistant_message("m4", FINAL_REPLY),
        ev_completed_with_tokens("r4", /*total_tokens*/ 120),
    ]);

    let first_matcher = |req: &wiremock::Request| {
        let body = std::str::from_utf8(&req.body).unwrap_or("");
        body.contains(FIRST_AUTO_MSG)
            && !body.contains(SECOND_AUTO_MSG)
            && !body_contains_text(body, compact_prompt())
    };
    mount_sse_once_match(&server, first_matcher, sse1).await;

    let second_matcher = |req: &wiremock::Request| {
        let body = std::str::from_utf8(&req.body).unwrap_or("");
        body.contains(SECOND_AUTO_MSG)
            && body.contains(FIRST_AUTO_MSG)
            && !body_contains_text(body, compact_prompt())
    };
    mount_sse_once_match(&server, second_matcher, sse2).await;

    let third_matcher = |req: &wiremock::Request| {
        let body = std::str::from_utf8(&req.body).unwrap_or("");
        body_contains_text(body, compact_prompt())
    };
    mount_sse_once_match(&server, third_matcher, sse3).await;

    let fourth_matcher = |req: &wiremock::Request| {
        let body = std::str::from_utf8(&req.body).unwrap_or("");
        body.contains(POST_AUTO_USER_MSG) && !body_contains_text(body, compact_prompt())
    };
    mount_sse_once_match(&server, fourth_matcher, sse4).await;

    let model_provider = non_openai_model_provider(&server);

    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        set_test_compact_prompt(config);
        config.model_auto_compact_token_limit = Some(200_000);
    });
    let test = builder.build(&server).await.unwrap();
    let codex = test.codex.clone();
    let session_configured = test.session_configured;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: FIRST_AUTO_MSG.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: SECOND_AUTO_MSG.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: POST_AUTO_USER_MSG.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Shutdown).await.unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::ShutdownComplete)).await;

    let rollout_path = session_configured.rollout_path.expect("rollout path");
    let text = std::fs::read_to_string(&rollout_path).unwrap_or_else(|e| {
        panic!(
            "failed to read rollout file {}: {e}",
            rollout_path.display()
        )
    });

    let mut turn_context_count = 0usize;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(entry): Result<RolloutLine, _> = serde_json::from_str(trimmed) else {
            continue;
        };
        match entry.item {
            RolloutItem::TurnContext(_) => {
                turn_context_count += 1;
            }
            RolloutItem::Compacted(_) => {}
            _ => {}
        }
    }

    assert_eq!(
        turn_context_count, 3,
        "rollout should contain one TurnContext entry per real user turn"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_compact_retries_after_context_window_error() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let user_turn = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed("r1"),
    ]);
    let compact_failed = sse_failed(
        "resp-fail",
        "context_length_exceeded",
        CONTEXT_LIMIT_MESSAGE,
    );
    let compact_succeeds = sse(vec![
        ev_assistant_message("m2", SUMMARY_TEXT),
        ev_completed("r2"),
    ]);

    let request_log = mount_sse_sequence(
        &server,
        vec![
            user_turn.clone(),
            compact_failed.clone(),
            compact_succeeds.clone(),
        ],
    )
    .await;

    let model_provider = non_openai_model_provider(&server);

    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        set_test_compact_prompt(config);
        config.model_auto_compact_token_limit = Some(200_000);
    });
    let codex = builder.build(&server).await.unwrap().codex;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "first turn".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await.unwrap();
    let warning_event = wait_for_event(&codex, |ev| matches!(ev, EventMsg::Warning(_))).await;
    let EventMsg::Warning(WarningEvent { message }) = warning_event else {
        panic!("expected warning event after compact retry");
    };
    assert_eq!(message, COMPACT_WARNING_MESSAGE);
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert_eq!(
        requests.len(),
        3,
        "expected user turn and two compact attempts"
    );

    let compact_attempt = requests[1].body_json();
    let retry_attempt = requests[2].body_json();

    let compact_input = compact_attempt["input"]
        .as_array()
        .unwrap_or_else(|| panic!("compact attempt missing input array: {compact_attempt}"));
    let retry_input = retry_attempt["input"]
        .as_array()
        .unwrap_or_else(|| panic!("retry attempt missing input array: {retry_attempt}"));
    let compact_contains_prompt =
        body_contains_text(&compact_attempt.to_string(), compact_prompt());
    let retry_contains_prompt = body_contains_text(&retry_attempt.to_string(), compact_prompt());
    assert_eq!(
        compact_contains_prompt, retry_contains_prompt,
        "compact attempts should consistently include or omit the summarization prompt"
    );
    assert_eq!(
        retry_input.len(),
        compact_input.len().saturating_sub(1),
        "retry should drop exactly one history item (before {} vs after {})",
        compact_input.len(),
        retry_input.len()
    );
    if let (Some(first_before), Some(first_after)) = (compact_input.first(), retry_input.first()) {
        assert_ne!(
            first_before, first_after,
            "retry should drop the oldest conversation item"
        );
    } else {
        panic!("expected non-empty compact inputs");
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// TODO(ccunningham): Re-enable after the follow-up compaction behavior PR lands.
// Current main behavior around non-context manual /compact failures is known-incorrect.
#[ignore = "behavior change covered in follow-up compaction PR"]
async fn manual_compact_non_context_failure_retries_then_emits_task_error() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let user_turn = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed("r1"),
    ]);
    let compact_failed_1 = sse_failed(
        "resp-fail-1",
        "server_error",
        "temporary compact failure one",
    );
    let compact_failed_2 = sse_failed(
        "resp-fail-2",
        "server_error",
        "temporary compact failure two",
    );

    mount_sse_sequence(&server, vec![user_turn, compact_failed_1, compact_failed_2]).await;

    let mut model_provider = non_openai_model_provider(&server);
    model_provider.stream_max_retries = Some(1);

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
                text: "first turn".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submit user input");
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await.expect("trigger compact");

    let reconnect_message = wait_for_event_match(&codex, |event| match event {
        EventMsg::StreamError(stream_error) => Some(stream_error.message.clone()),
        _ => None,
    })
    .await;
    assert!(
        reconnect_message.contains("Reconnecting... 1/1"),
        "expected reconnect stream error message, got {reconnect_message}"
    );

    let task_error_message = wait_for_event_match(&codex, |event| match event {
        EventMsg::Error(err) => Some(err.message.clone()),
        _ => None,
    })
    .await;
    assert!(
        task_error_message.contains("Error running local compact task"),
        "expected local compact task error prefix, got {task_error_message}"
    );
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_compact_allows_multiple_attempts_when_interleaved_with_other_turn_events() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let sse1 = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", /*total_tokens*/ 500),
    ]);
    let first_summary_payload = auto_summary(FIRST_AUTO_SUMMARY);
    let sse2 = sse(vec![
        ev_assistant_message("m2", &first_summary_payload),
        ev_completed_with_tokens("r2", /*total_tokens*/ 50),
    ]);
    let sse3 = sse(vec![
        ev_function_call(DUMMY_CALL_ID, DUMMY_FUNCTION_NAME, "{}"),
        ev_completed_with_tokens("r3", /*total_tokens*/ 150),
    ]);
    let sse4 = sse(vec![
        ev_assistant_message("m4", SECOND_LARGE_REPLY),
        ev_completed_with_tokens("r4", /*total_tokens*/ 450),
    ]);
    let second_summary_payload = auto_summary(SECOND_AUTO_SUMMARY);
    let sse5 = sse(vec![
        ev_assistant_message("m5", &second_summary_payload),
        ev_completed_with_tokens("r5", /*total_tokens*/ 60),
    ]);
    let sse6 = sse(vec![
        ev_assistant_message("m6", FINAL_REPLY),
        ev_completed_with_tokens("r6", /*total_tokens*/ 120),
    ]);
    let follow_up_user = "FOLLOW_UP_AUTO_COMPACT";
    let final_user = "FINAL_AUTO_COMPACT";

    let request_log = mount_sse_sequence(&server, vec![sse1, sse2, sse3, sse4, sse5, sse6]).await;

    let model_provider = non_openai_model_provider(&server);

    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        set_test_compact_prompt(config);
        config.model_auto_compact_token_limit = Some(200);
    });
    let codex = builder.build(&server).await.unwrap().codex;

    let mut auto_compact_lifecycle_events = Vec::new();
    for user in [MULTI_AUTO_MSG, follow_up_user, final_user] {
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

        loop {
            let event = codex.next_event().await.unwrap();
            if event.id.starts_with("auto-compact-")
                && matches!(
                    event.msg,
                    EventMsg::TurnStarted(_) | EventMsg::TurnComplete(_)
                )
            {
                auto_compact_lifecycle_events.push(event);
                continue;
            }
            if let EventMsg::TurnComplete(_) = &event.msg
                && !event.id.starts_with("auto-compact-")
            {
                break;
            }
        }
    }

    assert!(
        auto_compact_lifecycle_events.is_empty(),
        "auto compact should not emit task lifecycle events"
    );

    let request_bodies: Vec<String> = request_log
        .requests()
        .into_iter()
        .map(|request| request.body_json().to_string())
        .collect();
    assert_eq!(
        request_bodies.len(),
        6,
        "expected six requests including two auto compactions"
    );
    assert!(
        request_bodies[0].contains(MULTI_AUTO_MSG),
        "first request should contain the user input"
    );
    assert!(
        body_contains_text(&request_bodies[1], compact_prompt()),
        "first auto compact request should include the summarization prompt"
    );
    assert!(
        request_bodies[3].contains(&format!("unsupported call: {DUMMY_FUNCTION_NAME}")),
        "function call output should be sent before the second auto compact"
    );
    assert!(
        body_contains_text(&request_bodies[4], compact_prompt()),
        "second auto compact request should include the summarization prompt"
    );
}
