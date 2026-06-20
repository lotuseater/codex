use crate::session::turn_context::TurnContext;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;

pub(super) fn usage_hint_text(
    turn_context: &TurnContext,
    session_source: &SessionSource,
) -> Option<String> {
    if turn_context.multi_agent_version != MultiAgentVersion::V2 {
        return None;
    }

    let multi_agent_v2 = &turn_context.config.multi_agent_v2;
    if !multi_agent_v2.usage_hint_enabled {
        return None;
    }

    let k = multi_agent_v2.plan_token_economy_delegation_k;

    match session_source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. }) => Some(match &multi_agent_v2
            .subagent_usage_hint_text
        {
            Some(text) => text.clone(),
            None => codex_agent_policy::default_multi_agent_v2_subagent_usage_hint_text_with_k(k),
        }),
        SessionSource::Cli
        | SessionSource::VSCode
        | SessionSource::Exec
        | SessionSource::Mcp
        | SessionSource::Custom(_)
        | SessionSource::Unknown => Some(match &multi_agent_v2.root_agent_usage_hint_text {
            Some(text) => text.clone(),
            None => codex_agent_policy::default_multi_agent_v2_root_usage_hint_text_with_k(k),
        }),
        SessionSource::Internal(_) | SessionSource::SubAgent(_) => None,
    }
}
