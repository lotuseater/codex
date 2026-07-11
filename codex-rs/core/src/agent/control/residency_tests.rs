use crate::ThreadManager;
use crate::agent::AgentControl;
use crate::agent::AgentStatus;
use crate::codex_thread::CodexThread;
use crate::config::Config;
use crate::config::test_config;
use crate::thread_manager::ThreadManagerState;
use codex_features::Feature;
use codex_login::CodexAuth;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::ThreadSource;
use codex_protocol::protocol::TurnAbortReason;
use codex_protocol::protocol::TurnAbortedEvent;
use codex_protocol::protocol::TurnCompleteEvent;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn residency_slot_reservation_unloads_oldest_idle_v2_agent() {
    let mut config = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    config.multi_agent_v2.max_concurrent_threads_per_session = 2;
    let temp_home = tempfile::tempdir().expect("create temp home");
    config.codex_home = temp_home.path().to_path_buf().try_into().unwrap();
    config.cwd = temp_home.path().to_path_buf().try_into().unwrap();
    let manager = v2_thread_manager(&config);
    let root = manager
        .start_thread(config.clone())
        .await
        .expect("start root thread");
    let control = manager.agent_control();
    let state = control.upgrade().expect("thread manager should be live");

    let first_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("first resident slot");
    let first =
        spawn_v2_subagent(&control, &state, config.clone(), root.thread_id, "worker-1").await;
    first_slot.commit(first.thread_id);
    mark_thread_completed(first.thread.as_ref()).await;

    let second_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("second resident slot should evict the first idle agent");
    match manager.get_thread(first.thread_id).await {
        Err(CodexErr::ThreadNotFound(thread_id)) => assert_eq!(thread_id, first.thread_id),
        Err(err) => panic!("expected evicted thread to be missing, got {err:?}"),
        Ok(_) => panic!("expected evicted thread to be missing"),
    }
    let second = spawn_v2_subagent(&control, &state, config, root.thread_id, "worker-2").await;
    second_slot.commit(second.thread_id);

    assert!(manager.get_thread(root.thread_id).await.is_ok());
    assert!(manager.get_thread(second.thread_id).await.is_ok());
}

#[tokio::test]
async fn interrupted_v2_agent_is_lost_after_residency_eviction() {
    let mut config = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    config.multi_agent_v2.max_concurrent_threads_per_session = 2;
    let temp_home = tempfile::tempdir().expect("create temp home");
    config.codex_home = temp_home.path().to_path_buf().try_into().unwrap();
    config.cwd = temp_home.path().to_path_buf().try_into().unwrap();
    let manager = v2_thread_manager(&config);
    let root = manager
        .start_thread(config.clone())
        .await
        .expect("start root thread");
    let control = manager.agent_control();
    let state = control.upgrade().expect("thread manager should be live");

    let first_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("first resident slot");
    let first =
        spawn_v2_subagent(&control, &state, config.clone(), root.thread_id, "worker-1").await;
    first_slot.commit(first.thread_id);
    mark_thread_interrupted(first.thread.as_ref()).await;

    let second_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("second resident slot should evict the first interrupted idle agent");
    match manager.get_thread(first.thread_id).await {
        Err(CodexErr::ThreadNotFound(thread_id)) => assert_eq!(thread_id, first.thread_id),
        Err(err) => panic!("expected evicted thread to be missing, got {err:?}"),
        Ok(_) => panic!("expected evicted thread to be missing"),
    }
    let second =
        spawn_v2_subagent(&control, &state, config.clone(), root.thread_id, "worker-2").await;
    second_slot.commit(second.thread_id);
    mark_thread_completed(second.thread.as_ref()).await;

    let err = control
        .ensure_v2_agent_loaded(config, first.thread_id)
        .await
        .expect_err("evicted interrupted agent should stay lost");
    match err {
        CodexErr::ThreadNotFound(thread_id) => assert_eq!(thread_id, first.thread_id),
        err => panic!("expected ThreadNotFound, got {err:?}"),
    }

    assert!(manager.get_thread(root.thread_id).await.is_ok());
    assert!(manager.get_thread(second.thread_id).await.is_ok());
    match manager.get_thread(first.thread_id).await {
        Err(CodexErr::ThreadNotFound(thread_id)) => assert_eq!(thread_id, first.thread_id),
        Err(err) => panic!("expected evicted thread to be missing, got {err:?}"),
        Ok(_) => panic!("expected evicted thread to be missing"),
    }
}

