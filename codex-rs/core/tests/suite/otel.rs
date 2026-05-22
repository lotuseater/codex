use codex_core::config::Constrained;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_custom_tool_call;
use codex_core_test_runtime::responses::ev_function_call;
use codex_core_test_runtime::responses::ev_message_item_added;
use codex_core_test_runtime::responses::ev_output_text_delta;
use codex_core_test_runtime::responses::ev_reasoning_item;
use codex_core_test_runtime::responses::ev_reasoning_item_added;
use codex_core_test_runtime::responses::ev_reasoning_summary_text_delta;
use codex_core_test_runtime::responses::ev_reasoning_text_delta;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::mount_response_once;
use codex_core_test_runtime::responses::mount_sse_once;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::responses::sse_response;
use codex_core_test_runtime::responses::start_mock_server;
use codex_core_test_runtime::test_codex::TestCodex;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::wait_for_event;
use codex_features::Feature;
use codex_protocol::models::PermissionProfile;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::ReviewDecision;
use codex_protocol::user_input::UserInput;
use std::sync::Mutex;
use tracing::Level;
use tracing_test::traced_test;

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_test::internal::MockWriter;

fn extract_log_field(line: &str, key: &str) -> Option<String> {
    let quoted_prefix = format!("{key}=\"");
    if let Some(start) = line.find(&quoted_prefix) {
        let value_start = start + quoted_prefix.len();
        if let Some(end_rel) = line[value_start..].find('"') {
            return Some(line[value_start..value_start + end_rel].to_string());
        }
    }

    let bare_prefix = format!("{key}=");
    for token in line.split_whitespace() {
        let trimmed = token.trim_end_matches(',');
        if let Some(value) = trimmed.strip_prefix(&bare_prefix) {
            return Some(value.to_string());
        }
    }

    None
}

fn assert_empty_mcp_tool_fields(line: &str) -> Result<(), String> {
    let mcp_server = extract_log_field(line, "mcp_server")
        .ok_or_else(|| "missing mcp_server field".to_string())?;
    if !mcp_server.is_empty() {
        return Err(format!("expected empty mcp_server, got {mcp_server}"));
    }

    let mcp_server_origin = extract_log_field(line, "mcp_server_origin")
        .ok_or_else(|| "missing mcp_server_origin field".to_string())?;
    if !mcp_server_origin.is_empty() {
        return Err(format!(
            "expected empty mcp_server_origin, got {mcp_server_origin}"
        ));
    }

    Ok(())
}

fn shell_command_call(call_id: &str, command: &str) -> serde_json::Value {
    let args = serde_json::json!({ "command": command }).to_string();
    ev_function_call(call_id, "shell_command", &args)
}

fn touch_command(path: &str) -> String {
    if cfg!(windows) {
        format!("New-Item -ItemType File -Path {path} -Force | Out-Null")
    } else {
        format!("/usr/bin/touch {path}")
    }
}

