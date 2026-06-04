use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::to_response;
use chrono::Utc;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::MemoryResetResponse;
use codex_app_server_protocol::MemoryStatusResponse;
use codex_app_server_protocol::RequestId;
use codex_protocol::ThreadId;
use codex_protocol::protocol::SessionSource;
use codex_state::Stage1JobClaimOutcome;
use codex_state::StateRuntime;
use codex_state::ThreadMetadataBuilder;
use pretty_assertions::assert_eq;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::timeout;
use uuid::Uuid;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[tokio::test]
async fn memory_reset_clears_memory_files_and_rows_preserves_threads() -> Result<()> {
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path())?;
    let state_db = init_state_db(codex_home.path()).await?;

    let memory_root = codex_home.path().join("memories");
    tokio::fs::create_dir_all(memory_root.join("rollout_summaries")).await?;
    tokio::fs::write(memory_root.join("MEMORY.md"), "stale memory\n").await?;
    tokio::fs::write(
        memory_root.join("rollout_summaries").join("stale.md"),
        "stale rollout summary\n",
    )
    .await?;

    let thread_id = seed_stage1_output(&state_db, codex_home.path()).await?;

    let mut mcp = TestAppServer::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_raw_request("memory/reset", /*params*/ None)
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let _: MemoryResetResponse = to_response::<MemoryResetResponse>(response)?;

    let stage1_outputs = state_db
        .memories()
        .list_stage1_outputs_for_global(/*n*/ 10)
        .await?;
    assert_eq!(stage1_outputs, Vec::new());
    assert_eq!(
        state_db.get_thread_memory_mode(thread_id).await?.as_deref(),
        Some("enabled")
    );

    let mut remaining_entries = tokio::fs::read_dir(&memory_root).await?;
    assert!(
        remaining_entries.next_entry().await?.is_none(),
        "memory root should be empty after reset"
    );

    Ok(())
}

#[tokio::test]
async fn memory_status_reports_memory_files_and_state_counts() -> Result<()> {
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path())?;
    let state_db = init_state_db(codex_home.path()).await?;

    let memory_root = codex_home.path().join("memories");
    tokio::fs::create_dir_all(&memory_root).await?;
    tokio::fs::write(memory_root.join("MEMORY.md"), "global memory\n").await?;
    tokio::fs::write(memory_root.join("raw_memories.md"), "raw memory\n").await?;

    let _thread_id = seed_stage1_output(&state_db, codex_home.path()).await?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let request_id = mcp
        .send_raw_request("memory/status", /*params*/ None)
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let response: MemoryStatusResponse = to_response::<MemoryStatusResponse>(response)?;

    assert!(response.feature_enabled);
    assert!(response.state_db_available);
    assert_eq!(response.memory_root, memory_root.display().to_string());
    assert!(response.memory_root_exists);
    assert!(response.memory_index_exists);
    assert!(response.raw_memories_exists);
    assert_eq!(response.stage1_output_count, Some(1));
    assert_eq!(response.selected_for_phase2_count, Some(0));
    assert!(response.latest_source_updated_at.is_some());
    assert!(response.latest_generated_at.is_some());
    assert_eq!(
        response
            .jobs
            .expect("job status counts")
            .into_iter()
            .map(|job| (job.kind, job.status, job.count))
            .collect::<Vec<_>>(),
        vec![
            (
                "memory_consolidate_global".to_string(),
                "pending".to_string(),
                1
            ),
            ("memory_stage1".to_string(), "succeeded".to_string(), 1),
        ]
    );

    Ok(())
}

async fn seed_stage1_output(state_db: &Arc<StateRuntime>, codex_home: &Path) -> Result<ThreadId> {
    let now = Utc::now();
    let thread_id = ThreadId::from_string(&Uuid::new_v4().to_string())?;
    let worker_id = ThreadId::from_string(&Uuid::new_v4().to_string())?;
    let mut builder = ThreadMetadataBuilder::new(
        thread_id,
        codex_home.join("sessions").join("test.jsonl"),
        now,
        SessionSource::Cli,
    );
    builder.updated_at = Some(now);
    builder.cwd = codex_home.to_path_buf();
    let metadata = builder.build("mock_provider");
    state_db.upsert_thread(&metadata).await?;

    let claim = state_db
        .memories()
        .try_claim_stage1_job(
            thread_id,
            worker_id,
            now.timestamp(),
            /*lease_seconds*/ 3600,
            /*max_running_jobs*/ 64,
        )
        .await?;
    let Stage1JobClaimOutcome::Claimed { ownership_token } = claim else {
        anyhow::bail!("unexpected stage1 claim outcome: {claim:?}");
    };
    assert!(
        state_db
            .memories()
            .mark_stage1_job_succeeded(
                thread_id,
                ownership_token.as_str(),
                now.timestamp(),
                "raw memory",
                "rollout summary",
                /*rollout_slug*/ None,
            )
            .await?,
        "stage1 success should be recorded"
    );
    state_db
        .memories()
        .enqueue_global_consolidation(now.timestamp())
        .await?;

    Ok(thread_id)
}

async fn init_state_db(codex_home: &Path) -> Result<Arc<StateRuntime>> {
    let state_db = StateRuntime::init(codex_home.to_path_buf(), "mock_provider".into()).await?;
    state_db
        .mark_backfill_complete(/*last_watermark*/ None)
        .await?;
    Ok(state_db)
}

fn create_config_toml(codex_home: &Path) -> std::io::Result<()> {
    let config_toml = codex_home.join("config.toml");
    std::fs::write(
        config_toml,
        r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"
model_provider = "mock_provider"
suppress_unstable_features_warning = true

[features]
sqlite = true
memories = true

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "http://127.0.0.1:9/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#,
    )
}