/// A sub-agent that is spawned but abandoned before it ever runs stays in
/// `PendingInit` with no active turn and an empty mailbox. This is the state a
/// coordinator leaves behind when it gives up on a worker that never started:
/// `interrupt_agent` on such an agent is a no-op (an interrupt only aborts an
/// in-flight turn, and there is none), so it never reaches a terminal event. That
/// slot must still be reclaimable, otherwise the residency budget leaks for the
/// whole session and later spawns fail with `AgentLimitReached`. Eviction of a
/// `PendingInit` resident is age-gated (a healthy agent is only *transiently*
/// `PendingInit` right after commit); this test pins the grace to zero so an
/// abandoned agent is reclaimed immediately, and its companion test
/// (`fresh_pending_init_v2_agent_is_protected_within_grace`) covers the guard that
/// keeps a *fresh* agent from being evicted during that startup window.
#[tokio::test]
async fn abandoned_pending_init_v2_agent_is_reclaimed_by_residency_eviction() {
    let mut config = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    config.multi_agent_v2.max_concurrent_threads_per_session = 2;
    let temp_home = tempfile::tempdir().expect("create temp home");
    config.codex_home = temp_home.path().to_path_buf().try_into().unwrap();
    config.cwd = temp_home.path().to_path_buf().try_into().unwrap();
    let manager = v2_thread_manager(&config);
    let root = manager
        .start_thread(config.clone())
        .await
        .expect("start root thread");
    let control = manager.agent_control();
    let state = control.upgrade().expect("thread manager should be live");
    // Pin the grace window to zero so the abandoned agent below stands in for one that
    // has already aged past the eviction grace, keeping the test deterministic without
    // waiting on wall-clock time.
    control.v2_residency.set_pending_init_grace(Duration::ZERO);

    // Spawn a sub-agent and abandon it before driving any turn: it stays `PendingInit`
    // with no active turn and nothing queued in its mailbox.
    let first_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("first resident slot");
    let first =
        spawn_v2_subagent(&control, &state, config.clone(), root.thread_id, "worker-1").await;
    first_slot.commit(first.thread_id);
    assert_eq!(
        first.thread.agent_status().await,
        AgentStatus::PendingInit,
        "an un-driven, abandoned sub-agent should sit in PendingInit",
    );

    // Reserving a new slot (what the coordinator's retry spawn does) must reclaim the
    // abandoned agent's slot by evicting it, not fail with AgentLimitReached.
    let second_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("second resident slot should evict the abandoned pending-init agent");
    match manager.get_thread(first.thread_id).await {
        Err(CodexErr::ThreadNotFound(thread_id)) => assert_eq!(thread_id, first.thread_id),
        Err(err) => panic!("expected evicted thread to be missing, got {err:?}"),
        Ok(_) => panic!("expected evicted thread to be missing"),
    }
    let second = spawn_v2_subagent(&control, &state, config, root.thread_id, "worker-2").await;
    second_slot.commit(second.thread_id);

    assert!(manager.get_thread(root.thread_id).await.is_ok());
    assert!(manager.get_thread(second.thread_id).await.is_ok());
}

/// The mirror of the reclaim test, guarding the startup race it introduces. A *fresh*
/// sub-agent is also `PendingInit` with no active turn and an empty mailbox during the
/// brief window after its slot is committed (`reserve_slot` commits before the initial
/// task is enqueued and turned into a running turn). A concurrent spawn's eviction pass
/// must NOT reclaim that healthy agent — doing so would silently drop its queued initial
/// task. The age gate protects it: within the grace window a `PendingInit` resident is
/// not unloadable, so a spawn that cannot otherwise make room fails with
/// `AgentLimitReached` (which the caller can recover from) instead of evicting the live
/// agent.
#[tokio::test]
async fn fresh_pending_init_v2_agent_is_protected_within_grace() {
    let mut config = test_config().await;
    let _ = config.features.enable(Feature::MultiAgentV2);
    config.multi_agent_v2.max_concurrent_threads_per_session = 2;
    let temp_home = tempfile::tempdir().expect("create temp home");
    config.codex_home = temp_home.path().to_path_buf().try_into().unwrap();
    config.cwd = temp_home.path().to_path_buf().try_into().unwrap();
    let manager = v2_thread_manager(&config);
    let root = manager
        .start_thread(config.clone())
        .await
        .expect("start root thread");
    let control = manager.agent_control();
    let state = control.upgrade().expect("thread manager should be live");
    // A generous grace stands in for "this agent was just spawned and is still starting
    // up": its commit is far younger than the window, so it must be treated as healthy.
    control
        .v2_residency
        .set_pending_init_grace(Duration::from_secs(3600));

    // Spawn a sub-agent and leave it in its just-committed `PendingInit` state, exactly as
    // a healthy agent looks in the window before its initial turn starts.
    let first_slot = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect("first resident slot");
    let first =
        spawn_v2_subagent(&control, &state, config.clone(), root.thread_id, "worker-1").await;
    first_slot.commit(first.thread_id);
    assert_eq!(
        first.thread.agent_status().await,
        AgentStatus::PendingInit,
        "a just-spawned sub-agent should sit in PendingInit",
    );

    // A second reservation at capacity must NOT evict the fresh agent: with no evictable
    // resident it fails with AgentLimitReached (recoverable) rather than silently killing
    // the healthy initializing agent and losing its queued task.
    let err = control
        .reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
        .await
        .expect_err("a fresh within-grace PendingInit agent must not be evicted");
    match err {
        CodexErr::AgentLimitReached { max_threads } => assert_eq!(max_threads, 1),
        err => panic!("expected AgentLimitReached, got {err:?}"),
    }

    // The fresh agent is still resident, alive, and unchanged.
    assert!(manager.get_thread(first.thread_id).await.is_ok());
    assert_eq!(
        first.thread.agent_status().await,
        AgentStatus::PendingInit,
        "the protected agent should remain PendingInit and alive",
    );
    assert!(manager.get_thread(root.thread_id).await.is_ok());
}

