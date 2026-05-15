use crate::config::Config;
use codex_blackboard::BlackboardSessionConfig;
use codex_blackboard::BlackboardSessionMode;
use codex_blackboard::BlackboardSessionOptions;
use codex_config::types::BlackboardConfig;
use codex_protocol::protocol::SessionSource;
use std::sync::Arc;

pub(crate) use codex_blackboard::BlackboardSession;

pub(crate) fn new_blackboard_session(
    config: &Config,
    session_id: String,
    thread_id: String,
    session_source: &SessionSource,
) -> Arc<BlackboardSession> {
    BlackboardSession::new(BlackboardSessionOptions {
        config: session_config_from_config(&config.blackboard),
        codex_home: config.codex_home.to_path_buf(),
        session_id,
        thread_id,
        mode: if session_source.is_non_root_agent() {
            BlackboardSessionMode::NonRootAgent
        } else {
            BlackboardSessionMode::Root
        },
    })
}

fn session_config_from_config(config: &BlackboardConfig) -> BlackboardSessionConfig {
    BlackboardSessionConfig {
        path: config.path.clone(),
        global_index_path: config.global_index_path.clone(),
        poll_interval_ms: config.poll_interval_ms,
        heartbeat_interval_seconds: config.heartbeat_interval_seconds,
        stale_after_seconds: config.stale_after_seconds,
        recent_window_seconds: config.recent_window_seconds,
        max_injected_bytes: config.max_injected_bytes,
        max_entry_chars: config.max_entry_chars,
        max_file_bytes: config.max_file_bytes,
        max_joined_repos: config.max_joined_repos,
        enabled: config.enabled,
    }
}
