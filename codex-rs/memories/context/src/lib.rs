use std::collections::BTreeSet;
use std::path::Path;

use codex_utils_absolute_path::AbsolutePathBuf;
use serde_json::Value;

pub struct ProjectProblemMemoryContextRequest<'a> {
    pub codex_home: &'a AbsolutePathBuf,
    pub project_root: &'a Path,
    pub prompt: &'a str,
    pub max_matches: usize,
}

pub async fn build_project_problem_memory_context(
    request: ProjectProblemMemoryContextRequest<'_>,
) -> Option<String> {
    let prompt_terms = prompt_terms(request.prompt);
    if prompt_terms.is_empty() {
        return None;
    }

    let memory_root = memory_root(request.codex_home);
    let project_root = request
        .project_root
        .display()
        .to_string()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let max_matches = request.max_matches.clamp(1, 20);
    let mut matches = Vec::new();

    for index_name in ["project_index.jsonl", "problem_index.jsonl"] {
        let path = memory_root.join(index_name);
        let Ok(contents) = tokio::fs::read_to_string(path.as_path()).await else {
            continue;
        };
        for line in contents.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let text = value.to_string().to_ascii_lowercase();
            let cwd = value
                .get("cwd")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .replace('\\', "/")
                .to_ascii_lowercase();
            if !memory_entry_matches_project_scope(cwd.as_str(), project_root.as_str()) {
                continue;
            }
            let Some(score) =
                score_project_problem_memory_match(&prompt_terms, text.as_str(), !cwd.is_empty())
            else {
                continue;
            };
            matches.push((score, index_name, value));
        }
    }

    matches.sort_by(|left, right| right.0.cmp(&left.0));
    matches.truncate(max_matches);
    if matches.is_empty() {
        return None;
    }

    let mut lines = vec![
        "<project_problem_memory>".to_string(),
        "USAGE: routing evidence only; current repo state and live verification win.".to_string(),
    ];
    for (score, source, value) in matches {
        let summary = compact_context_text(
            value
                .get("rollout_summary")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            360,
        );
        let file = value
            .get("rollout_summary_file")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let family = value
            .get("problem_family")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let edit_surfaces = metadata_array_summary(&value, "edit_surfaces", 4);
        let routing_keywords = metadata_array_summary(&value, "routing_keywords", 4);
        let mut line = format!(
            "- score={score} source={source} rollout_summary_file={file} problem_family={family}: {summary}"
        );
        if !edit_surfaces.is_empty() {
            line.push_str(" edit_surfaces=[");
            line.push_str(edit_surfaces.as_str());
            line.push(']');
        }
        if !routing_keywords.is_empty() {
            line.push_str(" routing_keywords=[");
            line.push_str(routing_keywords.as_str());
            line.push(']');
        }
        lines.push(line);
    }
    lines.push("</project_problem_memory>".to_string());
    Some(lines.join("\n"))
}

fn memory_root(codex_home: &AbsolutePathBuf) -> AbsolutePathBuf {
    codex_home.join("memories")
}

fn metadata_array_summary(value: &Value, key: &str, max_items: usize) -> String {
    let mut summary = String::new();
    for item in value
        .get("metadata")
        .and_then(|metadata| metadata.get(key))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|item| compact_context_text(item, 80))
        .filter(|item| !item.is_empty())
        .take(max_items)
    {
        if !summary.is_empty() {
            summary.push_str(", ");
        }
        summary.push_str(item.as_str());
    }
    summary
}

fn compact_context_text(value: &str, max_chars: usize) -> String {
    let mut normalized = String::new();
    for segment in value.split_whitespace() {
        if !normalized.is_empty() {
            normalized.push(' ');
        }
        normalized.push_str(segment);
    }
    if normalized.chars().count() <= max_chars {
        return normalized;
    }

    let keep = max_chars.saturating_sub(3);
    let mut compact = normalized.chars().take(keep).collect::<String>();
    compact.push_str("...");
    compact
}

