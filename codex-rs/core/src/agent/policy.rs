use codex_protocol::protocol::SessionSource;

/// Re-export the baked-in default delegation-K threshold so `crate::agent::policy`
/// is the single in-core access point for plan-token-economy policy values
/// (mirrors the `default_multi_agent_v2_*` wrappers below).
pub(crate) use codex_agent_policy::DEFAULT_PLAN_TOKEN_ECONOMY_DELEGATION_K;

/// Re-export the nested-spawn rejection message so test assertions in
/// `crate::tools::handlers::multi_agents_tests` reach it via `crate::agent::policy`.
#[cfg(test)]
pub(crate) use codex_agent_policy::MULTI_AGENT_V2_NESTED_SPAWN_REJECTION;

pub(crate) fn default_multi_agent_v2_root_usage_hint_text() -> String {
    codex_agent_policy::default_multi_agent_v2_root_usage_hint_text()
}

pub(crate) fn default_multi_agent_v2_subagent_usage_hint_text() -> String {
    codex_agent_policy::default_multi_agent_v2_subagent_usage_hint_text()
}

pub(crate) fn next_thread_spawn_depth(parent_depth: i32) -> i32 {
    codex_agent_policy::next_thread_spawn_depth(parent_depth)
}

pub(crate) fn next_thread_spawn_depth_for_session_source(session_source: &SessionSource) -> i32 {
    codex_agent_policy::next_thread_spawn_depth_for_session_source(session_source)
}

pub(crate) fn exceeds_thread_spawn_depth_limit(depth: i32, max_depth: i32) -> bool {
    codex_agent_policy::exceeds_thread_spawn_depth_limit(depth, max_depth)
}
