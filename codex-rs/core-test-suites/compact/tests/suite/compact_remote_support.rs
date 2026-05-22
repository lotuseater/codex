#![allow(clippy::expect_used)]

pub(crate) use std::fs;
pub(crate) use std::path::PathBuf;

pub(crate) use anyhow::Result;
pub(crate) use codex_core_test_runtime::compact_fixtures::summary_with_prefix;
pub(crate) use codex_test_support_responses::context_snapshot;
pub(crate) use codex_test_support_responses::context_snapshot::ContextSnapshotOptions;
pub(crate) use codex_test_support_responses::context_snapshot::ContextSnapshotRenderMode;
pub(crate) use codex_core_test_runtime::responses;
pub(crate) use codex_core_test_runtime::responses::mount_sse_once;
pub(crate) use codex_core_test_runtime::responses::sse;
pub(crate) use codex_core_test_runtime::responses::start_websocket_server;
pub(crate) use codex_core_test_runtime::skip_if_no_network;
pub(crate) use codex_core_test_runtime::test_codex::TestCodexBuilder;
pub(crate) use codex_core_test_runtime::test_codex::TestCodexHarness;
pub(crate) use codex_core_test_runtime::test_codex::test_codex;
pub(crate) use codex_core_test_runtime::wait_for_event;
pub(crate) use codex_core_test_runtime::wait_for_event_match;
pub(crate) use codex_core_test_runtime::wait_for_event_with_timeout;
pub(crate) use codex_features::Feature;
pub(crate) use codex_login::CodexAuth;
pub(crate) use codex_protocol::config_types::ServiceTier;
pub(crate) use codex_protocol::dynamic_tools::DynamicToolSpec;
pub(crate) use codex_protocol::items::TurnItem;
pub(crate) use codex_protocol::models::ContentItem;
pub(crate) use codex_protocol::models::ResponseItem;
pub(crate) use codex_protocol::protocol::ConversationStartParams;
pub(crate) use codex_protocol::protocol::ErrorEvent;
pub(crate) use codex_protocol::protocol::EventMsg;
pub(crate) use codex_protocol::protocol::ItemCompletedEvent;
pub(crate) use codex_protocol::protocol::ItemStartedEvent;
pub(crate) use codex_protocol::protocol::Op;
pub(crate) use codex_protocol::protocol::RealtimeConversationRealtimeEvent;
pub(crate) use codex_protocol::protocol::RealtimeEvent;
pub(crate) use codex_protocol::protocol::RealtimeOutputModality;
pub(crate) use codex_protocol::protocol::RolloutItem;
pub(crate) use codex_protocol::protocol::RolloutLine;
pub(crate) use codex_protocol::user_input::UserInput;
pub(crate) use pretty_assertions::assert_eq;
pub(crate) use serde_json::Value;
pub(crate) use serde_json::json;
pub(crate) use tokio::time::Duration;
pub(crate) use wiremock::ResponseTemplate;

pub(crate) fn approx_token_count(text: &str) -> i64 {
    i64::try_from(text.len().saturating_add(3) / 4).unwrap_or(i64::MAX)
}

pub(crate) fn estimate_compact_input_tokens(request: &responses::ResponsesRequest) -> i64 {
    request.input().into_iter().fold(0i64, |acc, item| {
        acc.saturating_add(approx_token_count(&item.to_string()))
    })
}

pub(crate) fn estimate_compact_payload_tokens(request: &responses::ResponsesRequest) -> i64 {
    estimate_compact_input_tokens(request)
        .saturating_add(approx_token_count(&request.instructions_text()))
}

pub(crate) fn assert_tools_payload_does_not_defer(body: &Value) {
    if let Some(tools) = body.get("tools") {
        assert!(
            !contains_defer_loading(tools),
            "model-visible tools should not include deferred declarations: {tools}"
        );
    }
}

pub(crate) fn namespace_child_tool_names(body: &Value, namespace: &str) -> Vec<String> {
    body.get("tools")
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter().find_map(|tool| {
                if tool.get("type").and_then(Value::as_str) == Some("namespace")
                    && tool.get("name").and_then(Value::as_str) == Some(namespace)
                {
                    tool.get("tools").and_then(Value::as_array).map(|children| {
                        children
                            .iter()
                            .filter_map(|child| {
                                child
                                    .get("name")
                                    .and_then(Value::as_str)
                                    .map(str::to_string)
                            })
                            .collect()
                    })
                } else {
                    None
                }
            })
        })
        .unwrap_or_default()
}

pub(crate) fn contains_defer_loading(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            map.get("defer_loading").and_then(Value::as_bool) == Some(true)
                || map.values().any(contains_defer_loading)
        }
        Value::Array(values) => values.iter().any(contains_defer_loading),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => false,
    }
}

