pub(crate) const DEFAULT_MULTI_AGENT_V2_ROOT_USAGE_HINT_TEXT: &str =
    codex_agent_policy::DEFAULT_MULTI_AGENT_V2_ROOT_USAGE_HINT_TEXT;

pub(crate) const DEFAULT_MULTI_AGENT_V2_SUBAGENT_USAGE_HINT_TEXT: &str =
    codex_agent_policy::DEFAULT_MULTI_AGENT_V2_SUBAGENT_USAGE_HINT_TEXT;

pub(crate) const MULTI_AGENT_V2_NESTED_SPAWN_REJECTION: &str =
    codex_agent_policy::MULTI_AGENT_V2_NESTED_SPAWN_REJECTION;

pub(crate) use codex_agent_policy::MultiAgentV2SpawnLineage;
pub(crate) use codex_agent_policy::MultiAgentV2SpawnParent;

pub(crate) fn next_thread_spawn_depth(parent_depth: i32) -> i32 {
    codex_agent_policy::next_thread_spawn_depth(parent_depth)
}

pub(crate) fn exceeds_thread_spawn_depth_limit(depth: i32, max_depth: i32) -> bool {
    codex_agent_policy::exceeds_thread_spawn_depth_limit(depth, max_depth)
}

pub(crate) fn root_can_spawn_child(lineage: MultiAgentV2SpawnLineage) -> bool {
    codex_agent_policy::multi_agent_v2_root_can_spawn_child(lineage)
}
