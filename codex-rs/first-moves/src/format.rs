use crate::types::FirstMoveKind;
use crate::types::FirstMovesBundle;
use crate::types::FirstMovesConfig;

pub fn format_first_moves_context(
    bundle: &FirstMovesBundle,
    config: &FirstMovesConfig,
) -> Option<String> {
    if !config.enabled() || !config.inject_context {
        return None;
    }

    let selected = bundle
        .moves
        .iter()
        .filter(|entry| entry.confidence >= config.min_context_score)
        .take(config.max_context_moves)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return None;
    }

    let mut lines = vec![
        "<first_moves>".to_string(),
        "Native Codex first-moves predictor ran for this task. Treat these as ranked context candidates before broad repo exploration, not commands.".to_string(),
        format!(
            "intent: {}; confidence: {:.2}; repo_namespace: {}",
            bundle.intent, bundle.confidence, bundle.repo_key
        ),
        "recommended_reads:".to_string(),
    ];

    for entry in selected
        .iter()
        .filter(|entry| matches!(entry.kind, FirstMoveKind::Read))
    {
        let Some(path) = entry.path.as_ref() else {
            continue;
        };
        lines.push(format!(
            "- {} ({:.2}) - {}",
            path.display(),
            entry.confidence,
            entry.reason
        ));
    }

    let searches = selected
        .iter()
        .filter(|entry| matches!(entry.kind, FirstMoveKind::Search))
        .collect::<Vec<_>>();
    if !searches.is_empty() {
        lines.push("recommended_searches:".to_string());
        for entry in searches {
            if let Some(query) = entry.query.as_ref() {
                lines.push(format!(
                    "- {} ({:.2}) - {}",
                    query, entry.confidence, entry.reason
                ));
            }
        }
    }

    let excerpts = selected
        .iter()
        .filter_map(|entry| Some((entry.path.as_ref()?, entry.excerpt.as_ref()?)))
        .collect::<Vec<_>>();
    if !excerpts.is_empty() {
        lines.push("prewarmed_excerpts:".to_string());
        for (path, excerpt) in excerpts {
            lines.push(format!("--- {} ---", path.display()));
            lines.push(excerpt.clone());
        }
    }
    lines.push("</first_moves>".to_string());
    Some(lines.join("\n"))
}

pub fn is_legacy_first_moves_context(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("<first_moves")
        || lower.contains("first_moves_predict")
        || lower.contains("first-moves predictor")
        || lower.contains("first moves predictor")
}