/// Build a `ThreadManager` wired with a real `StoreLiveThreadFactory` so `start_thread`
/// (and the V2 sub-agent spawns below) can create live threads under
/// `Feature::MultiAgentV2`. The shared `with_models_provider_*_for_tests` constructors
/// hardcode `UnsupportedLiveThreadFactory`, which rejects live-thread creation; this
/// mirrors the production wiring in app-server `message_processor.rs` with test-only
/// inputs, changing only the live-thread factory relative to those constructors.
fn v2_thread_manager(config: &Config) -> ThreadManager {
    ThreadManager::new(
        config,
        crate::test_support::auth_manager_from_auth(CodexAuth::from_api_key("dummy")),
        SessionSource::Exec,
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        codex_extension_api::empty_extension_registry(),
        Arc::new(crate::test_support::EmptyUserInstructionsProvider),
        /*analytics_events_client*/ None,
        crate::thread_manager::thread_store_from_config(config, /*state_db*/ None),
        Arc::new(codex_thread_store::StoreLiveThreadFactory::new()),
        /*state_db*/ None,
        "11111111-1111-4111-8111-111111111111".to_string(),
        /*attestation_provider*/ None,
        /*external_time_provider*/ None,
    )
}

async fn spawn_v2_subagent(
    control: &AgentControl,
    state: &Arc<ThreadManagerState>,
    config: Config,
    parent_thread_id: ThreadId,
    label: &str,
) -> crate::thread_manager::NewThread {
    state
        .spawn_new_thread_with_source(
            config,
            control.clone(),
            SessionSource::SubAgent(SubAgentSource::Other(label.to_string())),
            Some(parent_thread_id),
            /*forked_from_thread_id*/ None,
            Some(ThreadSource::Subagent),
            /*metrics_service_name*/ None,
            /*inherited_environments*/ None,
            /*inherited_exec_policy*/ None,
            /*environments*/ None,
        )
        .await
        .expect("spawn v2 subagent")
}

async fn mark_thread_completed(thread: &CodexThread) {
    let turn = thread.codex.session.new_default_turn().await;
    thread
        .codex
        .session
        .send_event(
            turn.as_ref(),
            EventMsg::TurnComplete(TurnCompleteEvent {
                turn_id: turn.sub_id.clone(),
                last_agent_message: Some("done".to_string()),
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            }),
        )
        .await;
    clear_active_turn(thread).await;
}

async fn mark_thread_interrupted(thread: &CodexThread) {
    let turn = thread.codex.session.new_default_turn().await;
    thread
        .codex
        .session
        .send_event(
            turn.as_ref(),
            EventMsg::TurnAborted(TurnAbortedEvent {
                turn_id: Some(turn.sub_id.clone()),
                reason: TurnAbortReason::Interrupted,
                completed_at: None,
                duration_ms: None,
            }),
        )
        .await;
    clear_active_turn(thread).await;
}

async fn clear_active_turn(thread: &CodexThread) {
    // The fixture has no task runner to clear the turn after the terminal event.
    *thread.codex.session.active_turn.lock().await = None;
}
