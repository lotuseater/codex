pub(crate) use anyhow::Context;
pub(crate) use anyhow::Result;
pub(crate) use chrono::Utc;
pub(crate) use codex_config::config_toml::RealtimeWsVersion;
pub(crate) use codex_core::test_support::auth_manager_from_auth;
pub(crate) use codex_core_test_runtime::responses;
pub(crate) use codex_core_test_runtime::responses::WebSocketConnectionConfig;
pub(crate) use codex_core_test_runtime::responses::start_mock_server;
pub(crate) use codex_core_test_runtime::responses::start_websocket_server;
pub(crate) use codex_core_test_runtime::responses::start_websocket_server_with_headers;
pub(crate) use codex_core_test_runtime::skip_if_no_network;
pub(crate) use codex_core_test_runtime::streaming_sse::StreamingSseChunk;
pub(crate) use codex_core_test_runtime::streaming_sse::start_streaming_sse_server;
pub(crate) use codex_core_test_runtime::test_codex::TestCodex;
pub(crate) use codex_core_test_runtime::test_codex::test_codex;
pub(crate) use codex_core_test_runtime::wait_for_event;
pub(crate) use codex_core_test_runtime::wait_for_event_match;
pub(crate) use codex_login::CodexAuth;
pub(crate) use codex_login::OPENAI_API_KEY_ENV_VAR;
pub(crate) use codex_protocol::ThreadId;
pub(crate) use codex_protocol::models::ContentItem;
pub(crate) use codex_protocol::models::ResponseItem;
pub(crate) use codex_protocol::protocol::CodexErrorInfo;
pub(crate) use codex_protocol::protocol::ConversationAudioParams;
pub(crate) use codex_protocol::protocol::ConversationStartParams;
pub(crate) use codex_protocol::protocol::ConversationStartTransport;
pub(crate) use codex_protocol::protocol::ConversationTextParams;
pub(crate) use codex_protocol::protocol::ErrorEvent;
pub(crate) use codex_protocol::protocol::EventMsg;
pub(crate) use codex_protocol::protocol::InitialHistory;
pub(crate) use codex_protocol::protocol::Op;
pub(crate) use codex_protocol::protocol::RealtimeAudioFrame;
pub(crate) use codex_protocol::protocol::RealtimeConversationRealtimeEvent;
pub(crate) use codex_protocol::protocol::RealtimeConversationVersion;
pub(crate) use codex_protocol::protocol::RealtimeEvent;
pub(crate) use codex_protocol::protocol::RealtimeNoopRequested;
pub(crate) use codex_protocol::protocol::RealtimeOutputModality;
pub(crate) use codex_protocol::protocol::RealtimeVoice;
pub(crate) use codex_protocol::protocol::RolloutItem;
pub(crate) use codex_protocol::protocol::SessionSource;
pub(crate) use codex_protocol::user_input::UserInput;
pub(crate) use codex_utils_output_truncation::approx_token_count;
pub(crate) use pretty_assertions::assert_eq;
pub(crate) use serde_json::Value;
pub(crate) use serde_json::json;
pub(crate) use std::fs;
pub(crate) use std::process::Command;
pub(crate) use std::sync::Arc;
pub(crate) use std::sync::Mutex;
pub(crate) use std::time::Duration;
pub(crate) use tokio::sync::oneshot;
pub(crate) use tokio::time::timeout;
pub(crate) use wiremock::Match;
pub(crate) use wiremock::Mock;
pub(crate) use wiremock::Request as WiremockRequest;
pub(crate) use wiremock::ResponseTemplate;
pub(crate) use wiremock::matchers::method;
pub(crate) use wiremock::matchers::path_regex;

pub(crate) const STARTUP_CONTEXT_HEADER: &str = "Startup context from Codex.";
pub(crate) const STARTUP_CONTEXT_OPEN_TAG: &str = "<startup_context>";
pub(crate) const STARTUP_CONTEXT_CLOSE_TAG: &str = "</startup_context>";
pub(crate) const REALTIME_BACKEND_PROMPT: &str =
    include_str!("../../templates/realtime/backend_prompt.md");
pub(crate) const USER_FIRST_NAME_PLACEHOLDER: &str = "{{ user_first_name }}";
pub(crate) const MEMORY_PROMPT_PHRASE: &str =
    "You have access to a memory folder with guidance from prior runs.";
pub(crate) const REALTIME_CONVERSATION_TEST_SUBPROCESS_ENV_VAR: &str =
    "CODEX_REALTIME_CONVERSATION_TEST_SUBPROCESS";

#[derive(Debug, Clone)]
pub(crate) struct RealtimeCallRequestCapture {
    pub(crate) requests: Arc<Mutex<Vec<WiremockRequest>>>,
}

impl RealtimeCallRequestCapture {
    pub(crate) fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn single_request(&self) -> WiremockRequest {
        let requests = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(requests.len(), 1, "expected one realtime call request");
        requests[0].clone()
    }
}

impl Match for RealtimeCallRequestCapture {
    pub(crate) fn matches(&self, request: &WiremockRequest) -> bool {
        self.requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(request.clone());
        true
    }
}

