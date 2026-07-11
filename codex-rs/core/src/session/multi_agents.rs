use crate::config::MultiAgentV2Config;
use crate::context::ContextualUserFragment;
use crate::context::InternalContextSource;
use crate::context::InternalModelContextFragment;
use crate::session::turn_context::TurnContext;
use codex_protocol::config_types::MultiAgentMode;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ReasoningEffort;
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

/// Builds the delegation usage-hint history item on the configured channel:
/// a hidden contextual-user fragment when `delegation_injection_role = "user"`
/// (obeyed by the model, UI-hidden, dropped by compaction retention), otherwise
/// the legacy developer-role update item.
pub(crate) fn build_usage_hint_item(
    multi_agent_v2: &MultiAgentV2Config,
    sections: Vec<String>,
) -> Option<ResponseItem> {
    if sections.is_empty() {
        return None;
    }

    if multi_agent_v2.inject_delegation_as_user() {
        let text = sections.join("\n\n");
        Some(ContextualUserFragment::into(
            InternalModelContextFragment::new(
                InternalContextSource::from_static("multi_agent_usage_hint"),
                text,
            ),
        ))
    } else {
        crate::context_manager::updates::build_developer_update_item(sections)
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

pub(crate) fn effective_multi_agent_mode(turn_context: &TurnContext) -> Option<MultiAgentMode> {
    if turn_context.multi_agent_version != MultiAgentVersion::V2 {
        return None;
    }

    // A configured hint, including an empty string, defines a custom policy instead of an
    // effort-derived built-in policy.
    let multi_agent_mode = match &turn_context
        .config
        .multi_agent_v2
        .multi_agent_mode_hint_text
    {
        Some(hint_text) => MultiAgentMode::Custom(hint_text.clone()),
        None => match turn_context.effective_reasoning_effort() {
            Some(ReasoningEffort::Ultra) => MultiAgentMode::Proactive,
            _ => MultiAgentMode::ExplicitRequestOnly,
        },
    };

    match &turn_context.session_source {
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
