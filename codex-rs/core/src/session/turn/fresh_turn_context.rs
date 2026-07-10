use std::sync::Arc;

use crate::session::TurnInput;
use crate::session::desktop_automation::desktop_automation_context_for_prompt;
use crate::session::desktop_automation::merge_desktop_automation_context;
use crate::session::first_moves::first_moves_context_for_fresh_turn;
use crate::session::first_moves::merge_first_moves_context;
use crate::session::first_moves::spawn_repo_context_scout_shadow_for_fresh_turn;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::user_input::UserInput;

/// fork-local: gather the fresh-turn additional contexts (first-moves /
/// repo-context-scout / desktop-automation / blackboard) for a fresh user turn.
/// Returns the merged `additional_contexts` the caller folds into the recorded
/// input; empty when the turn has no user prompt text. Mirrors the block that
/// previously lived inline in `run_turn`.
pub(crate) async fn gather_fresh_turn_additional_contexts(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    input: &[TurnInput],
) -> Vec<String> {
    let prompt = user_prompt_messages(input).join("\n");
    let mut additional_contexts = Vec::new();
    if !prompt.is_empty() {
        spawn_repo_context_scout_shadow_for_fresh_turn(sess, turn_context, prompt.as_str()).await;
        let first_moves_context =
            first_moves_context_for_fresh_turn(sess, turn_context, prompt.as_str()).await;
        let desktop_automation_context = desktop_automation_context_for_prompt(
            turn_context.config.desktop_automation,
            prompt.as_str(),
        );
        let blackboard_context = sess
            .services
            .blackboard
            .context_for_turn(turn_context.cwd.as_path(), prompt.as_str())
            .await;
        additional_contexts = merge_desktop_automation_context(
            desktop_automation_context,
            merge_first_moves_context(first_moves_context, additional_contexts),
        );
        if let Some(blackboard_context) = blackboard_context {
            additional_contexts.push(blackboard_context);
        }
    }
    additional_contexts
}

/// fork-local: fuse the auto-coordinator framing into a turn's input, gated by
/// multi-agent V2 + the resolved AutoCoordinatorMode heuristic over the input's
/// user text. On the user channel the framing is appended as a trailing text
/// block on the last user item (recorded as one user message by
/// `run_hooks_and_record_inputs`); on the developer channel it is pushed onto
/// `additional_contexts` instead. Returns `false` without side effects when the
/// framing should not fire (no user text in `input`, non-V2 session,
/// coordinator off, or a non-decomposable prompt under `Auto`); returns `true`
/// when it fused, so `run_turn` can bound the framing to at most one copy per
/// turn across the fresh-input and pending-drain call sites.
pub(crate) fn fuse_auto_coordinator_framing(
    turn_context: &TurnContext,
    input: &mut [TurnInput],
    additional_contexts: &mut Vec<String>,
) -> bool {
    let prompt = user_prompt_messages(input).join("\n");
    if prompt.is_empty() {
        return false;
    }
    if turn_context.multi_agent_version != MultiAgentVersion::V2
        || !turn_context
            .config
            .multi_agent_v2
            .should_inject_auto_coordinator(prompt.as_str())
    {
        return false;
    }
    if turn_context
        .config
        .multi_agent_v2
        .inject_delegation_as_user()
    {
        // User channel: fuse the framing into the SAME role:"user" prompt so
        // the model obeys it (a developer-role message is discounted). Append
        // it as a trailing text block on the user turn's content vec, recorded
        // as one user message by run_hooks_and_record_inputs.
        if let Some(content) = input.iter_mut().rev().find_map(|item| match item {
            TurnInput::UserInput { content, .. } => Some(content),
            _ => None,
        }) {
            content.push(UserInput::Text {
                text: codex_agent_policy::AUTO_COORDINATOR_FRAMING_TEXT.to_string(),
                text_elements: Vec::new(),
            });
        }
    } else {
        additional_contexts.push(codex_agent_policy::AUTO_COORDINATOR_FRAMING_TEXT.to_string());
    }
    true
}

fn user_prompt_messages(input: &[TurnInput]) -> Vec<String> {
    flattened_user_input(input)
        .into_iter()
        .filter_map(|item| match item {
            UserInput::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

fn flattened_user_input(input: &[TurnInput]) -> Vec<UserInput> {
    input
        .iter()
        .filter_map(|item| match item {
            TurnInput::UserInput { content, .. } => Some(content.as_slice()),
            TurnInput::ResponseItem(_) | TurnInput::InterAgentCommunication(_) => None,
        })
        .flatten()
        .cloned()
        .collect()
}
