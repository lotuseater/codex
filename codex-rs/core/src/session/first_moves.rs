use std::path::PathBuf;
use std::sync::Arc;

use codex_first_moves::PredictRequest;
use codex_first_moves::format_first_moves_context;
use codex_first_moves::is_legacy_first_moves_context;
use codex_first_moves::predict;
use codex_protocol::items::TurnItem;

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
    if history
        .raw_items()
        .iter()
        .filter_map(parse_turn_item)
        .any(|item| matches!(item, TurnItem::UserMessage(_)))
    {
        return None;
    }

    let session_id = sess.conversation_id.to_string();
    let bundle = match predict(PredictRequest {
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
            return None;
        }
    };

    format_first_moves_context(&bundle, config)
}

pub(super) fn merge_first_moves_context(
    first_moves_context: Option<String>,
    mut hook_contexts: Vec<String>,
) -> Vec<String> {
    let Some(first_moves_context) = first_moves_context else {
        return hook_contexts;
    };
    hook_contexts.retain(|context| !is_legacy_first_moves_context(context));
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
                "keep me".to_string(),
            ],
        );

        assert_eq!(
            contexts,
            vec![
                "<first_moves>\nnative\n</first_moves>".to_string(),
                "keep me".to_string(),
            ]
        );
    }
}