pub(crate) fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key.clone(), canonical_json(value)))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical_json).collect()),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => value.clone(),
    }
}

pub(crate) const PRETURN_CONTEXT_DIFF_CWD: &str = "/tmp/PRETURN_CONTEXT_DIFF_CWD";
pub(crate) const DUMMY_FUNCTION_NAME: &str = "test_tool";
pub(crate) const REMOTE_COMPACT_TURN_COMPLETE_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) fn context_snapshot_options() -> ContextSnapshotOptions {
    ContextSnapshotOptions::default()
        .strip_capability_instructions()
        .render_mode(ContextSnapshotRenderMode::KindWithTextPrefix { max_chars: 64 })
}

pub(crate) fn format_labeled_requests_snapshot(
    scenario: &str,
    sections: &[(&str, &responses::ResponsesRequest)],
) -> String {
    context_snapshot::format_labeled_requests_snapshot(
        scenario,
        sections,
        &context_snapshot_options(),
    )
}

pub(crate) fn compacted_summary_only_output(summary: &str) -> Vec<ResponseItem> {
    vec![ResponseItem::Compaction {
        encrypted_content: summary_with_prefix(summary),
    }]
}

pub(crate) fn remote_realtime_test_codex_builder(
    realtime_server: &responses::WebSocketTestServer,
) -> TestCodexBuilder {
    let realtime_base_url = realtime_server.uri().to_string();
    test_codex()
        .with_auth(CodexAuth::from_api_key("dummy"))
        .with_config(move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
        })
}

pub(crate) async fn start_remote_realtime_server() -> responses::WebSocketTestServer {
    start_websocket_server(vec![vec![
        vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_remote_compact", "instructions": "backend prompt" }
        })],
        // Keep the websocket open after startup so routed transcript items during the test do not
        // exhaust the scripted responses and mark realtime inactive before the assertions run.
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    ]])
    .await
}

pub(crate) async fn start_realtime_conversation(codex: &codex_core::CodexThread) -> Result<()> {
    codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
        }))
        .await?;

    wait_for_event_match(codex, |msg| match msg {
        EventMsg::RealtimeConversationStarted(started) => Some(Ok(started.clone())),
        EventMsg::Error(err) => Some(Err(err.clone())),
        _ => None,
    })
    .await
    .unwrap_or_else(|err: ErrorEvent| panic!("conversation start failed: {err:?}"));

    wait_for_event_match(codex, |msg| match msg {
        EventMsg::RealtimeConversationRealtime(RealtimeConversationRealtimeEvent {
            payload:
                RealtimeEvent::SessionUpdated {
                    realtime_session_id: session_id,
                    ..
                },
        }) => Some(session_id.clone()),
        _ => None,
    })
    .await;

    Ok(())
}

pub(crate) async fn close_realtime_conversation(codex: &codex_core::CodexThread) -> Result<()> {
    codex.submit(Op::RealtimeConversationClose).await?;
    wait_for_event_match(codex, |msg| match msg {
        EventMsg::RealtimeConversationClosed(closed) => Some(closed.clone()),
        _ => None,
    })
    .await;
    Ok(())
}

pub(crate) fn assert_request_contains_realtime_start(request: &responses::ResponsesRequest) {
    let body = request.body_json().to_string();
    assert!(
        body.contains("<realtime_conversation>"),
        "expected request to restate realtime instructions"
    );
    assert!(
        !body.contains("Reason: inactive"),
        "expected request to use realtime start instructions"
    );
}

pub(crate) fn assert_request_contains_custom_realtime_start(
    request: &responses::ResponsesRequest,
    instructions: &str,
) {
    let body = request.body_json().to_string();
    assert!(
        body.contains("<realtime_conversation>"),
        "expected request to preserve the realtime wrapper"
    );
    assert!(
        body.contains(instructions),
        "expected request to use custom realtime start instructions"
    );
    assert!(
        !body.contains("Realtime conversation started."),
        "expected request to replace the default realtime start instructions"
    );
}

pub(crate) fn assert_request_contains_realtime_end(request: &responses::ResponsesRequest) {
    let body = request.body_json().to_string();
    assert!(
        body.contains("<realtime_conversation>"),
        "expected request to restate realtime instructions"
    );
    assert!(
        body.contains("Reason: inactive"),
        "expected request to use realtime end instructions"
    );
}

pub(crate) async fn wait_for_turn_complete(codex: &codex_core::CodexThread) {
    wait_for_event_with_timeout(
        codex,
        |ev| matches!(ev, EventMsg::TurnComplete(_)),
        REMOTE_COMPACT_TURN_COMPLETE_TIMEOUT,
    )
    .await;
}
