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

#[tokio::test]
async fn handle_responses_span_records_response_kind_and_tool_name() {
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
        sse(vec![
            ev_function_call("function-call", "nonexistent", "{\"value\":1}"),
            ev_completed("done"),
        ]),
    )
    .await;
    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "tool handled"),
            ev_completed("done"),
        ]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
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

    let logs = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();

    assert!(
        logs.lines().any(|line| {
            line.contains("handle_responses{")
                && line.contains("otel.name=\"function_call\"")
                && line.contains("tool_name=\"nonexistent\"")
                && line.contains("from=\"output_item_done\"")
        }),
        "missing handle_responses span with function call metadata\nlogs:\n{logs}"
    );
    assert!(
        logs.lines().any(|line| {
            line.contains("handle_responses{") && line.contains("otel.name=\"completed\"")
        }),
        "missing handle_responses span for completion\nlogs:\n{logs}"
    );
}
#[tokio::test(flavor = "current_thread")]
async fn record_responses_sets_span_fields_for_response_events() {
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

    let sse_body = sse(vec![
        ev_response_created("resp-1"),
        serde_json::json!({
            "type": "response.output_item.added",
            "item": {
                "type": "function_call",
                "call_id": "call-1",
                "name": "fn",
                "arguments": "{\"value\":1}"
            }
        }),
        ev_message_item_added("msg-added", "hi there"),
        ev_reasoning_item_added("reasoning-1", &["summary"]),
        ev_output_text_delta("delta"),
        ev_reasoning_summary_text_delta("summary-delta"),
        ev_reasoning_text_delta("raw-delta"),
        ev_function_call("call-1", "fn", "{\"key\":\"value\"}"),
        ev_assistant_message("msg-1", "agent"),
        ev_reasoning_item("reasoning-1", &["summary"], &[]),
        ev_completed("resp-1"),
    ]);

    mount_response_once(&server, sse_response(sse_body)).await;
    mount_response_once(
        &server,
        sse_response(sse(vec![
            ev_assistant_message("msg-2", "follow-up complete"),
            ev_completed("resp-2"),
        ])),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_model("gpt-5.4")
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

    let expected = [
        ("created", None::<&str>, None::<&str>),
        ("rate_limits", None, None),
        ("function_call", Some("output_item_added"), Some("fn")),
        ("message_from_assistant", Some("output_item_done"), None),
        ("reasoning", Some("output_item_done"), None),
        ("text_delta", None, None),
        ("reasoning_summary_delta", None, None),
        ("reasoning_content_delta", None, None),
        ("completed", None, None),
    ];

    for (name, from, tool_name) in expected {
        let otel_name = format!("otel.name=\"{name}\"");
        let from_field = from.map(|from| format!("from=\"{from}\""));
        let tool_name_field = tool_name.map(|tool_name| format!("tool_name=\"{tool_name}\""));

        assert!(
            logs.lines().any(|line| {
                line.contains("handle_responses{")
                    && line.contains(&otel_name)
                    && line.contains("codex.request.reasoning_effort=high")
                    && from_field
                        .as_ref()
                        .is_none_or(|from_field| line.contains(from_field))
                    && tool_name_field
                        .as_ref()
                        .is_none_or(|tool_name_field| line.contains(tool_name_field))
            }),
            "missing span fields for {name}\nlogs:\n{logs}"
        );
    }
}
#[tokio::test]
#[traced_test]
async fn handle_response_item_records_tool_result_for_custom_tool_call() {
    let server = start_mock_server().await;

    mount_sse_once(
        &server,
        sse(vec![
            ev_custom_tool_call(
                "custom-tool-call",
                "unsupported_tool",
                "{\"key\":\"value\"}",
            ),
            ev_completed("done"),
        ]),
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
        let line = lines
            .iter()
            .find(|line| {
                line.contains("codex.tool_result") && line.contains("call_id=custom-tool-call")
            })
            .ok_or_else(|| "missing codex.tool_result event".to_string())?;

        if !line.contains("tool_name=unsupported_tool") {
            return Err("missing tool_name field".to_string());
        }
        if !line.contains("arguments={\"key\":\"value\"}") {
            return Err("missing arguments field".to_string());
        }
        if !line.contains("output=unsupported custom tool call: unsupported_tool") {
            return Err("missing output field".to_string());
        }
        if !line.contains("success=false") {
            return Err("missing success field".to_string());
        }
        assert_empty_mcp_tool_fields(line)?;

        Ok(())
    });
}
#[tokio::test]
#[traced_test]
async fn handle_response_item_records_tool_result_for_function_call() {
    let server = start_mock_server().await;

    mount_sse_once(
        &server,
        sse(vec![
            ev_function_call("function-call", "nonexistent", "{\"value\":1}"),
            ev_completed("done"),
        ]),
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

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TokenCount(_))).await;

    logs_assert(|lines: &[&str]| {
        let line = lines
            .iter()
            .find(|line| {
                line.contains("codex.tool_result") && line.contains("call_id=function-call")
            })
            .ok_or_else(|| "missing codex.tool_result event".to_string())?;

        if !line.contains("tool_name=nonexistent") {
            return Err("missing tool_name field".to_string());
        }
        if !line.contains("arguments={\"value\":1}") {
            return Err("missing arguments field".to_string());
        }
        if !line.contains("output=unsupported call: nonexistent") {
            return Err("missing output field".to_string());
        }
        if !line.contains("success=false") {
            return Err("missing success field".to_string());
        }
        assert_empty_mcp_tool_fields(line)?;

        Ok(())
    });
}
#[tokio::test]
#[traced_test]
async fn handle_response_item_records_tool_result_for_shell_command_call() {
    let server = start_mock_server().await;

    mount_sse_once(
        &server,
        sse(vec![
            shell_command_call("shell-call", "echo shell"),
            ev_completed("done"),
        ]),
    )
    .await;

    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "shell command done"),
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
            config.permissions.approval_policy = Constrained::allow_any(AskForApproval::Never);
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
        let line = lines
            .iter()
            .find(|line| line.contains("codex.tool_result") && line.contains("call_id=shell-call"))
            .ok_or_else(|| "missing codex.tool_result event".to_string())?;

        if !line.contains("tool_name=shell_command") {
            return Err("missing tool_name field".to_string());
        }
        if !line.contains("arguments={\"command\":\"echo shell\"}") {
            return Err("missing arguments field".to_string());
        }
        let output_idx = line
            .find("output=")
            .ok_or_else(|| "missing output field".to_string())?;
        if line[output_idx + "output=".len()..].is_empty() {
            return Err("empty output field".to_string());
        }
        if !line.contains("success=false") {
            return Err("missing success field".to_string());
        }
        assert_empty_mcp_tool_fields(line)?;

        Ok(())
    });
}

fn tool_decision_assertion<'a>(
    call_id: &'a str,
    expected_decision: &'a str,
    expected_source: &'a str,
) -> impl Fn(&[&str]) -> Result<(), String> + 'a {
    let call_id = call_id.to_string();
    let expected_decision = expected_decision.to_string();
    let expected_source = expected_source.to_string();

    move |lines: &[&str]| {
        let line = lines
            .iter()
            .find(|line| {
                line.contains("codex.tool_decision") && line.contains(&format!("call_id={call_id}"))
            })
            .ok_or_else(|| format!("missing codex.tool_decision event for {call_id}"))?;

        let lower = line.to_lowercase();
        if !lower.contains("tool_name=shell_command") {
            return Err("missing tool_name for shell_command".to_string());
        }
        if !lower.contains(&format!("decision={expected_decision}")) {
            return Err(format!("unexpected decision for {call_id}"));
        }
        if !lower.contains(&format!("source={expected_source}")) {
            return Err(format!("unexpected source for {expected_source}"));
        }

        Ok(())
    }
}
