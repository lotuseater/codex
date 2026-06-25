use crate::config::MultiAgentV2Config;
use crate::session::turn_context::TurnContext;
use codex_protocol::config_types::MultiAgentMode;
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

fn configured_usage_hint_text_for_source<'a>(
    multi_agent_v2: &'a MultiAgentV2Config,
    session_source: &SessionSource,
) -> Option<&'a str> {
    match session_source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. }) => {
            multi_agent_v2.subagent_usage_hint_text.as_deref()
        }
        SessionSource::Cli
        | SessionSource::VSCode
        | SessionSource::Exec
        | SessionSource::Mcp
        | SessionSource::Custom(_)
        | SessionSource::Unknown => multi_agent_v2.root_agent_usage_hint_text.as_deref(),
        SessionSource::Internal(_) | SessionSource::SubAgent(_) => None,
    }
}

pub(crate) fn effective_multi_agent_mode(
    multi_agent_version: MultiAgentVersion,
    session_source: &SessionSource,
    multi_agent_mode: MultiAgentMode,
) -> Option<MultiAgentMode> {
    if multi_agent_version != MultiAgentVersion::V2 {
        return None;
    }

    match session_source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })
        | SessionSource::Cli
        | SessionSource::VSCode
        | SessionSource::Exec
        | SessionSource::Mcp
        | SessionSource::Custom(_)
        | SessionSource::Unknown => Some(multi_agent_mode),
        SessionSource::Internal(_) | SessionSource::SubAgent(_) => None,
    }
}