pub(crate) fn normalized_json_string(raw: &str) -> Result<String> {
    let value: Value = serde_json::from_str(raw).context("expected JSON fixture to parse")?;
    serde_json::to_string(&value).context("expected JSON fixture to serialize")
}

pub(crate) fn websocket_request_text(
    request: &codex_core_test_runtime::responses::WebSocketRequest,
) -> Option<String> {
    request.body_json()["item"]["content"][0]["text"]
        .as_str()
        .map(str::to_owned)
}

pub(crate) fn websocket_request_instructions(
    request: &codex_core_test_runtime::responses::WebSocketRequest,
) -> Option<String> {
    request.body_json()["session"]["instructions"]
        .as_str()
        .map(str::to_owned)
}

pub(crate) async fn wait_for_websocket_request(
    server: &codex_core_test_runtime::responses::WebSocketTestServer,
    connection_index: usize,
    request_index: usize,
) -> Result<codex_core_test_runtime::responses::WebSocketRequest> {
    timeout(
        Duration::from_secs(2),
        server.wait_for_request(connection_index, request_index),
    )
    .await
    .with_context(|| {
        format!("timed out waiting for websocket request {connection_index}/{request_index}")
    })
}

pub(crate) fn expected_realtime_backend_prompt() -> String {
    REALTIME_BACKEND_PROMPT
        .trim_end()
        .replace(USER_FIRST_NAME_PLACEHOLDER, &test_user_first_name())
}

pub(crate) fn test_user_first_name() -> String {
    [whoami::realname(), whoami::username()]
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter_map(|name| name.split_whitespace().next().map(str::to_string))
        .find(|name| !name.is_empty())
        .unwrap_or_else(|| "there".to_string())
}

pub(crate) async fn wait_for_matching_websocket_request<F>(
    server: &codex_core_test_runtime::responses::WebSocketTestServer,
    description: &str,
    predicate: F,
) -> codex_core_test_runtime::responses::WebSocketRequest
where
    F: Fn(&codex_core_test_runtime::responses::WebSocketRequest) -> bool,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(request) = server
            .connections()
            .iter()
            .flat_map(|connection| connection.iter())
            .find(|request| predicate(request))
            .cloned()
        {
            return request;
        }

        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

pub(crate) fn run_realtime_conversation_test_in_subprocess(
    test_name: &str,
    openai_api_key: Option<&str>,
) -> Result<()> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--exact")
        .arg(test_name)
        .env(REALTIME_CONVERSATION_TEST_SUBPROCESS_ENV_VAR, "1");
    // The child talks to a loopback websocket server; parent proxy settings can
    // route that connection away from the test server in Bazel environments.
    for &key in codex_network_proxy::PROXY_ENV_KEYS {
        command.env_remove(key);
    }
    match openai_api_key {
        Some(openai_api_key) => {
            command.env(OPENAI_API_KEY_ENV_VAR, openai_api_key);
        }
        None => {
            command.env_remove(OPENAI_API_KEY_ENV_VAR);
        }
    }
    let output = command.output()?;
    assert!(
        output.status.success(),
        "subprocess test `{test_name}` failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    Ok(())
}
pub(crate) async fn seed_recent_thread(
    test: &TestCodex,
    title: &str,
    first_user_message: &str,
    slug: &str,
) -> Result<()> {
    let db = test.codex.state_db().context("state db enabled")?;
    let thread_id = ThreadId::new();
    let updated_at = Utc::now();
    let rollout_path = test
        .codex_home_path()
        .join(format!("rollout-{thread_id}.jsonl"));
    // This helper seeds SQLite metadata directly. Local listing drops stale metadata rows whose
    // rollout path no longer exists, so create the placeholder path that the test metadata points
    // at without exercising rollout writing in this realtime-context test.
    std::fs::write(&rollout_path, "")?;
    let mut metadata_builder = codex_state::ThreadMetadataBuilder::new(
        thread_id,
        rollout_path,
        updated_at,
        SessionSource::Cli,
    );
    metadata_builder.cwd = test.workspace_path(format!("workspace-{slug}"));
    metadata_builder.model_provider = Some("test-provider".to_string());
    metadata_builder.git_branch = Some(format!("branch-{slug}"));
    let mut metadata = metadata_builder.build("test-provider");
    metadata.title = title.to_string();
    metadata.first_user_message = Some(first_user_message.to_string());
    db.upsert_thread(&metadata).await?;

    Ok(())
}

pub(crate) fn sse_event(event: Value) -> String {
    responses::sse(vec![event])
}

pub(crate) fn message_input_texts(body: &Value, role: &str) -> Vec<String> {
    body.get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        .filter(|item| item.get("role").and_then(Value::as_str) == Some(role))
        .filter_map(|item| item.get("content").and_then(Value::as_array))
        .flatten()
        .filter(|span| span.get("type").and_then(Value::as_str) == Some("input_text"))
        .filter_map(|span| span.get("text").and_then(Value::as_str).map(str::to_owned))
        .collect()
}
