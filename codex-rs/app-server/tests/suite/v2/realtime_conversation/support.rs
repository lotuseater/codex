pub(crate) use anyhow::Context;
pub(crate) use anyhow::Result;
pub(crate) use app_test_support::McpProcess;
pub(crate) use app_test_support::create_final_assistant_message_sse_response;
pub(crate) use app_test_support::create_mock_responses_server_sequence_unchecked;
pub(crate) use app_test_support::create_shell_command_sse_response;
pub(crate) use app_test_support::to_response;
pub(crate) use codex_app_server_protocol::CommandExecutionStatus;
pub(crate) use codex_app_server_protocol::ItemCompletedNotification;
pub(crate) use codex_app_server_protocol::ItemStartedNotification;
pub(crate) use codex_app_server_protocol::JSONRPCError;
pub(crate) use codex_app_server_protocol::JSONRPCResponse;
pub(crate) use codex_app_server_protocol::LoginAccountResponse;
pub(crate) use codex_app_server_protocol::RequestId;
pub(crate) use codex_app_server_protocol::ThreadItem;
pub(crate) use codex_app_server_protocol::ThreadRealtimeAppendAudioParams;
pub(crate) use codex_app_server_protocol::ThreadRealtimeAppendAudioResponse;
pub(crate) use codex_app_server_protocol::ThreadRealtimeAppendTextParams;
pub(crate) use codex_app_server_protocol::ThreadRealtimeAppendTextResponse;
pub(crate) use codex_app_server_protocol::ThreadRealtimeAudioChunk;
pub(crate) use codex_app_server_protocol::ThreadRealtimeClosedNotification;
pub(crate) use codex_app_server_protocol::ThreadRealtimeErrorNotification;
pub(crate) use codex_app_server_protocol::ThreadRealtimeItemAddedNotification;
pub(crate) use codex_app_server_protocol::ThreadRealtimeListVoicesParams;
pub(crate) use codex_app_server_protocol::ThreadRealtimeListVoicesResponse;
pub(crate) use codex_app_server_protocol::ThreadRealtimeOutputAudioDeltaNotification;
pub(crate) use codex_app_server_protocol::ThreadRealtimeSdpNotification;
pub(crate) use codex_app_server_protocol::ThreadRealtimeStartParams;
pub(crate) use codex_app_server_protocol::ThreadRealtimeStartResponse;
pub(crate) use codex_app_server_protocol::ThreadRealtimeStartTransport;
pub(crate) use codex_app_server_protocol::ThreadRealtimeStartedNotification;
pub(crate) use codex_app_server_protocol::ThreadRealtimeStopParams;
pub(crate) use codex_app_server_protocol::ThreadRealtimeStopResponse;
pub(crate) use codex_app_server_protocol::ThreadRealtimeTranscriptDeltaNotification;
pub(crate) use codex_app_server_protocol::ThreadRealtimeTranscriptDoneNotification;
pub(crate) use codex_app_server_protocol::ThreadStartParams;
pub(crate) use codex_app_server_protocol::ThreadStartResponse;
pub(crate) use codex_app_server_protocol::TurnCompletedNotification;
pub(crate) use codex_app_server_protocol::TurnStartedNotification;
pub(crate) use codex_features::FEATURES;
pub(crate) use codex_features::Feature;
pub(crate) use codex_protocol::protocol::RealtimeConversationVersion;
pub(crate) use codex_protocol::protocol::RealtimeOutputModality;
pub(crate) use codex_protocol::protocol::RealtimeVoice;
pub(crate) use codex_protocol::protocol::RealtimeVoicesList;
pub(crate) use app_test_support::responses;
pub(crate) use app_test_support::responses::WebSocketConnectionConfig;
pub(crate) use app_test_support::responses::WebSocketRequest;
pub(crate) use app_test_support::responses::WebSocketTestServer;
pub(crate) use app_test_support::responses::start_websocket_server;
pub(crate) use app_test_support::responses::start_websocket_server_with_headers;
pub(crate) use app_test_support::skip_if_no_network;
pub(crate) use pretty_assertions::assert_eq;
pub(crate) use serde::de::DeserializeOwned;
pub(crate) use serde_json::Value;
pub(crate) use serde_json::json;
pub(crate) use std::path::Path;
pub(crate) use std::sync::Arc;
pub(crate) use std::sync::Mutex;
pub(crate) use std::sync::mpsc;
pub(crate) use std::time::Duration;
pub(crate) use tempfile::TempDir;
pub(crate) use tokio::time::timeout;
pub(crate) use wiremock::Match;
pub(crate) use wiremock::Mock;
pub(crate) use wiremock::MockServer;
pub(crate) use wiremock::Request as WiremockRequest;
pub(crate) use wiremock::Respond;
pub(crate) use wiremock::ResponseTemplate;
pub(crate) use wiremock::matchers::method;
pub(crate) use wiremock::matchers::path;
pub(crate) use wiremock::matchers::path_regex;

pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const DELEGATED_SHELL_TOOL_TIMEOUT_MS: u64 = 30_000;
pub(crate) const STARTUP_CONTEXT_HEADER: &str = "Startup context from Codex.";
pub(crate) const V2_STEERING_ACKNOWLEDGEMENT: &str =
    "This was sent to steer the previous background agent task.";
pub(crate) const V2_HANDOFF_COMPLETE_ACKNOWLEDGEMENT: &str =
    "Background agent finished. Use the preceding [BACKEND] messages as the result.";

#[derive(Debug, Clone, Copy)]
pub(crate) enum StartupContextConfig<'a> {
    Generated,
    Override(&'a str),
}

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

pub(crate) struct GatedSseResponse {
    pub(crate) gate_rx: Mutex<Option<mpsc::Receiver<()>>>,
    pub(crate) response: String,
}

impl Respond for GatedSseResponse {
    pub(crate) fn respond(&self, _: &WiremockRequest) -> ResponseTemplate {
        let gate_rx = self
            .gate_rx
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(gate_rx) = gate_rx {
            let _ = gate_rx.recv();
        }
        responses::sse_response(self.response.clone())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RealtimeTestVersion {
    V1,
    V2,
}

impl RealtimeTestVersion {
    pub(crate) fn config_value(self) -> &'static str {
        match self {
            RealtimeTestVersion::V1 => "v1",
            RealtimeTestVersion::V2 => "v2",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RealtimeTestSandbox {
    ReadOnly,
    DangerFullAccess,
}

impl RealtimeTestSandbox {
    pub(crate) fn config_value(self) -> &'static str {
        match self {
            RealtimeTestSandbox::ReadOnly => "read-only",
            RealtimeTestSandbox::DangerFullAccess => "danger-full-access",
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct StartedWebrtcRealtime {
    pub(crate) started: ThreadRealtimeStartedNotification,
    pub(crate) sdp: ThreadRealtimeSdpNotification,
}

// Scripted SSE responses for the normal background agent loop. Realtime can ask for a delegated
// background agent turn; that turn talks to this mock `/responses` endpoint and may request
// ordinary tools.
pub(crate) struct MainLoopResponsesScript {
    pub(crate) responses: Vec<String>,
}

// Scripted server events for the direct realtime sideband WebSocket. This mock is the realtime
// session app-server joins after call creation; it is not the background agent Responses stream.
pub(crate) struct RealtimeSidebandScript {
    pub(crate) connections: Vec<WebSocketConnectionConfig>,
}

pub(crate) struct RealtimeE2eHarness {
    pub(crate) mcp: McpProcess,
    pub(crate) _codex_home: TempDir,
    pub(crate) main_loop_responses_server: MockServer,
    pub(crate) realtime_server: WebSocketTestServer,
    pub(crate) call_capture: RealtimeCallRequestCapture,
    pub(crate) thread_id: String,
}

impl RealtimeE2eHarness {
    // Owns the full mocked app-server realtime route: MCP client, Responses mocks, WebRTC call
    // creation capture, sideband WebSocket server, login, config, and a started thread.
    pub(crate) async fn new(
        realtime_version: RealtimeTestVersion,
        main_loop: MainLoopResponsesScript,
        realtime_sideband: RealtimeSidebandScript,
    ) -> Result<Self> {
        let main_loop_responses_server =
            create_mock_responses_server_sequence_unchecked(main_loop.responses).await;
        Self::new_with_main_loop_responses_server_and_sandbox(
            realtime_version,
            main_loop_responses_server,
            realtime_sideband,
            RealtimeTestSandbox::ReadOnly,
        )
        .await
    }

    pub(crate) async fn new_with_sandbox(
        realtime_version: RealtimeTestVersion,
        main_loop: MainLoopResponsesScript,
        realtime_sideband: RealtimeSidebandScript,
        sandbox: RealtimeTestSandbox,
    ) -> Result<Self> {
        let main_loop_responses_server =
            create_mock_responses_server_sequence_unchecked(main_loop.responses).await;
        Self::new_with_main_loop_responses_server_and_sandbox(
            realtime_version,
            main_loop_responses_server,
            realtime_sideband,
            sandbox,
        )
        .await
    }

    pub(crate) async fn new_with_main_loop_responses_server(
        realtime_version: RealtimeTestVersion,
        main_loop_responses_server: MockServer,
        realtime_sideband: RealtimeSidebandScript,
    ) -> Result<Self> {
        Self::new_with_main_loop_responses_server_and_sandbox(
            realtime_version,
            main_loop_responses_server,
            realtime_sideband,
            RealtimeTestSandbox::ReadOnly,
        )
        .await
    }

    pub(crate) async fn new_with_main_loop_responses_server_and_sandbox(
        realtime_version: RealtimeTestVersion,
        main_loop_responses_server: MockServer,
        realtime_sideband: RealtimeSidebandScript,
        sandbox: RealtimeTestSandbox,
    ) -> Result<Self> {
        let call_capture = RealtimeCallRequestCapture::new();
        Mock::given(method("POST"))
            .and(path("/v1/realtime/calls"))
            .and(call_capture.clone())
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Location", "/v1/realtime/calls/rtc_e2e")
                    .set_body_string("v=answer\r\n"),
            )
            .mount(&main_loop_responses_server)
            .await;

        let realtime_server =
            start_websocket_server_with_headers(realtime_sideband.connections).await;
        let codex_home = TempDir::new()?;
        create_config_toml_with_realtime_version(
            codex_home.path(),
            &main_loop_responses_server.uri(),
            realtime_server.uri(),
            /*realtime_enabled*/ true,
            StartupContextConfig::Override("startup context"),
            realtime_version,
            sandbox,
        )?;

        let mut mcp = McpProcess::new(codex_home.path()).await?;
        timeout(DEFAULT_TIMEOUT, mcp.initialize()).await??;
        login_with_api_key(&mut mcp, "sk-test-key").await?;

        let thread_start_request_id = mcp
            .send_thread_start_request(ThreadStartParams::default())
            .await?;
        let thread_start_response: JSONRPCResponse = timeout(
            DEFAULT_TIMEOUT,
            mcp.read_stream_until_response_message(RequestId::Integer(thread_start_request_id)),
        )
        .await??;
        let thread_start: ThreadStartResponse = to_response(thread_start_response)?;

        Ok(Self {
            mcp,
            _codex_home: codex_home,
            main_loop_responses_server,
            realtime_server,
            call_capture,
            thread_id: thread_start.thread.id,
        })
    }

    pub(crate) async fn start_webrtc_realtime(&mut self, offer_sdp: &str) -> Result<StartedWebrtcRealtime> {
        // Starts realtime through the public JSON-RPC method, then waits for the same client-visible
        // notifications a desktop app needs: started first, SDP answer second.
        let start_request_id = self
            .mcp
            .send_thread_realtime_start_request(ThreadRealtimeStartParams {
                thread_id: self.thread_id.clone(),
                output_modality: RealtimeOutputModality::Audio,
                prompt: Some(Some("backend prompt".to_string())),
                realtime_session_id: None,
                transport: Some(ThreadRealtimeStartTransport::Webrtc {
                    sdp: offer_sdp.to_string(),
                }),
                voice: None,
            })
            .await?;
        let start_response: JSONRPCResponse = timeout(
            DEFAULT_TIMEOUT,
            self.mcp
                .read_stream_until_response_message(RequestId::Integer(start_request_id)),
        )
        .await??;
        let _: ThreadRealtimeStartResponse = to_response(start_response)?;

        let started = self
            .read_notification::<ThreadRealtimeStartedNotification>("thread/realtime/started")
            .await?;
        let sdp = self
            .read_notification::<ThreadRealtimeSdpNotification>("thread/realtime/sdp")
            .await?;

        Ok(StartedWebrtcRealtime { started, sdp })
    }

    pub(crate) async fn read_notification<T: DeserializeOwned>(&mut self, method: &str) -> Result<T> {
        read_notification(&mut self.mcp, method).await
    }

    /// Returns the nth JSON message app-server wrote to the fake Realtime API
    /// sideband websocket.
    pub(crate) async fn sideband_outbound_request(&self, request_index: usize) -> Value {
        timeout(
            DEFAULT_TIMEOUT,
            self.realtime_server
                .wait_for_request(/*connection_index*/ 0, request_index),
        )
        .await
        .unwrap_or_else(|_| {
            panic!("timed out waiting for realtime sideband request {request_index}")
        })
        .body_json()
    }

    pub(crate) async fn append_audio(&mut self, thread_id: String) -> Result<()> {
        let request_id = self
            .mcp
            .send_thread_realtime_append_audio_request(ThreadRealtimeAppendAudioParams {
                thread_id,
                audio: ThreadRealtimeAudioChunk {
                    data: "BQYH".to_string(),
                    sample_rate: 24_000,
                    num_channels: 1,
                    samples_per_channel: Some(480),
                    item_id: None,
                },
            })
            .await?;
        let response: JSONRPCResponse = timeout(
            DEFAULT_TIMEOUT,
            self.mcp
                .read_stream_until_response_message(RequestId::Integer(request_id)),
        )
        .await??;
        let _: ThreadRealtimeAppendAudioResponse = to_response(response)?;
        Ok(())
    }

    pub(crate) async fn append_text(&mut self, thread_id: String, text: &str) -> Result<()> {
        let request_id = self
            .mcp
            .send_thread_realtime_append_text_request(ThreadRealtimeAppendTextParams {
                thread_id,
                text: text.to_string(),
            })
            .await?;
        let response: JSONRPCResponse = timeout(
            DEFAULT_TIMEOUT,
            self.mcp
                .read_stream_until_response_message(RequestId::Integer(request_id)),
        )
        .await??;
        let _: ThreadRealtimeAppendTextResponse = to_response(response)?;
        Ok(())
    }

    pub(crate) async fn main_loop_responses_requests(&self) -> Result<Vec<Value>> {
        responses_requests(&self.main_loop_responses_server).await
    }

    pub(crate) async fn shutdown(self) {
        self.realtime_server.shutdown().await;
    }
}

pub(crate) fn main_loop_responses(responses: Vec<String>) -> MainLoopResponsesScript {
    MainLoopResponsesScript { responses }
}

pub(crate) fn no_main_loop_responses() -> MainLoopResponsesScript {
    main_loop_responses(Vec::new())
}

pub(crate) fn realtime_sideband(connections: Vec<WebSocketConnectionConfig>) -> RealtimeSidebandScript {
    RealtimeSidebandScript { connections }
}

pub(crate) fn realtime_sideband_connection(
    realtime_server_events: Vec<Vec<Value>>,
) -> WebSocketConnectionConfig {
    WebSocketConnectionConfig {
        requests: realtime_server_events,
        response_headers: Vec::new(),
        accept_delay: None,
        close_after_requests: true,
    }
}

pub(crate) fn open_realtime_sideband_connection(
    realtime_server_events: Vec<Vec<Value>>,
) -> WebSocketConnectionConfig {
    WebSocketConnectionConfig {
        close_after_requests: false,
        ..realtime_sideband_connection(realtime_server_events)
    }
}

pub(crate) fn session_updated(realtime_session_id: &str) -> Value {
    json!({
        "type": "session.updated",
        "session": { "id": realtime_session_id, "instructions": "backend prompt" }
    })
}

pub(crate) fn v2_background_agent_tool_call(call_id: &str, prompt: &str) -> Value {
    json!({
        "type": "conversation.item.done",
        "item": {
            "id": format!("item_{call_id}"),
            "type": "function_call",
            "name": "background_agent",
            "call_id": call_id,
            "arguments": json!({ "prompt": prompt }).to_string()
        }
    })
}









/// Regression coverage for Realtime V2 text input while a response is active.
///
/// Text input is append-only, so app-server should send the user message without
/// requesting a new realtime response.

/// Regression coverage for append-only Realtime V2 text input when the active
/// response is cancelled instead of completed.

/// Regression coverage for the Realtime V2 background-agent final-output path.
///
/// Once the background agent finishes, app-server sends the final function-call
/// output to realtime and then requests a new `response.create` so realtime can
/// react to that final output.

/// Regression coverage for Realtime V2 steering while a background-agent task is
/// already active.
///
/// The second background-agent tool call is treated as guidance for the active
/// task. App-server acknowledges that steering message to realtime and then
/// emits `response.create` so realtime can speak that acknowledgement.






pub(crate) async fn read_notification<T: DeserializeOwned>(mcp: &mut McpProcess, method: &str) -> Result<T> {
    let notification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_notification_message(method),
    )
    .await??;
    let params = notification
        .params
        .context("expected notification params to be present")?;
    Ok(serde_json::from_value(params)?)
}

pub(crate) async fn login_with_api_key(mcp: &mut McpProcess, api_key: &str) -> Result<()> {
    let request_id = mcp.send_login_account_api_key_request(api_key).await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let login: LoginAccountResponse = to_response(response)?;
    assert_eq!(login, LoginAccountResponse::ApiKey {});

    Ok(())
}

pub(crate) async fn wait_for_started_command_execution(
    mcp: &mut McpProcess,
) -> Result<ItemStartedNotification> {
    loop {
        let started = read_notification::<ItemStartedNotification>(mcp, "item/started").await?;
        if let ThreadItem::CommandExecution { .. } = &started.item {
            return Ok(started);
        }
    }
}

pub(crate) async fn wait_for_completed_command_execution(
    mcp: &mut McpProcess,
) -> Result<ItemCompletedNotification> {
    loop {
        let completed =
            read_notification::<ItemCompletedNotification>(mcp, "item/completed").await?;
        if let ThreadItem::CommandExecution { .. } = &completed.item {
            return Ok(completed);
        }
    }
}

pub(crate) async fn responses_requests(server: &MockServer) -> Result<Vec<Value>> {
    server
        .received_requests()
        .await
        .context("failed to fetch received requests")?
        .into_iter()
        .filter(|request| request.url.path().ends_with("/responses"))
        .map(|request| {
            request
                .body_json::<Value>()
                .context("Responses request body should be JSON")
        })
        .collect()
}

pub(crate) fn response_request_contains_text(request: &Value, text: &str) -> bool {
    match request {
        Value::String(value) => value.contains(text),
        Value::Array(values) => values
            .iter()
            .any(|value| response_request_contains_text(value, text)),
        Value::Object(map) => map
            .values()
            .any(|value| response_request_contains_text(value, text)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

pub(crate) fn realtime_tool_ok_command() -> Vec<String> {
    #[cfg(windows)]
    {
        vec![
            "powershell.exe".to_string(),
            "-NoProfile".to_string(),
            "-Command".to_string(),
            "[Console]::Write('realtime-tool-ok')".to_string(),
        ]
    }

    #[cfg(not(windows))]
    {
        vec!["printf".to_string(), "realtime-tool-ok".to_string()]
    }
}

pub(crate) fn function_call_output_sideband_requests(server: &WebSocketTestServer) -> Vec<Value> {
    server
        .single_connection()
        .iter()
        .map(WebSocketRequest::body_json)
        .filter(|request| {
            request["type"] == "conversation.item.create"
                && request["item"]["type"] == "function_call_output"
        })
        .collect()
}

pub(crate) fn assert_v2_function_call_output(request: &Value, call_id: &str, expected_output: &str) {
    assert_eq!(
        request,
        &json!({
            "type": "conversation.item.create",
            "item": {
                "type": "function_call_output",
                "call_id": call_id,
                "output": expected_output,
            }
        })
    );
}

pub(crate) fn assert_v2_progress_update(request: &Value, expected_text: &str) {
    assert_eq!(
        request,
        &json!({
            "type": "conversation.item.create",
            "item": {
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": format!("[BACKEND] {expected_text}")
                }]
            }
        })
    );
}

pub(crate) fn assert_v2_user_text_item(request: &Value, expected_text: &str) {
    assert_eq!(
        request,
        &json!({
            "type": "conversation.item.create",
            "item": {
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": format!("[USER] {expected_text}")
                }]
            }
        })
    );
}

pub(crate) fn assert_v2_response_create(request: &Value) {
    assert_eq!(
        request,
        &json!({
            "type": "response.create"
        })
    );
}

pub(crate) fn assert_v1_session_update(request: &Value) -> Result<()> {
    assert_eq!(request["type"].as_str(), Some("session.update"));
    assert_eq!(request["session"]["type"].as_str(), Some("quicksilver"));
    assert!(
        request["session"]["instructions"]
            .as_str()
            .context("v1 session.update instructions")?
            .contains("startup context")
    );
    assert_eq!(
        request["session"]["audio"]["output"]["voice"].as_str(),
        Some("cove")
    );
    assert_eq!(request["session"]["tools"], Value::Null);
    Ok(())
}

pub(crate) fn assert_v2_session_update(request: &Value) -> Result<()> {
    assert_eq!(request["type"].as_str(), Some("session.update"));
    assert_eq!(request["session"]["type"].as_str(), Some("realtime"));
    assert!(
        request["session"]["instructions"]
            .as_str()
            .context("v2 session.update instructions")?
            .contains("startup context")
    );
    assert_eq!(
        request["session"]["tools"][0]["name"].as_str(),
        Some("background_agent")
    );
    assert_eq!(
        request["session"]["tools"][1]["name"].as_str(),
        Some("remain_silent")
    );
    assert_eq!(
        request["session"]["audio"]["input"]["transcription"]["model"].as_str(),
        Some("gpt-4o-mini-transcribe")
    );
    Ok(())
}

pub(crate) fn assert_call_create_multipart(
    request: WiremockRequest,
    offer_sdp: &str,
    session: &str,
) -> Result<()> {
    assert_eq!(request.url.path(), "/v1/realtime/calls");
    assert_eq!(request.url.query(), None);
    assert_eq!(
        request
            .headers
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("multipart/form-data; boundary=codex-realtime-call-boundary")
    );
    let body = String::from_utf8(request.body).context("multipart body should be utf-8")?;
    let session = normalized_json_string(session)?;
    assert_eq!(
        body,
        format!(
            "--codex-realtime-call-boundary\r\n\
             Content-Disposition: form-data; name=\"sdp\"\r\n\
             Content-Type: application/sdp\r\n\
             \r\n\
             {offer_sdp}\r\n\
             --codex-realtime-call-boundary\r\n\
             Content-Disposition: form-data; name=\"session\"\r\n\
             Content-Type: application/json\r\n\
             \r\n\
             {session}\r\n\
             --codex-realtime-call-boundary--\r\n"
        )
    );
    Ok(())
}

pub(crate) fn v1_session_create_json() -> &'static str {
    r#"{"audio":{"input":{"format":{"type":"audio/pcm","rate":24000}},"output":{"voice":"cove"}},"type":"quicksilver","model":"gpt-realtime-1.5","instructions":"backend prompt\n\nstartup context"}"#
}

pub(crate) fn create_config_toml(
    codex_home: &Path,
    responses_server_uri: &str,
    realtime_server_uri: &str,
    realtime_enabled: bool,
    startup_context: StartupContextConfig<'_>,
) -> std::io::Result<()> {
    create_config_toml_with_realtime_version(
        codex_home,
        responses_server_uri,
        realtime_server_uri,
        realtime_enabled,
        startup_context,
        RealtimeTestVersion::V2,
        RealtimeTestSandbox::ReadOnly,
    )
}

pub(crate) fn create_config_toml_with_realtime_version(
    codex_home: &Path,
    responses_server_uri: &str,
    realtime_server_uri: &str,
    realtime_enabled: bool,
    startup_context: StartupContextConfig<'_>,
    realtime_version: RealtimeTestVersion,
    sandbox: RealtimeTestSandbox,
) -> std::io::Result<()> {
    let realtime_feature_key = FEATURES
        .iter()
        .find(|spec| spec.id == Feature::RealtimeConversation)
        .map(|spec| spec.key)
        .unwrap_or("realtime_conversation");
    let realtime_version = realtime_version.config_value();
    let sandbox = sandbox.config_value();
    let startup_context = match startup_context {
        StartupContextConfig::Generated => String::new(),
        StartupContextConfig::Override(context) => {
            format!("experimental_realtime_ws_startup_context = {context:?}\n")
        }
    };

    std::fs::write(
        codex_home.join("config.toml"),
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "{sandbox}"
model_provider = "mock_provider"
experimental_realtime_ws_base_url = "{realtime_server_uri}"
experimental_realtime_ws_backend_prompt = "backend prompt"
{startup_context}

[realtime]
version = "{realtime_version}"
pub(crate) type = "conversational"

[features]
{realtime_feature_key} = {realtime_enabled}

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{responses_server_uri}/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#
        ),
    )
}

pub(crate) fn assert_invalid_request(error: JSONRPCError, message: String) {
    assert_eq!(error.error.code, -32600);
    assert_eq!(error.error.message, message);
    assert_eq!(error.error.data, None);
}
