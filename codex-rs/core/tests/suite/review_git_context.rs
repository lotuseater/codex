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

async fn review_uses_overridden_cwd_for_base_branch_merge_base() {
    skip_if_no_network!();

    let sse_raw = r#"[{"type":"response.completed", "response": {"id": "__ID__"}}]"#;
    let (server, request_log) =
        start_responses_server_with_sse(sse_raw, /*expected_requests*/ 1).await;

    let initial_cwd = TempDir::new().unwrap();

    let repo_dir = TempDir::new().unwrap();
    let repo_path = repo_dir.path();

    fn run_git(repo_path: &std::path::Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .args(args)
            .output()
            .expect("spawn git");
        assert!(
            output.status.success(),
            "git {:?} failed: stdout={:?} stderr={:?}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    run_git(repo_path, &["init", "-b", "main"]);
    run_git(repo_path, &["config", "user.email", "test@example.com"]);
    run_git(repo_path, &["config", "user.name", "Test User"]);
    std::fs::write(repo_path.join("file.txt"), "hello\n").unwrap();
    run_git(repo_path, &["add", "."]);
    run_git(repo_path, &["commit", "-m", "initial"]);

    let head_sha = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("rev-parse HEAD");
    assert!(head_sha.status.success());
    let head_sha = String::from_utf8(head_sha.stdout)
        .expect("utf8 sha")
        .trim()
        .to_string();

    let codex_home = Arc::new(TempDir::new().unwrap());
    let initial_cwd_path = initial_cwd.path().to_path_buf();
    let codex = new_conversation_for_server(&server, codex_home.clone(), move |config| {
        config.cwd = initial_cwd_path.abs();
    })
    .await;

    codex
        .submit(Op::OverrideTurnContext {
            cwd: Some(repo_path.to_path_buf()),
            approval_policy: None,
            approvals_reviewer: None,
            sandbox_policy: None,
            permission_profile: None,
            windows_sandbox_level: None,
            model: None,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: None,
            collaboration_mode: None,
            personality: None,
        })
        .await
        .unwrap();

    codex
        .submit(Op::Review {
            review_request: ReviewRequest {
                target: ReviewTarget::BaseBranch {
                    branch: "main".to_string(),
                },
                user_facing_hint: None,
            },
        })
        .await
        .unwrap();

    let _entered = wait_for_event(&codex, |ev| matches!(ev, EventMsg::EnteredReviewMode(_))).await;
    let _complete = wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let requests = request_log.requests();
    assert_eq!(requests.len(), 1);
    for request in &requests {
        assert_eq!(request.path(), "/v1/responses");
    }
    let body = requests[0].body_json();
    let input = body["input"].as_array().expect("input array");

    let saw_merge_base_sha = input
        .iter()
        .filter_map(|msg| msg["content"][0]["text"].as_str())
        .any(|text| text.contains(&head_sha));
    assert!(
        saw_merge_base_sha,
        "expected review prompt to include merge-base sha {head_sha}"
    );

    let _codex_home_guard = codex_home;
    server.verify().await;
}

/// Start a mock Responses API server and mount the given SSE stream body.

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
