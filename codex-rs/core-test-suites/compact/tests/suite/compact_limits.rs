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
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multiple_auto_compact_per_task_runs_after_token_limit_hit() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let non_openai_provider_name = non_openai_model_provider(&server).name;
    let codex = test_codex()
        .with_config(move |config| {
            config.model_provider.name = non_openai_provider_name;
        })
        .build(&server)
        .await
        .expect("build codex")
        .codex;

    // user message
    let user_message = "create an app";

    // Prepare the mock responses from the model

    // summary texts from model
    let first_summary_text = "The task is to create an app. I started to create a react app.";
    let second_summary_text = "The task is to create an app. I started to create a react app. then I realized that I need to create a node app.";
    let third_summary_text = "The task is to create an app. I started to create a react app. then I realized that I need to create a node app. then I realized that I need to create a python app.";
    // summary texts with prefix
    let prefixed_first_summary = summary_with_prefix(first_summary_text);
    let prefixed_second_summary = summary_with_prefix(second_summary_text);
    let prefixed_third_summary = summary_with_prefix(third_summary_text);
    // token used count after long work
    let token_count_used = 270_000;
    // token used count after compaction
    let token_count_used_after_compaction = 80000;

    // mock responses from the model

    let reasoning_response_1 = ev_reasoning_item("m1", &["I will create a react app"], &[]);
    let encrypted_content_1 = reasoning_response_1["item"]["encrypted_content"]
        .as_str()
        .unwrap();

    // first chunk of work
    let model_reasoning_response_1_sse = sse(vec![
        reasoning_response_1.clone(),
        ev_shell_command_call("r1-shell", "echo make-react"),
        ev_completed_with_tokens("r1", token_count_used),
    ]);

    // first compaction response
    let model_compact_response_1_sse = sse(vec![
        ev_assistant_message("m2", first_summary_text),
        ev_completed_with_tokens("r2", token_count_used_after_compaction),
    ]);

    let reasoning_response_2 = ev_reasoning_item("m3", &["I will create a node app"], &[]);
    let encrypted_content_2 = reasoning_response_2["item"]["encrypted_content"]
        .as_str()
        .unwrap();

    // second chunk of work
    let model_reasoning_response_2_sse = sse(vec![
        reasoning_response_2.clone(),
        ev_shell_command_call("r3-shell", "echo make-node"),
        ev_completed_with_tokens("r3", token_count_used),
    ]);

    // second compaction response
    let model_compact_response_2_sse = sse(vec![
        ev_assistant_message("m4", second_summary_text),
        ev_completed_with_tokens("r4", token_count_used_after_compaction),
    ]);

    let reasoning_response_3 = ev_reasoning_item("m6", &["I will create a python app"], &[]);
    let encrypted_content_3 = reasoning_response_3["item"]["encrypted_content"]
        .as_str()
        .unwrap();

    // third chunk of work
    let model_reasoning_response_3_sse = sse(vec![
        ev_reasoning_item("m6", &["I will create a python app"], &[]),
        ev_shell_command_call("r6-shell", "echo make-python"),
        ev_completed_with_tokens("r6", token_count_used),
    ]);

    // third compaction response
    let model_compact_response_3_sse = sse(vec![
        ev_assistant_message("m7", third_summary_text),
        ev_completed_with_tokens("r7", token_count_used_after_compaction),
    ]);

    // final response
    let model_final_response_sse = sse(vec![
        ev_assistant_message(
            "m8",
            "The task is to create an app. I started to create a react app. then I realized that I need to create a node app. then I realized that I need to create a python app.",
        ),
        ev_completed_with_tokens("r8", token_count_used_after_compaction + 1000),
    ]);

    // mount the mock responses from the model
    let bodies = vec![
        model_reasoning_response_1_sse,
        model_compact_response_1_sse,
        model_reasoning_response_2_sse,
        model_compact_response_2_sse,
        model_reasoning_response_3_sse,
        model_compact_response_3_sse,
        model_final_response_sse,
    ];
    let request_log = mount_sse_sequence(&server, bodies).await;

    // Start the conversation with the user message
    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: user_message.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submit user input");
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    // collect the requests payloads from the model
    let requests_payloads = request_log.requests();
    let body = requests_payloads[0].body_json();
    let input = body.get("input").and_then(|v| v.as_array()).unwrap();

    fn strip_agents_parts_from_user_message(
        value: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        let content = value
            .get("content")
            .and_then(|content| content.as_array())?;
        let filtered_content = content
            .iter()
            .filter(|item| {
                !item
                    .get("text")
                    .and_then(|text| text.as_str())
                    .is_some_and(|text| text.starts_with("# AGENTS.md instructions for "))
            })
            .cloned()
            .collect::<Vec<_>>();
        if filtered_content.is_empty() {
            return None;
        }
        let mut normalized = value.clone();
        normalized["content"] = serde_json::Value::Array(filtered_content);
        Some(normalized)
    }

    fn normalize_inputs(values: &[serde_json::Value]) -> Vec<serde_json::Value> {
        values
            .iter()
            .filter_map(|value| {
                if value
                    .get("type")
                    .and_then(|ty| ty.as_str())
                    .is_some_and(|ty| ty == "function_call_output")
                {
                    return None;
                }

                let text = value
                    .get("content")
                    .and_then(|content| content.as_array())
                    .and_then(|content| content.first())
                    .and_then(|item| item.get("text"))
                    .and_then(|text| text.as_str());

                // Ignore cached prefix messages (project docs + permissions) since they are not
                // relevant to compaction behavior and can change as bundled prompts evolve.
                let role = value.get("role").and_then(|role| role.as_str());
                if role == Some("developer")
                    && text.is_some_and(|text| text.contains("`sandbox_mode`"))
                {
                    return None;
                }
                if role == Some("user") {
                    return strip_agents_parts_from_user_message(value);
                }
                Some(value.clone())
            })
            .collect()
    }

    let initial_input = normalize_inputs(input);
    let environment_message = initial_input[0]["content"][0]["text"].as_str().unwrap();

    // test 1: after compaction, we should have one environment message, one user message, and one user message with summary prefix
    let compaction_indices = [2, 4, 6];
    let expected_summaries = [
        prefixed_first_summary.as_str(),
        prefixed_second_summary.as_str(),
        prefixed_third_summary.as_str(),
    ];
    for (i, expected_summary) in compaction_indices.into_iter().zip(expected_summaries) {
        let body = requests_payloads.clone()[i].body_json();
        let input = body.get("input").and_then(|v| v.as_array()).unwrap();
        let input = normalize_inputs(input);
        assert_eq!(input.len(), 3);
        let environment_message = input[0]["content"][0]["text"].as_str().unwrap();
        let user_message_received = input[1]["content"][0]["text"].as_str().unwrap();
        let summary_message = input[2]["content"][0]["text"].as_str().unwrap();
        assert_eq!(environment_message, environment_message);
        assert_eq!(user_message_received, user_message);
        assert_eq!(
            summary_message, expected_summary,
            "compaction request at index {i} should include the prefixed summary"
        );
    }

    // test 2: the expected requests inputs should be as follows:
    let expected_requests_inputs = json!([
    [
        // 0: first request of the user message.
      {
        "content": [
          {
            "text": environment_message,
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": "create an app",
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      }
    ]
    ,
    [
        // 1: first automatic compaction request.
      {
        "content": [
          {
            "text": environment_message,
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": "create an app",
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": null,
        "encrypted_content": encrypted_content_1,
        "summary": [
          {
            "text": "I will create a react app",
            "type": "summary_text"
          }
        ],
        "type": "reasoning"
      },
      {
        "arguments": "{\"command\":\"echo make-react\"}",
        "call_id": "r1-shell",
        "name": "shell_command",
        "type": "function_call"
      },
      {
        "call_id": "r1-shell",
        "output": "execution error: Io(Os { code: 2, kind: NotFound, message: \"No such file or directory\" })",
        "type": "function_call_output"
      },
      {
        "content": [
          {
            "text": compact_prompt(),
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      }
    ]
    ,
    [
      // 2: request after first automatic compaction.
      {
        "content": [
          {
            "text": environment_message,
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": "create an app",
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": prefixed_first_summary.clone(),
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      }
    ]
    ,
    [
        // 3: request for second automatic compaction.
      {
        "content": [
          {
            "text": environment_message,
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": "create an app",
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": prefixed_first_summary.clone(),
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": null,
        "encrypted_content": encrypted_content_2,
        "summary": [
          {
            "text": "I will create a node app",
            "type": "summary_text"
          }
        ],
        "type": "reasoning"
      },
      {
        "arguments": "{\"command\":\"echo make-node\"}",
        "call_id": "r3-shell",
        "name": "shell_command",
        "type": "function_call"
      },
      {
        "call_id": "r3-shell",
        "output": "execution error: Io(Os { code: 2, kind: NotFound, message: \"No such file or directory\" })",
        "type": "function_call_output"
      },
      {
        "content": [
          {
            "text": compact_prompt(),
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      }
    ]
    ,
    // 4: request after second automatic compaction.
    [
      {
        "content": [
          {
            "text": environment_message,
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": "create an app",
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": prefixed_second_summary.clone(),
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      }
    ]
    ,
    [
      // 5: request for third automatic compaction.
      {
        "content": [
          {
            "text": environment_message,
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": "create an app",
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": prefixed_second_summary.clone(),
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": null,
        "encrypted_content": encrypted_content_3,
        "summary": [
          {
            "text": "I will create a python app",
            "type": "summary_text"
          }
        ],
        "type": "reasoning"
      },
      {
        "arguments": "{\"command\":\"echo make-python\"}",
        "call_id": "r6-shell",
        "name": "shell_command",
        "type": "function_call"
      },
      {
        "call_id": "r6-shell",
        "output": "execution error: Io(Os { code: 2, kind: NotFound, message: \"No such file or directory\" })",
        "type": "function_call_output"
      },
      {
        "content": [
          {
            "text": compact_prompt(),
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      }
    ]
    ,
    [
      {
        // 6: request after third automatic compaction.
        "content": [
          {
            "text": environment_message,
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": "create an app",
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      },
      {
        "content": [
          {
            "text": prefixed_third_summary.clone(),
            "type": "input_text"
          }
        ],
        "role": "user",
        "type": "message"
      }
    ]
    ]);

    for (i, request) in requests_payloads.iter().enumerate() {
        let body = request.body_json();
        let input = body.get("input").and_then(|v| v.as_array()).unwrap();
        let expected_input = expected_requests_inputs[i].as_array().unwrap();
        assert_eq!(normalize_inputs(input), normalize_inputs(expected_input));
    }

    // test 3: the number of requests should be 7
    assert_eq!(requests_payloads.len(), 7);
} // Windows CI only: bump to 4 workers to prevent SSE/event starvation and test timeouts.
#[cfg_attr(windows, tokio::test(flavor = "multi_thread", worker_threads = 4))]
#[cfg_attr(not(windows), tokio::test(flavor = "multi_thread", worker_threads = 2))]
async fn auto_compact_runs_after_token_limit_hit() {
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
    let prefixed_auto_summary = AUTO_SUMMARY_TEXT;

    let request_log = mount_sse_sequence(&server, vec![sse1, sse2, sse3, sse4]).await;

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

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    let request_bodies: Vec<String> = requests
        .iter()
        .map(|request| request.body_json().to_string())
        .collect();
    assert_eq!(
        request_bodies.len(),
        4,
        "expected user turns, a compaction request, and the follow-up turn; got {}",
        request_bodies.len()
    );
    let auto_compact_count = request_bodies
        .iter()
        .filter(|body| body_contains_text(body, compact_prompt()))
        .count();
    assert_eq!(
        auto_compact_count, 1,
        "expected exactly one auto compact request"
    );
    let auto_compact_index = request_bodies
        .iter()
        .enumerate()
        .find_map(|(idx, body)| body_contains_text(body, compact_prompt()).then_some(idx))
        .expect("auto compact request missing");
    assert_eq!(
        auto_compact_index, 2,
        "auto compact should add a third request"
    );

    let follow_up_index = request_bodies
        .iter()
        .enumerate()
        .rev()
        .find_map(|(idx, body)| {
            (body.contains(POST_AUTO_USER_MSG) && !body_contains_text(body, compact_prompt()))
                .then_some(idx)
        })
        .expect("follow-up request missing");
    assert_eq!(follow_up_index, 3, "follow-up request should be last");

    let body_first = requests[0].body_json();
    let body_auto = requests[auto_compact_index].body_json();
    let body_follow_up = requests[follow_up_index].body_json();
    let instructions = body_auto
        .get("instructions")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let baseline_instructions = body_first
        .get("instructions")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    assert_eq!(
        instructions, baseline_instructions,
        "auto compact should keep the standard developer instructions",
    );

    let input_auto = body_auto.get("input").and_then(|v| v.as_array()).unwrap();
    let last_auto = input_auto
        .last()
        .expect("auto compact request should append a user message");
    assert_eq!(
        last_auto.get("type").and_then(|v| v.as_str()),
        Some("message")
    );
    assert_eq!(last_auto.get("role").and_then(|v| v.as_str()), Some("user"));
    let last_text = last_auto
        .get("content")
        .and_then(|v| v.as_array())
        .and_then(|items| items.first())
        .and_then(|item| item.get("text"))
        .and_then(|text| text.as_str())
        .unwrap_or_default();
    assert_eq!(
        last_text,
        compact_prompt(),
        "auto compact should send the summarization prompt as a user message",
    );

    let input_follow_up = body_follow_up
        .get("input")
        .and_then(|v| v.as_array())
        .unwrap();
    let user_texts: Vec<String> = input_follow_up
        .iter()
        .filter(|item| item.get("type").and_then(|v| v.as_str()) == Some("message"))
        .filter(|item| item.get("role").and_then(|v| v.as_str()) == Some("user"))
        .filter_map(|item| {
            item.get("content")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|entry| entry.get("text"))
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string)
        })
        .collect();
    assert!(
        user_texts.iter().any(|text| text == FIRST_AUTO_MSG),
        "auto compact follow-up request should include the first user message"
    );
    assert!(
        user_texts.iter().any(|text| text == SECOND_AUTO_MSG),
        "auto compact follow-up request should include the second user message"
    );
    assert!(
        user_texts.iter().any(|text| text == POST_AUTO_USER_MSG),
        "auto compact follow-up request should include the new user message"
    );
    assert!(
        user_texts
            .iter()
            .any(|text| text.contains(prefixed_auto_summary)),
        "auto compact follow-up request should include the summary message"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_compact_clamps_config_limit_to_context_window() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let context_window = 100;
    let config_limit = 200;
    let over_limit_tokens = context_window * 90 / 100 + 1;

    let first_turn = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", over_limit_tokens),
    ]);
    let auto_summary_payload = auto_summary(AUTO_SUMMARY_TEXT);
    let auto_compact_turn = sse(vec![
        ev_assistant_message("m2", &auto_summary_payload),
        ev_completed_with_tokens("r2", /*total_tokens*/ 10),
    ]);
    let post_auto_compact_turn = sse(vec![ev_completed_with_tokens(
        "r3", /*total_tokens*/ 10,
    )]);

    let first_turn_mock = mount_sse_once(&server, first_turn).await;
    let auto_compact_mock = mount_sse_once(&server, auto_compact_turn).await;
    mount_sse_once(&server, post_auto_compact_turn).await;

    let model_provider = non_openai_model_provider(&server);
    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        set_test_compact_prompt(config);
        config.model_context_window = Some(context_window);
        config.model_auto_compact_token_limit = Some(config_limit);
    });
    let codex = builder.build(&server).await.unwrap();

    codex.submit_turn("OVER_LIMIT_TURN").await.unwrap();
    codex.submit_turn("FOLLOW_UP_AFTER_CLAMP").await.unwrap();

    assert!(
        first_turn_mock.single_request().input().iter().any(|item| {
            item.get("type").and_then(|value| value.as_str()) == Some("message")
                && item
                    .get("content")
                    .and_then(|content| content.as_array())
                    .and_then(|entries| entries.first())
                    .and_then(|entry| entry.get("text"))
                    .and_then(|value| value.as_str())
                    == Some("OVER_LIMIT_TURN")
        }),
        "first request should contain the over-limit user input"
    );

    let auto_compact_body = auto_compact_mock.single_request().body_json().to_string();
    assert!(
        body_contains_compaction_prompt(&auto_compact_body),
        "auto compact should run with the summarization prompt when config limit exceeds context"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_compact_uses_slow_context_budget_after_turn() {
    skip_if_no_network!();

    let server = start_mock_server().await;

    let context_window = 100;
    let config_limit = 200;
    let over_slow_limit_tokens = 80;

    let first_turn = sse(vec![
        ev_assistant_message("m1", FIRST_REPLY),
        ev_completed_with_tokens("r1", over_slow_limit_tokens),
    ]);
    let auto_summary_payload = auto_summary(AUTO_SUMMARY_TEXT);
    let auto_compact_turn = sse(vec![
        ev_assistant_message("m2", &auto_summary_payload),
        ev_completed_with_tokens("r2", /*total_tokens*/ 10),
    ]);
    let post_auto_compact_turn = sse(vec![ev_completed_with_tokens(
        "r3", /*total_tokens*/ 10,
    )]);

    let first_turn_mock = mount_sse_once(&server, first_turn).await;
    let auto_compact_mock = mount_sse_once(&server, auto_compact_turn).await;
    mount_sse_once(&server, post_auto_compact_turn).await;

    let model_provider = non_openai_model_provider(&server);
    let mut builder = test_codex().with_config(move |config| {
        config.model_provider = model_provider;
        set_test_compact_prompt(config);
        config.context_budget_mode = ContextBudgetMode::Slow;
        config.model_context_window = Some(context_window);
        config.model_auto_compact_token_limit = Some(config_limit);
    });
    let codex = builder.build(&server).await.unwrap();

    codex.submit_turn("SLOW_BUDGET_OVER_LIMIT").await.unwrap();
    codex
        .submit_turn("FOLLOW_UP_AFTER_SLOW_BUDGET")
        .await
        .unwrap();

    assert!(
        first_turn_mock.single_request().input().iter().any(|item| {
            item.get("type").and_then(|value| value.as_str()) == Some("message")
                && item
                    .get("content")
                    .and_then(|content| content.as_array())
                    .and_then(|entries| entries.first())
                    .and_then(|entry| entry.get("text"))
                    .and_then(|value| value.as_str())
                    == Some("SLOW_BUDGET_OVER_LIMIT")
        }),
        "first request should contain the slow-budget over-limit user input"
    );

    let auto_compact_body = auto_compact_mock.single_request().body_json().to_string();
    assert!(
        body_contains_compaction_prompt(&auto_compact_body),
        "auto compact should use the Slow-mode post-turn limit"
    );
}
