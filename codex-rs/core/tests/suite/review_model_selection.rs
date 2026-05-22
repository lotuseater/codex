use codex_core::CodexThread;
use codex_core::REVIEW_PROMPT;
use codex_core::config::Config;
use codex_core::review_format::render_review_output_text;
use codex_core_test_runtime::PathBufExt;
use codex_core_test_runtime::load_sse_fixture_with_id_from_str;
use codex_core_test_runtime::responses::ResponseMock;
use codex_core_test_runtime::responses::mount_sse_sequence;
use codex_core_test_runtime::responses::start_mock_server;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::wait_for_event;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::ENVIRONMENT_CONTEXT_OPEN_TAG;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ExitedReviewModeEvent;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::ReviewCodeLocation;
use codex_protocol::protocol::ReviewFinding;
use codex_protocol::protocol::ReviewLineRange;
use codex_protocol::protocol::ReviewOutputEvent;
use codex_protocol::protocol::ReviewRequest;
use codex_protocol::protocol::ReviewTarget;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::RolloutLine;
use codex_protocol::user_input::UserInput;
use pretty_assertions::assert_eq;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::io::AsyncWriteExt as _;
use uuid::Uuid;
use wiremock::MockServer;

async fn review_uses_custom_review_model_from_config() {
    skip_if_no_network!();

    // Minimal stream: just a completed event
    let sse_raw = r#"[
        {"type":"response.completed", "response": {"id": "__ID__"}}
    ]"#;
    let (server, request_log) =
        start_responses_server_with_sse(sse_raw, /*expected_requests*/ 1).await;
    let codex_home = Arc::new(TempDir::new().unwrap());
    // Choose a review model different from the main model; ensure it is used.
    let codex = new_conversation_for_server(&server, codex_home.clone(), |cfg| {
        cfg.model = Some("gpt-4.1".to_string());
        cfg.review_model = Some("gpt-5.4".to_string());
    })
    .await;

    codex
        .submit(Op::Review {
            review_request: ReviewRequest {
                target: ReviewTarget::Custom {
                    instructions: "use custom model".to_string(),
                },
                user_facing_hint: None,
            },
        })
        .await
        .unwrap();

    // Wait for completion
    let _entered = wait_for_event(&codex, |ev| matches!(ev, EventMsg::EnteredReviewMode(_))).await;
    let _closed = wait_for_event(&codex, |ev| {
        matches!(
            ev,
            EventMsg::ExitedReviewMode(ExitedReviewModeEvent {
                review_output: None
            })
        )
    })
    .await;
    let _complete = wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    // Assert the request body model equals the configured review model
    let request = request_log.single_request();
    assert_eq!(request.path(), "/v1/responses");
    let body = request.body_json();
    assert_eq!(body["model"].as_str().unwrap(), "gpt-5.4");

    let _codex_home_guard = codex_home;
    server.verify().await;
}

/// Ensure that when `review_model` is not set in the config, the review request
/// uses the session model.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn review_uses_session_model_when_review_model_unset() {
    skip_if_no_network!();

    // Minimal stream: just a completed event
    let sse_raw = r#"[
        {"type":"response.completed", "response": {"id": "__ID__"}}
    ]"#;
    let (server, request_log) =
        start_responses_server_with_sse(sse_raw, /*expected_requests*/ 1).await;
    let codex_home = Arc::new(TempDir::new().unwrap());
    let codex = new_conversation_for_server(&server, codex_home.clone(), |cfg| {
        cfg.model = Some("gpt-4.1".to_string());
        cfg.review_model = None;
    })
    .await;

    codex
        .submit(Op::Review {
            review_request: ReviewRequest {
                target: ReviewTarget::Custom {
                    instructions: "use session model".to_string(),
                },
                user_facing_hint: None,
            },
        })
        .await
        .unwrap();

    let _entered = wait_for_event(&codex, |ev| matches!(ev, EventMsg::EnteredReviewMode(_))).await;
    let _closed = wait_for_event(&codex, |ev| {
        matches!(
            ev,
            EventMsg::ExitedReviewMode(ExitedReviewModeEvent {
                review_output: None
            })
        )
    })
    .await;
    let _complete = wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let request = request_log.single_request();
    assert_eq!(request.path(), "/v1/responses");
    let body = request.body_json();
    assert_eq!(body["model"].as_str().unwrap(), "gpt-4.1");

    let _codex_home_guard = codex_home;
    server.verify().await;
}

/// When a review session begins, it must not prepend prior chat history from
/// the parent session. The request `input` should contain only the review
/// prompt from the user.
// Windows CI only: bump to 4 workers to prevent SSE/event starvation and test timeouts.
#[cfg_attr(windows, tokio::test(flavor = "multi_thread", worker_threads = 4))]
#[cfg_attr(not(windows), tokio::test(flavor = "multi_thread", worker_threads = 2))]

async fn start_responses_server_with_sse(
    sse_raw: &str,
    expected_requests: usize,
) -> (MockServer, ResponseMock) {
    let server = start_mock_server().await;
    let sse = load_sse_fixture_with_id_from_str(sse_raw, &Uuid::new_v4().to_string());
    let responses = vec![sse; expected_requests];
    let request_log = mount_sse_sequence(&server, responses).await;
    (server, request_log)
}

/// Create a conversation configured to talk to the provided mock server.
#[expect(clippy::expect_used)]
async fn new_conversation_for_server<F>(
    server: &MockServer,
    codex_home: Arc<TempDir>,
    mutator: F,
) -> Arc<CodexThread>
where
    F: FnOnce(&mut Config) + Send + 'static,
{
    let base_url = format!("{}/v1", server.uri());
    let mut builder = test_codex()
        .with_home(codex_home)
        .with_config(move |config| {
            config.model_provider.base_url = Some(base_url.clone());
            mutator(config);
        });
    builder
        .build(server)
        .await
        .expect("create conversation")
        .codex
}

/// Create a conversation resuming from a rollout file, configured to talk to the provided mock server.
#[expect(clippy::expect_used)]
async fn resume_conversation_for_server<F>(
    server: &MockServer,
    codex_home: Arc<TempDir>,
    resume_path: std::path::PathBuf,
    mutator: F,
) -> Arc<CodexThread>
where
    F: FnOnce(&mut Config) + Send + 'static,
{
    let base_url = format!("{}/v1", server.uri());
    let mut builder = test_codex()
        .with_home(codex_home.clone())
        .with_config(move |config| {
            config.model_provider.base_url = Some(base_url.clone());
            mutator(config);
        });
    builder
        .resume(server, codex_home, resume_path)
        .await
        .expect("resume conversation")
        .codex
}