fn prompt_terms(prompt: &str) -> Vec<String> {
    prompt
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .map(str::trim)
        .filter(|term| term.len() >= 3)
        .map(str::to_ascii_lowercase)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn score_project_problem_memory_match(
    prompt_terms: &[String],
    text: &str,
    scoped: bool,
) -> Option<usize> {
    let term_hits = prompt_terms
        .iter()
        .filter(|term| text.contains(term.as_str()))
        .count();
    if term_hits == 0 {
        return None;
    }
    Some(term_hits + usize::from(scoped) * 4)
}

fn memory_entry_matches_project_scope(cwd: &str, project_root: &str) -> bool {
    if cwd.is_empty() {
        return true;
    }

    path_text_has_boundary_prefix(cwd, project_root)
        || path_text_has_boundary_prefix(project_root, cwd)
}

fn path_text_has_boundary_prefix(path: &str, prefix: &str) -> bool {
    let path = path.trim_end_matches('/');
    let prefix = prefix.trim_end_matches('/');
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use codex_utils_absolute_path::AbsolutePathBuf;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn project_problem_memory_scope_rejects_other_repos() {
        assert!(memory_entry_matches_project_scope(
            "c:/users/oleh/documents/github/open_ai/codex",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
        assert!(memory_entry_matches_project_scope(
            "c:/users/oleh/documents/github/open_ai/codex/codex-rs",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
        assert!(!memory_entry_matches_project_scope(
            "c:/users/oleh/documents/github/other_repo",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
        assert!(!memory_entry_matches_project_scope(
            "c:/users/oleh/documents/github/open_ai/codex-old",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
    }

    #[test]
    fn project_problem_memory_scope_boost_requires_term_match() {
        let terms = prompt_terms("donut physics");

        assert_eq!(
            score_project_problem_memory_match(
                &terms,
                "same repo memory about deployment wrappers",
                true,
            ),
            None
        );
        assert_eq!(
            score_project_problem_memory_match(
                &terms,
                "same repo memory about donut rendering physics",
                true,
            ),
            Some(6)
        );
    }

    #[test]
    fn project_problem_memory_summary_stays_compact() {
        let noisy_summary = "one\n\n two\tthree ".repeat(80);

        let compact = compact_context_text(noisy_summary.as_str(), 40);

        assert_eq!(compact.chars().count(), 40);
        assert!(compact.ends_with("..."));
        assert!(!compact.contains('\n'));
        assert!(!compact.contains('\t'));
    }

    #[test]
    fn metadata_array_summary_limits_items_and_width() {
        let value = json!({
            "metadata": {
                "edit_surfaces": [
                    "codex-rs/core/src/session/first_moves.rs with a very long detail that should be clipped",
                    "codex-rs/first-moves/src/predict.rs",
                    "codex-rs/context-pack/src/lib.rs",
                    "codex-rs/tools/src/cognos_ops.rs",
                    "codex-rs/extra/ignored.rs"
                ]
            }
        });

        let summary = metadata_array_summary(&value, "edit_surfaces", 2);

        assert!(summary.contains("codex-rs/core/src/session/first_moves.rs"));
        assert!(summary.contains("codex-rs/first-moves/src/predict.rs"));
        assert!(!summary.contains("context-pack"));
        assert!(summary.split(", ").all(|item| item.chars().count() <= 80));
    }

    #[tokio::test]
    async fn build_project_problem_context_filters_and_renders_matches() {
        let home = TempDir::new().expect("create temp codex home");
        let memories = home.path().join("memories");
        tokio::fs::create_dir_all(&memories)
            .await
            .expect("create memories dir");
        let matching = json!({
            "cwd": "C:/Users/Oleh/Documents/GitHub/open_ai/codex",
            "rollout_summary_file": "rollout.md",
            "problem_family": "routing",
            "rollout_summary": "Donut rendering physics bug was fixed in the first moves path.",
            "metadata": {
                "edit_surfaces": ["codex-rs/core/src/session/first_moves.rs"],
                "routing_keywords": ["donut", "physics"]
            }
        });
        let other_repo = json!({
            "cwd": "C:/Users/Oleh/Documents/GitHub/other",
            "rollout_summary": "Donut physics in another repo",
            "metadata": {}
        });
        tokio::fs::write(
            memories.join("project_index.jsonl"),
            format!("{matching}\n{other_repo}\n"),
        )
        .await
        .expect("write project index");

        let context = build_project_problem_memory_context(ProjectProblemMemoryContextRequest {
            codex_home: &AbsolutePathBuf::try_from(home.path().to_path_buf())
                .expect("tempdir is absolute"),
            project_root: Path::new("C:/Users/Oleh/Documents/GitHub/open_ai/codex"),
            prompt: "debug donut physics",
            max_matches: 3,
        })
        .await
        .expect("expected memory context");

        assert!(context.contains("<project_problem_memory>"));
        assert!(context.contains("rollout_summary_file=rollout.md"));
        assert!(context.contains("edit_surfaces=[codex-rs/core/src/session/first_moves.rs]"));
        assert!(!context.contains("another repo"));
    }
}