#[test]
fn extract_log_field_handles_empty_bare_values() {
    let line = "event.name=\"codex.tool_result\" mcp_server= mcp_server_origin=";
    assert_eq!(extract_log_field(line, "mcp_server"), Some(String::new()));
    assert_eq!(
        extract_log_field(line, "mcp_server_origin"),
        Some(String::new())
    );
}
#[test]
fn extract_log_field_does_not_confuse_similar_keys() {
    let line = "event.name=\"codex.tool_result\" mcp_server_origin=stdio";
    assert_eq!(extract_log_field(line, "mcp_server"), None);
    assert_eq!(
        extract_log_field(line, "mcp_server_origin"),
        Some("stdio".to_string())
    );
}
#[tokio::test]
#[traced_test]
async fn responses_api_emits_api_request_event() {
    let server = start_mock_server().await;

    mount_sse_once(&server, sse(vec![ev_completed("done")])).await;

    let TestCodex { codex, .. } = test_codex().build(&server).await.unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| line.contains("codex.api_request"))
            .map(|_| Ok(()))
            .unwrap_or_else(|| Err("expected codex.api_request event".to_string()))
    });

    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| line.contains("codex.conversation_starts"))
            .map(|_| Ok(()))
            .unwrap_or_else(|| Err("expected codex.conversation_starts event".to_string()))
    });
}
#[tokio::test]
#[traced_test]
async fn process_sse_emits_tracing_for_output_item() {
    let server = start_mock_server().await;

    mount_sse_once(
        &server,
        sse(vec![ev_assistant_message("id1", "hi"), ev_completed("id2")]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex().build(&server).await.unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| {
                line.contains("codex.sse_event")
                    && line.contains("event.kind=response.output_item.done")
            })
            .map(|_| Ok(()))
            .unwrap_or(Err("missing response.output_item.done event".to_string()))
    });
}
#[tokio::test]
#[traced_test]
async fn process_sse_emits_failed_event_on_parse_error() {
    let server = start_mock_server().await;

    mount_sse_once(&server, "data: not-json\n\n".to_string()).await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(move |config| {
            config
                .features
                .disable(Feature::GhostCommit)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| {
                line.contains("codex.sse_event")
                    && line.contains("error.message")
                    && line.contains("expected ident at line 1 column 2")
            })
            .map(|_| Ok(()))
            .unwrap_or(Err("missing codex.sse_event".to_string()))
    });
}
#[tokio::test]
#[traced_test]
async fn process_sse_records_failed_event_when_stream_closes_without_completed() {
    let server = start_mock_server().await;

    mount_sse_once(&server, sse(vec![ev_assistant_message("id", "hi")])).await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(move |config| {
            config
                .features
                .disable(Feature::GhostCommit)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| {
                line.contains("codex.sse_event")
                    && line.contains("error.message")
                    && line.contains("stream closed before response.completed")
            })
            .map(|_| Ok(()))
            .unwrap_or(Err("missing codex.sse_event".to_string()))
    });
}
#[tokio::test]
#[traced_test]
async fn process_sse_failed_event_records_response_error_message() {
    let server = start_mock_server().await;

    mount_sse_once(
        &server,
        sse(vec![serde_json::json!({
            "type": "response.failed",
            "response": {
                "error": {
                    "message": "boom",
                    "code": "bad"
                }
            }
        })]),
    )
    .await;
    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "local shell done"),
            ev_completed("done"),
        ]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(move |config| {
            config
                .features
                .disable(Feature::GhostCommit)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| {
                line.contains("codex.sse_event")
                    && line.contains("event.kind=response.failed")
                    && line.contains("error.message")
                    && line.contains("boom")
            })
            .map(|_| Ok(()))
            .unwrap_or(Err("missing codex.sse_event".to_string()))
    });
}
#[tokio::test]
#[traced_test]
async fn process_sse_failed_event_logs_parse_error() {
    let server = start_mock_server().await;

    mount_sse_once(
        &server,
        sse(vec![serde_json::json!({
            "type": "response.failed",
            "response": {
                "error": "not-an-object"
            }
        })]),
    )
    .await;
    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "local shell done"),
            ev_completed("done"),
        ]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(move |config| {
            config
                .features
                .disable(Feature::GhostCommit)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| {
                line.contains("codex.sse_event") && line.contains("event.kind=response.failed")
            })
            .map(|_| Ok(()))
            .unwrap_or(Err("missing codex.sse_event".to_string()))
    });
}
#[tokio::test]
#[traced_test]
async fn process_sse_failed_event_logs_missing_error() {
    let server = start_mock_server().await;

    mount_sse_once(
        &server,
        sse(vec![serde_json::json!({
            "type": "response.failed",
            "response": {}
        })]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(move |config| {
            config
                .features
                .disable(Feature::GhostCommit)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| {
                line.contains("codex.sse_event") && line.contains("event.kind=response.failed")
            })
            .map(|_| Ok(()))
            .unwrap_or(Err("missing codex.sse_event".to_string()))
    });
}
#[tokio::test]
#[traced_test]
async fn process_sse_failed_event_logs_response_completed_parse_error() {
    let server = start_mock_server().await;

    mount_sse_once(
        &server,
        sse(vec![serde_json::json!({
            "type": "response.completed",
            "response": {}
        })]),
    )
    .await;

    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "local shell done"),
            ev_completed("done"),
        ]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(move |config| {
            config
                .features
                .disable(Feature::GhostCommit)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await
        .unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| {
                line.contains("codex.sse_event")
                    && line.contains("event.kind=response.completed")
                    && line.contains("error.message")
                    && line.contains("failed to parse ResponseCompleted")
            })
            .map(|_| Ok(()))
            .unwrap_or(Err("missing codex.sse_event".to_string()))
    });
}
#[tokio::test]
#[traced_test]
async fn process_sse_emits_completed_telemetry() {
    let server = start_mock_server().await;

    mount_sse_once(
        &server,
        sse(vec![serde_json::json!({
            "type": "response.completed",
            "response": {
                "id": "resp1",
                "usage": {
                    "input_tokens": 3,
                    "input_tokens_details": { "cached_tokens": 1 },
                    "output_tokens": 5,
                    "output_tokens_details": { "reasoning_tokens": 2 },
                    "total_tokens": 9
                }
            }
        })]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex().build(&server).await.unwrap();

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| {
                line.contains("codex.sse_event")
                    && line.contains("event.kind=response.completed")
                    && line.contains("input_token_count=3")
                    && line.contains("output_token_count=5")
                    && line.contains("cached_token_count=1")
                    && line.contains("reasoning_token_count=2")
                    && line.contains("tool_token_count=9")
            })
            .map(|_| Ok(()))
            .unwrap_or(Err("missing response.completed telemetry".to_string()))
    });
}
#[tokio::test(flavor = "current_thread")]
async fn turn_and_completed_response_spans_record_token_usage() {
    let buffer: &'static Mutex<Vec<u8>> = Box::leak(Box::new(Mutex::new(Vec::new())));
    let subscriber = tracing_subscriber::fmt()
        .with_level(true)
        .with_ansi(false)
        .with_max_level(Level::TRACE)
        .with_span_events(FmtSpan::FULL)
        .with_writer(MockWriter::new(buffer))
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let server = start_mock_server().await;

    mount_sse_once(
        &server,
        sse(vec![serde_json::json!({
            "type": "response.completed",
            "response": {
                "id": "resp1",
                "usage": {
                    "input_tokens": 3,
                    "input_tokens_details": { "cached_tokens": 1 },
                    "output_tokens": 5,
                    "output_tokens_details": { "reasoning_tokens": 2 },
                    "total_tokens": 9
                }
            }
        })]),
    )
    .await;

    let test = test_codex()
        .with_config(|config| {
            config.model_reasoning_effort = Some(ReasoningEffort::High);
            config
                .features
                .disable(Feature::GhostCommit)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await
        .unwrap();

    let TestCodex { codex, .. } = test;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let logs = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();

    assert!(
        logs.lines().any(|line| {
            line.contains("handle_responses{")
                && line.contains("otel.name=\"completed\"")
                && line.contains("codex.request.reasoning_effort=high")
                && line.contains("gen_ai.usage.input_tokens=3")
                && line.contains("gen_ai.usage.cache_read.input_tokens=1")
                && line.contains("gen_ai.usage.output_tokens=5")
                && line.contains("codex.usage.reasoning_output_tokens=2")
                && line.contains("codex.usage.total_tokens=9")
        }),
        "missing completed response span token usage\nlogs:\n{logs}"
    );
    assert!(
        logs.lines().any(|line| {
            line.contains("turn{otel.name=\"session_task.turn\"")
                && line.contains("codex.turn.reasoning_effort=high")
                && line.contains("codex.turn.token_usage.input_tokens=3")
                && line.contains("codex.turn.token_usage.cached_input_tokens=1")
                && line.contains("codex.turn.token_usage.non_cached_input_tokens=2")
                && line.contains("codex.turn.token_usage.output_tokens=5")
                && line.contains("codex.turn.token_usage.reasoning_output_tokens=2")
                && line.contains("codex.turn.token_usage.total_tokens=9")
        }),
        "missing regular turn span token usage\nlogs:\n{logs}"
    );
}
