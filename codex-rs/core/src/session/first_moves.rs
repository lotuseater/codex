use std::path::PathBuf;
use std::sync::Arc;

use codex_context_pack::ContextPackRequest;
use codex_context_pack::has_context_pack;
use codex_context_pack::is_explicit_repo_routing_prompt;
use codex_context_pack::render_graphify_scout_pack;
use codex_first_moves::PredictRequest;
use codex_first_moves::format_first_moves_context;
use codex_first_moves::is_legacy_first_moves_context;
use codex_first_moves::is_whole_repo_exploration_prompt;
use codex_first_moves::predict;
use codex_protocol::items::TurnItem;
use codex_protocol::permissions::FileSystemSandboxKind;
use codex_repo_context_scout::ScoutCommandMode;
use codex_repo_context_scout::ScoutRequest;
use codex_repo_context_scout::ScoutTrigger;
use codex_repo_context_scout::run_shadow;

use crate::event_mapping::parse_turn_item;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;

pub(super) async fn first_moves_context_for_fresh_turn(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    prompt: &str,
) -> Option<String> {
    let config = &turn_context.config.first_moves;
    if !config.enabled() || !config.inject_context {
        return None;
    }

    let history = sess.clone_history().await;
    let has_visible_user_turn = history
        .raw_items()
        .iter()
        .filter_map(parse_turn_item)
        .any(|item| matches!(item, TurnItem::UserMessage(_)));
    if has_visible_user_turn && !should_inject_later_context(prompt) {
        return None;
    }

    let context_pack = context_pack_for_fresh_turn(turn_context, prompt);
    let session_id = sess.conversation_id.to_string();
    let first_moves_context = match predict(PredictRequest {
        project_root: turn_context.cwd.as_path(),
        codex_home: turn_context.config.codex_home.as_path(),
        prompt,
        session_id: Some(session_id.as_str()),
        config: config.clone(),
        already_loaded_paths: vec![PathBuf::from("AGENTS.md")],
        record_prediction: true,
    })
    .await
    {
        Ok(bundle) => bundle,
        Err(err) => {
            tracing::trace!("native first-moves prediction skipped: {err}");
            return context_pack;
        }
    };

    combine_context_pack_and_first_moves(
        context_pack,
        format_first_moves_context(&first_moves_context, config),
    )
}

fn should_inject_later_context(prompt: &str) -> bool {
    is_whole_repo_exploration_prompt(prompt) || is_explicit_repo_routing_prompt(prompt)
}

fn context_pack_for_fresh_turn(turn_context: &TurnContext, prompt: &str) -> Option<String> {
    if turn_context
        .environments
        .primary()
        .is_some_and(|environment| environment.environment.is_remote())
    {
        return None;
    }
    if matches!(
        turn_context.file_system_sandbox_policy().kind,
        FileSystemSandboxKind::Restricted | FileSystemSandboxKind::ExternalSandbox
    ) {
        return None;
    }
    render_graphify_scout_pack(&ContextPackRequest::new(turn_context.cwd.as_path(), prompt))
}

fn combine_context_pack_and_first_moves(
    context_pack: Option<String>,
    first_moves_context: Option<String>,
) -> Option<String> {
    match (context_pack, first_moves_context) {
        (Some(context_pack), Some(first_moves_context)) => {
            Some(format!("{context_pack}\n\n{first_moves_context}"))
        }
        (Some(context_pack), None) => Some(context_pack),
        (None, Some(first_moves_context)) => Some(first_moves_context),
        (None, None) => None,
    }
}

pub(super) async fn spawn_repo_context_scout_shadow_for_fresh_turn(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    prompt: &str,
) {
    let config = turn_context.config.repo_context_scout;
    if !config.mode.shadow_enabled() {
        return;
    }
    if turn_context
        .environments
        .primary()
        .is_some_and(|environment| environment.environment.is_remote())
    {
        tracing::trace!("repo context scout shadow skipped for remote environment");
        return;
    }
    if matches!(
        turn_context.file_system_sandbox_policy().kind,
        FileSystemSandboxKind::Restricted | FileSystemSandboxKind::ExternalSandbox
    ) {
        tracing::trace!("repo context scout shadow skipped for restricted filesystem turn");
        return;
    }
    let history = sess.clone_history().await;
    if history
        .raw_items()
        .iter()
        .filter_map(parse_turn_item)
        .any(|item| matches!(item, TurnItem::UserMessage(_)))
    {
        return;
    }

    let project_root = turn_context.cwd.to_path_buf();
    let codex_home = turn_context.config.codex_home.to_path_buf();
    let prompt = prompt.to_string();
    let _ = tokio::task::spawn_blocking(move || {
        if let Err(err) = run_shadow(ScoutRequest {
            project_root: project_root.as_path(),
            codex_home: codex_home.as_path(),
            prompt: prompt.as_str(),
            config,
            mode: ScoutCommandMode::Scout,
            trigger: ScoutTrigger::FreshTurn,
        }) {
            tracing::trace!("repo context scout shadow skipped: {err}");
        }
    });
}

pub(super) fn merge_first_moves_context(
    first_moves_context: Option<String>,
    mut hook_contexts: Vec<String>,
) -> Vec<String> {
    let Some(first_moves_context) = first_moves_context else {
        return hook_contexts;
    };
    hook_contexts
        .retain(|context| !is_legacy_first_moves_context(context) && !has_context_pack(context));
    hook_contexts.insert(0, first_moves_context);
    hook_contexts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_native_context_drops_legacy_first_moves_hook_context() {
        let contexts = merge_first_moves_context(
            Some("<first_moves>\nnative\n</first_moves>".to_string()),
            vec![
                "first_moves_predict warmed AGENTS.md".to_string(),
                "FIRST_READS: keep this normal contract text".to_string(),
                "SCOUT_EVIDENCE: keep caller-provided scout contract".to_string(),
                "keep me".to_string(),
            ],
        );

        assert_eq!(
            contexts,
            vec![
                "<first_moves>\nnative\n</first_moves>".to_string(),
                "FIRST_READS: keep this normal contract text".to_string(),
                "SCOUT_EVIDENCE: keep caller-provided scout contract".to_string(),
                "keep me".to_string(),
            ]
        );
    }

    #[test]
    fn later_context_is_allowed_only_for_repo_routing_prompts() {
        assert!(should_inject_later_context(
            "please inspect the whole repo before changing it"
        ));
        assert!(should_inject_later_context(
            "where is spawn_agent implemented in this repo?"
        ));
        assert!(!should_inject_later_context(
            "please inspect codex-rs/core/src/session/first_moves.rs"
        ));
        assert!(!should_inject_later_context("go on"));
    }

    #[test]
    fn combine_context_pack_and_first_moves_keeps_both_contexts() {
        assert_eq!(
            combine_context_pack_and_first_moves(
                Some("<context_pack>\npack\n</context_pack>".to_string()),
                Some("<first_moves>\nnative\n</first_moves>".to_string()),
            ),
            Some(
                "<context_pack>\npack\n</context_pack>\n\n<first_moves>\nnative\n</first_moves>"
                    .to_string()
            ),
        );
    }
}
