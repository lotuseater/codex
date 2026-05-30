use super::super::*;

pub(super) fn test_thread_goal(
    status: codex_app_server_protocol::ThreadGoalStatus,
    token_budget: Option<i64>,
    tokens_used: i64,
) -> codex_app_server_protocol::ThreadGoal {
    codex_app_server_protocol::ThreadGoal {
        thread_id: "thread-1".to_string(),
        objective: "Keep improving the benchmark".to_string(),
        status,
        token_budget,
        tokens_used,
        time_used_seconds: 30 * 60,
        created_at: 0,
        updated_at: 0,
    }
}

pub(super) fn hook_started_run(
    id: &str,
    event_name: codex_app_server_protocol::HookEventName,
    status_message: Option<&str>,
) -> codex_app_server_protocol::HookRunSummary {
    hook_run_summary(
        id,
        event_name,
        codex_app_server_protocol::HookRunStatus::Running,
        status_message,
        Vec::new(),
    )
}

pub(super) fn hook_completed_run(
    id: &str,
    event_name: codex_app_server_protocol::HookEventName,
    status: codex_app_server_protocol::HookRunStatus,
    entries: Vec<codex_app_server_protocol::HookOutputEntry>,
) -> codex_app_server_protocol::HookRunSummary {
    hook_run_summary(
        id, event_name, status, /*status_message*/ None, entries,
    )
}

pub(super) fn hook_run_summary(
    id: &str,
    event_name: codex_app_server_protocol::HookEventName,
    status: codex_app_server_protocol::HookRunStatus,
    status_message: Option<&str>,
    entries: Vec<codex_app_server_protocol::HookOutputEntry>,
) -> codex_app_server_protocol::HookRunSummary {
    codex_app_server_protocol::HookRunSummary {
        id: id.to_string(),
        event_name,
        handler_type: codex_app_server_protocol::HookHandlerType::Command,
        execution_mode: codex_app_server_protocol::HookExecutionMode::Sync,
        scope: codex_app_server_protocol::HookScope::Turn,
        source_path: PathBuf::from(test_path_display("/tmp/hooks.json")).abs(),
        source: codex_app_server_protocol::HookSource::User,
        display_order: 0,
        status,
        status_message: status_message.map(str::to_string),
        started_at: 1,
        completed_at: (status != codex_app_server_protocol::HookRunStatus::Running).then_some(2),
        duration_ms: (status != codex_app_server_protocol::HookRunStatus::Running).then_some(1),
        entries,
    }
}

pub(super) fn hook_live_and_history_snapshot(chat: &ChatWidget, phase: &str, history: &str) -> String {
    let history = if history.is_empty() {
        "<empty>"
    } else {
        history
    };
    format!(
        "{phase}\nlive hooks:\n{}history:\n{history}",
        active_hook_blob(chat),
    )
}
