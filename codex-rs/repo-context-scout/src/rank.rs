use std::collections::BTreeSet;

use crate::types::ChangedAreas;
use crate::types::RepoContextScoutConfig;
use crate::types::RepoIndex;
use crate::types::ScoutCandidate;
use crate::types::SupportRoute;

pub(crate) fn rank_files(
    index: &RepoIndex,
    changed: &ChangedAreas,
    prompt: &str,
    config: &RepoContextScoutConfig,
) -> Vec<ScoutCandidate> {
    let terms = prompt_terms(prompt);
    let changed_paths = changed
        .paths
        .iter()
        .map(|path| path.path.clone())
        .collect::<BTreeSet<_>>();
    let changed_focused = is_changed_focused(prompt, &terms);
    let mut candidates = index
        .files
        .iter()
        .filter_map(|file| {
            let mut score = 0.0;
            let mut reasons = Vec::new();
            if changed_paths.contains(&file.path) {
                score += if changed_focused { 8.0 } else { 5.0 };
                reasons.push("changed".to_string());
            }
            for term in &terms {
                if file.path.to_ascii_lowercase().contains(term) {
                    score += 2.0;
                    reasons.push(format!("path:{term}"));
                }
                if file
                    .anchors
                    .iter()
                    .any(|anchor| anchor.text.to_ascii_lowercase().contains(term))
                {
                    score += 1.5;
                    reasons.push(format!("anchor:{term}"));
                }
            }
            if changed_focused && is_test_or_build_path(&file.path) {
                score += 1.5;
                reasons.push("verification_area".to_string());
            }
            if score <= 0.0 {
                return None;
            }
            reasons.sort();
            reasons.dedup();
            Some(ScoutCandidate {
                path: file.path.clone(),
                score,
                reasons,
                anchors: file.anchors.iter().take(5).cloned().collect(),
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    if !changed_focused {
        candidates.truncate(config.max_candidates);
        return candidates;
    }

    select_changed_first(candidates, changed, &changed_paths, config.max_candidates)
}

pub(crate) fn support_routes_for_prompt(
    prompt: &str,
    candidates: &[ScoutCandidate],
) -> Vec<SupportRoute> {
    let terms = prompt_terms(prompt);
    let hints = candidates
        .iter()
        .take(12)
        .map(|candidate| candidate.path.clone())
        .collect::<Vec<_>>();
    let mut routes = Vec::new();
    if terms.iter().any(|term| {
        matches!(
            term.as_str(),
            "resume" | "continue" | "history" | "previous"
        )
    }) {
        routes.push(SupportRoute {
            name: "gsd2_artifact_exploration_prompt".to_string(),
            reason: "prior command artifacts may avoid rerunning broad exploration".to_string(),
            path_hints: hints.clone(),
        });
    }
    if terms.iter().any(|term| {
        matches!(
            term.as_str(),
            "architecture" | "topology" | "flow" | "graph"
        )
    }) {
        routes.push(SupportRoute {
            name: "graphify_topology_prompt".to_string(),
            reason: "relationship/topology question".to_string(),
            path_hints: hints.clone(),
        });
    }
    if terms
        .iter()
        .any(|term| matches!(term.as_str(), "handoff" | "review" | "parallel"))
    {
        routes.push(SupportRoute {
            name: "repomix_artifact_context_prompt".to_string(),
            reason: "scoped artifact may help review or handoff".to_string(),
            path_hints: hints.clone(),
        });
    }
    if terms
        .iter()
        .any(|term| matches!(term.as_str(), "symbol" | "reference" | "function" | "type"))
    {
        routes.push(SupportRoute {
            name: "serena_semantic_lookup_prompt".to_string(),
            reason: "symbol lookup can refine candidate files".to_string(),
            path_hints: hints,
        });
    }
    routes
}

fn prompt_terms(prompt: &str) -> Vec<String> {
    let mut terms = prompt
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .map(str::trim)
        .filter(|term| term.len() >= 3)
        .map(str::to_ascii_lowercase)
        .filter(|term| !STOP_WORDS.contains(&term.as_str()))
        .collect::<Vec<_>>();
    terms.sort();
    terms.dedup();
    terms
}

fn is_test_or_build_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("test")
        || lower.ends_with("cargo.toml")
        || lower.ends_with("package.json")
        || lower.starts_with("scripts/")
        || lower.contains("/scripts/")
}

fn is_changed_focused(prompt: &str, terms: &[String]) -> bool {
    let lower = prompt.to_ascii_lowercase();
    terms.iter().any(|term| {
        matches!(
            term.as_str(),
            "review"
                | "fix"
                | "test"
                | "build"
                | "bug"
                | "change"
                | "changes"
                | "patch"
                | "refactor"
                | "update"
                | "improve"
        )
    }) || lower.contains("implement")
}

fn select_changed_first(
    candidates: Vec<ScoutCandidate>,
    changed: &ChangedAreas,
    changed_paths: &BTreeSet<String>,
    max_candidates: usize,
) -> Vec<ScoutCandidate> {
    let mut selected = Vec::new();
    let mut selected_paths = BTreeSet::new();
    for candidate in candidates
        .iter()
        .filter(|candidate| changed_paths.contains(&candidate.path))
    {
        if selected.len() >= max_candidates {
            break;
        }
        selected_paths.insert(candidate.path.clone());
        selected.push(candidate.clone());
    }
    for changed_path in &changed.paths {
        if selected.len() >= max_candidates {
            break;
        }
        if selected_paths.contains(&changed_path.path) {
            continue;
        }
        selected_paths.insert(changed_path.path.clone());
        selected.push(ScoutCandidate {
            path: changed_path.path.clone(),
            score: 7.0,
            reasons: vec!["changed".to_string(), "not_indexed".to_string()],
            anchors: Vec::new(),
        });
    }
    for candidate in candidates {
        if selected.len() >= max_candidates {
            break;
        }
        if selected_paths.insert(candidate.path.clone()) {
            selected.push(candidate);
        }
    }
    selected
}

const STOP_WORDS: &[&str] = &[
    "the",
    "and",
    "for",
    "with",
    "this",
    "that",
    "from",
    "into",
    "please",
    "implement",
    "plan",
    "repo",
    "context",
    "scout",
];
