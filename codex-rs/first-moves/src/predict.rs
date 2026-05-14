use crate::logic::LogicInput;
use crate::logic::assess_candidate;
use crate::shadow::ShadowPredictInput;
use crate::shadow::record_shadow_prediction;
use crate::storage::PathLearning;
use crate::storage::load_learning;
use crate::storage::normalize_path_text;
use crate::storage::record_prediction;
use crate::storage::resolve_repo_root;
use crate::storage::short_hash;
use crate::storage::storage_for;
use crate::types::FirstMove;
use crate::types::FirstMoveKind;
use crate::types::FirstMovesBundle;
use crate::types::FirstMovesConfig;
use crate::types::PredictRequest;
use crate::types::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use walkdir::DirEntry;
use walkdir::WalkDir;

const SKIP_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".codex",
    ".cache",
    ".pytest_cache",
    ".mypy_cache",
    ".venv",
    "__pycache__",
    "bin",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "obj",
    "out",
    "target",
];

const BASELINE_FILES: &[&str] = &[
    "agents.md",
    "readme.md",
    "cargo.toml",
    "package.json",
    "pyproject.toml",
    "cmakelists.txt",
];

const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "toml", "md", "json", "yaml", "yml", "ts", "tsx", "js", "jsx", "py", "ps1", "cpp", "hpp",
    "h", "c", "cc", "go", "java", "cs",
];

#[derive(Debug, Clone)]
pub(crate) struct Candidate {
    pub(crate) rel_path: String,
    pub(crate) name: String,
    pub(crate) path_terms: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Intent {
    BuildTest,
    Cache,
    Debug,
    FirstMoves,
    GuiAutomation,
    Implement,
    Review,
    Research,
    General,
}

impl Intent {
    fn as_str(self) -> &'static str {
        match self {
            Self::BuildTest => "build_test",
            Self::Cache => "cache",
            Self::Debug => "debug",
            Self::FirstMoves => "first_moves",
            Self::GuiAutomation => "gui_automation",
            Self::Implement => "implement",
            Self::Review => "review",
            Self::Research => "research",
            Self::General => "general",
        }
    }
}

pub fn is_whole_repo_exploration_prompt(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    if lower.trim_start().starts_with('/') {
        return false;
    }

    let asks_to_explore = [
        "study",
        "explore",
        "inspect",
        "investigate",
        "understand",
        "analyze",
        "analyse",
        "review",
        "audit",
        "explain",
        "describe",
        "map",
        "read through",
        "look through",
        "look at",
    ]
    .iter()
    .any(|term| lower.contains(term));
    if !asks_to_explore {
        return false;
    }

    [
        "repo",
        "repository",
        "codebase",
        "project",
        "workspace",
        "whole tree",
        "entire tree",
        "whole code",
        "entire code",
        "all files",
        "broad",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

pub async fn predict(request: PredictRequest<'_>) -> Result<FirstMovesBundle> {
    let config = request.config.clone();
    let project_root = resolve_repo_root(request.project_root);
    let storage = storage_for(&project_root, request.codex_home);
    let repo_key = storage.repo_key.clone();
    let intent = detect_intent(request.prompt);
    let prompt_terms = tokenize(request.prompt);
    let already_loaded = already_loaded_paths(&request.already_loaded_paths);
    let learning = load_learning(&storage).await;
    let memory_hints =
        project_problem_memory_hints(request.codex_home, &project_root, &prompt_terms);
    let mut notes = vec![format!(
        "native predictor scanned repo namespace {repo_key}"
    )];
    if storage.repo_db_exists {
        notes.push("repo .first_moves.db detected and read for learning".to_string());
    }
    if !memory_hints.is_empty() {
        notes.push(format!(
            "project/problem memory hint fragments: {}",
            memory_hints.len()
        ));
    }

    let candidates = scan_candidates(&project_root, &config);
    notes.push(format!("candidate files scanned: {}", candidates.len()));
    let mut moves = score_candidates(
        request.prompt,
        &prompt_terms,
        &already_loaded,
        &learning,
        &memory_hints,
        intent,
        candidates.clone(),
    );
    moves.extend(search_hints(request.prompt, intent));
    moves.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    moves.truncate(config.max_candidates);
    add_excerpts(&mut moves, &project_root, &config);

    let confidence = moves.first().map_or(0.0, |entry| entry.confidence);
    let bundle = FirstMovesBundle {
        confidence,
        intent: intent.as_str().to_string(),
        project_root,
        repo_key,
        storage,
        moves,
        notes,
    };

    if request.record_prediction
        && let Err(err) = record_prediction(
            &bundle.storage,
            request.session_id,
            request.prompt,
            &bundle.intent,
            bundle.confidence,
            &bundle.moves,
        )
        .await
    {
        tracing::warn!("failed to record first-moves prediction: {err}");
    }
    if request.record_prediction
        && let Err(err) = record_shadow_prediction(ShadowPredictInput {
            codex_home: request.codex_home,
            prompt: request.prompt,
            session_id: request.session_id,
            config: &config,
            storage: &bundle.storage,
            candidates: &candidates,
            prompt_terms: &prompt_terms,
            already_loaded: &already_loaded,
            memory_hints: &memory_hints,
            intent,
            native_moves: &bundle.moves,
        })
    {
        tracing::debug!("failed to record first-moves shadow prediction: {err}");
    }

    Ok(bundle)
}

fn scan_candidates(project_root: &Path, config: &FirstMovesConfig) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for entry in WalkDir::new(project_root)
        .max_depth(config.max_scan_depth)
        .into_iter()
        .filter_entry(keep_entry)
        .filter_map(std::result::Result::ok)
    {
        if candidates.len() >= config.max_scan_files {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Ok(rel_path) = path.strip_prefix(project_root) else {
            continue;
        };
        let rel_path = rel_path.to_string_lossy().replace('\\', "/");
        if !include_file(&rel_path) {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let path_terms = tokenize(&rel_path);
        candidates.push(Candidate {
            rel_path,
            name,
            path_terms,
        });
    }
    candidates
}

fn keep_entry(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }
    let name = entry
        .file_name()
        .to_str()
        .unwrap_or_default()
        .to_ascii_lowercase();
    !SKIP_DIRS.contains(&name.as_str())
}

fn include_file(rel_path: &str) -> bool {
    let lower = rel_path.to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or_default();
    if BASELINE_FILES.contains(&name) {
        return true;
    }
    if name.ends_with(".lock")
        || name.ends_with(".min.js")
        || name.ends_with(".generated.rs")
        || name.ends_with(".generated.ts")
        || name.ends_with(".generated.js")
        || name.ends_with(".d.ts")
    {
        return false;
    }
    Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| SOURCE_EXTENSIONS.contains(&ext))
}

fn score_candidates(
    prompt: &str,
    prompt_terms: &HashSet<String>,
    already_loaded: &HashSet<String>,
    learning: &HashMap<String, PathLearning>,
    memory_hints: &HashSet<String>,
    intent: Intent,
    candidates: Vec<Candidate>,
) -> Vec<FirstMove> {
    let prompt_lower = prompt.to_ascii_lowercase().replace('\\', "/");
    let mut moves = Vec::new();
    for candidate in candidates {
        let rel_lower = candidate.rel_path.to_ascii_lowercase();
        if already_loaded.contains(&rel_lower) {
            continue;
        }

        let mut score = baseline_score(&candidate);
        let mut reasons = Vec::new();
        if prompt_lower.contains(&rel_lower) {
            score += 0.75;
            reasons.push("explicit path mention");
        } else if prompt_lower.contains(&candidate.name) && candidate.name.len() >= 5 {
            score += 0.35;
            reasons.push("explicit filename mention");
        }

        let overlap = prompt_terms.intersection(&candidate.path_terms).count();
        if overlap > 0 {
            score += (overlap as f64 * 0.06).min(0.24);
            reasons.push("prompt/path term overlap");
        }

        if let Some((boost, reason)) = intent_boost(intent, &rel_lower) {
            score += boost;
            reasons.push(reason);
        }

        if memory_hints
            .iter()
            .any(|hint| hint.len() >= 3 && rel_lower.contains(hint))
        {
            score += 0.18;
            reasons.push("project/problem memory routing hint");
        }

        if let Some(learning) = learning.get(&normalize_path_text(&candidate.rel_path)) {
            if learning.hits > 0 {
                score += ((learning.hits as f64) * 0.08).min(0.20);
                reasons.push("confirmed local hit history");
            }
            let misses = learning.observed.saturating_sub(learning.hits);
            if misses > 0 {
                score -= ((misses as f64) * 0.02).min(0.12);
            }
        }

        let logic_input = LogicInput {
            intent,
            prompt_lower: &prompt_lower,
            prompt_terms,
            memory_hints,
        };
        let logic_assessment = assess_candidate(&candidate, &logic_input);
        if !logic_assessment.is_empty() {
            score += logic_assessment.score_delta;
            reasons.extend(logic_assessment.reasons);
        }

        if score < 0.25 {
            continue;
        }

        let confidence = score.clamp(0.0, 0.99);
        moves.push(FirstMove {
            kind: FirstMoveKind::Read,
            confidence,
            reason: if reasons.is_empty() {
                "repo structure baseline".to_string()
            } else {
                reasons.join("; ")
            },
            source_layer: source_layer(confidence, reasons.as_slice()).to_string(),
            path: Some(PathBuf::from(candidate.rel_path)),
            query: None,
            excerpt: None,
        });
    }
    moves
}

fn project_problem_memory_hints(
    codex_home: &Path,
    project_root: &Path,
    prompt_terms: &HashSet<String>,
) -> HashSet<String> {
    if prompt_terms.is_empty() {
        return HashSet::new();
    }

    let memory_root = codex_home.join("memories");
    let project_root = normalize_path_text(&project_root.display().to_string());
    let mut hints = HashSet::new();
    for file_name in ["project_index.jsonl", "problem_index.jsonl"] {
        let Ok(contents) = fs::read_to_string(memory_root.join(file_name)) else {
            continue;
        };
        for line in contents.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let line_text = value.to_string().to_ascii_lowercase();
            if !prompt_terms.iter().any(|term| line_text.contains(term)) {
                continue;
            }
            let cwd = value
                .get("cwd")
                .and_then(Value::as_str)
                .map(normalize_path_text)
                .unwrap_or_default();
            if !memory_entry_matches_project_scope(cwd.as_str(), project_root.as_str()) {
                continue;
            }
            collect_memory_hint_array(&value, "edit_surfaces", &mut hints);
            collect_memory_hint_array(&value, "routing_keywords", &mut hints);
        }
    }
    hints
}

fn collect_memory_hint_array(value: &Value, key: &str, hints: &mut HashSet<String>) {
    let Some(items) = value
        .get("metadata")
        .and_then(|metadata| metadata.get(key))
        .and_then(Value::as_array)
    else {
        return;
    };

    for item in items.iter().filter_map(Value::as_str) {
        let normalized = normalize_path_text(item);
        if normalized.len() >= 3 {
            hints.insert(normalized);
        }
    }
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

fn baseline_score(candidate: &Candidate) -> f64 {
    let rel = candidate.rel_path.to_ascii_lowercase();
    let name = rel.rsplit('/').next().unwrap_or_default();
    if name == "agents.md" {
        0.05
    } else if BASELINE_FILES.contains(&name) {
        0.18
    } else if rel.contains("/src/") || rel.starts_with("src/") {
        0.12
    } else if rel.contains("/tests/") || rel.starts_with("tests/") {
        0.10
    } else {
        0.04
    }
}

pub(crate) fn repo_structure_score(candidate: &Candidate) -> f64 {
    baseline_score(candidate)
}

fn intent_boost(intent: Intent, rel: &str) -> Option<(f64, &'static str)> {
    match intent {
        Intent::FirstMoves => {
            if rel.contains("first-moves")
                || rel.contains("first_moves")
                || rel.contains("session/turn.rs")
                || rel.contains("tool_registry_plan")
                || rel.contains("mcp_tool_exposure")
            {
                Some((0.48, "first-moves implementation intent"))
            } else if rel.contains("config") {
                Some((0.20, "first-moves config intent"))
            } else {
                None
            }
        }
        Intent::GuiAutomation => {
            if rel.contains("desktop_automation") || rel.contains("desktop-automation") {
                Some((0.45, "desktop automation intent"))
            } else if rel.contains("tool_registry") || rel.contains("handler") {
                Some((0.20, "automation tool wiring intent"))
            } else {
                None
            }
        }
        Intent::Cache => {
            if rel.contains("cache") || rel.contains("operation_cache") {
                Some((0.42, "cache intent"))
            } else {
                None
            }
        }
        Intent::BuildTest => {
            if rel.ends_with("cargo.toml")
                || rel.ends_with("build.bazel")
                || rel.contains("scripts/build")
                || rel.contains("tests")
            {
                Some((0.34, "build/test intent"))
            } else {
                None
            }
        }
        Intent::Review => {
            if rel.contains("review") || rel.contains("tests") {
                Some((0.26, "review intent"))
            } else {
                None
            }
        }
        Intent::Debug => {
            if rel.contains("tests") || rel.contains("handler") || rel.contains("registry") {
                Some((0.22, "debug intent"))
            } else {
                None
            }
        }
        Intent::Research => {
            if rel.starts_with("docs/") || rel.ends_with("readme.md") {
                Some((0.26, "research/docs intent"))
            } else {
                None
            }
        }
        Intent::Implement => {
            if rel.contains("src/") || rel.contains("tool") || rel.contains("config") {
                Some((0.14, "implementation intent"))
            } else {
                None
            }
        }
        Intent::General => None,
    }
}

fn source_layer(confidence: f64, reasons: &[&str]) -> &'static str {
    if reasons.iter().any(|reason| reason.contains("explicit")) {
        "explicit_path"
    } else if reasons
        .iter()
        .any(|reason| reason.contains("confirmed local hit"))
    {
        "repo_learning"
    } else if reasons
        .iter()
        .any(|reason| reason.starts_with("logic gate"))
    {
        "logic_gate"
    } else if reasons
        .iter()
        .any(|reason| reason.starts_with("probabilistic evidence"))
    {
        "probabilistic_evidence"
    } else if confidence >= 0.55 {
        "intent"
    } else {
        "repo_structure"
    }
}

fn search_hints(prompt: &str, intent: Intent) -> Vec<FirstMove> {
    let mut hints = Vec::new();
    match intent {
        Intent::FirstMoves => {
            hints.push(search(
                "first_moves|first-moves|FirstMoves",
                0.72,
                "first-moves symbols",
            ));
            hints.push(search(
                "run_user_prompt_submit_hooks|record_additional_contexts",
                0.68,
                "first-turn context injection",
            ));
            hints.push(search(
                "ToolHandlerKind|build_tool_registry_plan",
                0.64,
                "built-in tool wiring",
            ));
        }
        Intent::GuiAutomation => {
            hints.push(search(
                "desktop_automation|automation_harness_detect|dab_",
                0.72,
                "automation tool symbols",
            ));
        }
        Intent::Cache => {
            hints.push(search(
                "operation_cache|cache namespace|CODEX_PROJECT_CACHE_NAMESPACE",
                0.70,
                "cache symbols",
            ));
        }
        Intent::BuildTest => {
            hints.push(search(
                "build-local-codex|FastRelease|LowMemRelease",
                0.66,
                "build lane",
            ));
        }
        Intent::Debug | Intent::Implement | Intent::Review | Intent::Research | Intent::General => {
        }
    }
    if prompt.to_ascii_lowercase().contains("config") {
        hints.push(search(
            "ConfigToml|ConfigProfile|write-config-schema",
            0.62,
            "config wiring",
        ));
    }
    hints
}

fn search(query: &str, confidence: f64, reason: &str) -> FirstMove {
    FirstMove {
        kind: FirstMoveKind::Search,
        confidence,
        reason: reason.to_string(),
        source_layer: "intent_search".to_string(),
        path: None,
        query: Some(query.to_string()),
        excerpt: None,
    }
}

fn add_excerpts(moves: &mut [FirstMove], project_root: &Path, config: &FirstMovesConfig) {
    if !config.prewarm_enabled() {
        return;
    }
    let mut warmed = 0usize;
    for entry in moves.iter_mut() {
        if warmed >= config.max_prewarm_files || entry.confidence < config.min_prewarm_score {
            break;
        }
        if !matches!(entry.kind, FirstMoveKind::Read) {
            continue;
        }
        let Some(path) = entry.path.as_ref() else {
            continue;
        };
        let abs_path = project_root.join(path);
        let Ok(mut bytes) = fs::read(abs_path) else {
            continue;
        };
        bytes.truncate(config.max_read_bytes);
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        entry.excerpt = Some(trim_excerpt(&text));
        warmed += 1;
    }
}

fn trim_excerpt(text: &str) -> String {
    text.lines()
        .take(80)
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(4_000)
        .collect()
}

fn detect_intent(prompt: &str) -> Intent {
    let lower = prompt.to_ascii_lowercase();
    if lower.contains("first_moves")
        || lower.contains("first moves")
        || lower.contains("first-moves")
    {
        Intent::FirstMoves
    } else if lower.contains("dab")
        || lower.contains("gui")
        || lower.contains("automation harness")
        || lower.contains("desktop automation")
    {
        Intent::GuiAutomation
    } else if lower.contains("cache") {
        Intent::Cache
    } else if lower.contains("build") || lower.contains("test") || lower.contains("cargo") {
        Intent::BuildTest
    } else if lower.contains("review") {
        Intent::Review
    } else if lower.contains("bug") || lower.contains("fix") || lower.contains("debug") {
        Intent::Debug
    } else if lower.contains("research") || lower.contains("docs") {
        Intent::Research
    } else if lower.contains("implement") || lower.contains("add ") || lower.contains("change ") {
        Intent::Implement
    } else {
        Intent::General
    }
}

fn tokenize(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .map(str::to_ascii_lowercase)
        .filter(|term| term.len() >= 3)
        .collect()
}

fn already_loaded_paths(paths: &[PathBuf]) -> HashSet<String> {
    let mut loaded = HashSet::from(["agents.md".to_string()]);
    loaded.extend(
        paths
            .iter()
            .map(|path| normalize_path_text(path.to_string_lossy().as_ref())),
    );
    loaded
}

#[allow(dead_code)]
fn repo_fingerprint(project_root: &Path) -> String {
    short_hash(project_root.to_string_lossy().as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format_first_moves_context;
    use pretty_assertions::assert_eq;

    #[test]
    fn whole_repo_exploration_prompt_detection_is_specific() {
        assert!(is_whole_repo_exploration_prompt("study the repo"));
        assert!(is_whole_repo_exploration_prompt(
            "please review this codebase"
        ));
        assert!(is_whole_repo_exploration_prompt(
            "explore the whole project"
        ));
        assert!(!is_whole_repo_exploration_prompt("study src/foo.cpp"));
        assert!(!is_whole_repo_exploration_prompt("fix the build"));
        assert!(!is_whole_repo_exploration_prompt("/review"));
    }

    #[test]
    fn project_problem_memory_scope_uses_path_boundaries() {
        assert!(memory_entry_matches_project_scope(
            "c:/users/oleh/documents/github/open_ai/codex/codex-rs",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
        assert!(memory_entry_matches_project_scope(
            "c:/users/oleh/documents/github/open_ai",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
        assert!(memory_entry_matches_project_scope(
            "",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
        assert!(!memory_entry_matches_project_scope(
            "c:/users/oleh/documents/github/open_ai/codex-old",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
    }

    #[tokio::test]
    async fn explicit_path_mentions_rank_first_and_agents_is_skipped() {
        let temp = tempfile::tempdir().expect("temp dir");
        let project = temp.path().join("repo");
        let codex_home = temp.path().join("codex-home");
        std::fs::create_dir_all(project.join("src")).expect("src dir");
        std::fs::write(project.join("AGENTS.md"), "repo instructions").expect("agents");
        std::fs::write(project.join("src/lib.rs"), "pub fn target() {}\n").expect("lib");
        std::fs::write(project.join("README.md"), "readme").expect("readme");

        let bundle = predict(PredictRequest {
            project_root: &project,
            codex_home: &codex_home,
            prompt: "inspect src/lib.rs before changing behavior",
            session_id: Some("session"),
            config: FirstMovesConfig::default(),
            already_loaded_paths: vec![PathBuf::from("AGENTS.md")],
            record_prediction: false,
        })
        .await
        .expect("prediction");

        assert_eq!(bundle.moves[0].path, Some(PathBuf::from("src/lib.rs")));
        let context =
            format_first_moves_context(&bundle, &FirstMovesConfig::default()).expect("context");
        assert!(context.contains("src/lib.rs"));
        assert!(!context.contains("AGENTS.md"));
    }

    #[tokio::test]
    async fn prediction_from_subdirectory_uses_enclosing_repo_root() {
        let temp = tempfile::tempdir().expect("temp dir");
        let project = temp.path().join("repo");
        let subdir = project.join("src");
        let codex_home = temp.path().join("codex-home");
        std::fs::create_dir_all(&subdir).expect("src dir");
        std::fs::create_dir(project.join(".git")).expect("git marker");
        std::fs::write(project.join("Cargo.toml"), "[package]\nname = \"repo\"\n")
            .expect("cargo toml");

        let bundle = predict(PredictRequest {
            project_root: &subdir,
            codex_home: &codex_home,
            prompt: "check Cargo.toml configuration",
            session_id: Some("session"),
            config: FirstMovesConfig::default(),
            already_loaded_paths: Vec::new(),
            record_prediction: false,
        })
        .await
        .expect("prediction");

        assert_eq!(bundle.project_root, project);
        assert!(bundle.repo_key.starts_with("repo-"));
        assert!(
            bundle
                .moves
                .iter()
                .any(|entry| entry.path == Some(PathBuf::from("Cargo.toml")))
        );
    }

    #[tokio::test]
    async fn project_problem_memory_hints_boost_matching_surface() {
        let temp = tempfile::tempdir().expect("temp dir");
        let project = temp.path().join("repo");
        let codex_home = temp.path().join("codex-home");
        std::fs::create_dir_all(project.join("codex-rs/core/src/session")).expect("session dir");
        std::fs::create_dir_all(project.join("codex-rs/core/src/tools")).expect("tools dir");
        std::fs::create_dir_all(codex_home.join("memories")).expect("memories dir");
        std::fs::write(project.join("Cargo.toml"), "[package]\nname = \"repo\"\n")
            .expect("cargo toml");
        std::fs::write(
            project.join("codex-rs/core/src/session/first_moves.rs"),
            "pub fn first_moves_memory_route() {}\n",
        )
        .expect("first moves");
        std::fs::write(
            project.join("codex-rs/core/src/tools/spec.rs"),
            "pub fn tool_spec() {}\n",
        )
        .expect("spec");

        let index_line = serde_json::json!({
            "cwd": project.display().to_string(),
            "metadata": {
                "problem_families": ["first moves routing"],
                "edit_surfaces": ["codex-rs/core/src/session"],
                "routing_keywords": ["first_moves"]
            }
        });
        std::fs::write(
            codex_home.join("memories/project_index.jsonl"),
            format!("{index_line}\n"),
        )
        .expect("project index");

        let bundle = predict(PredictRequest {
            project_root: &project,
            codex_home: &codex_home,
            prompt: "fix first moves routing",
            session_id: Some("session"),
            config: FirstMovesConfig::default(),
            already_loaded_paths: Vec::new(),
            record_prediction: false,
        })
        .await
        .expect("prediction");

        let first = bundle.moves.first().expect("at least one move");
        assert_eq!(
            first.path,
            Some(PathBuf::from("codex-rs/core/src/session/first_moves.rs"))
        );
        assert!(first.reason.contains("project/problem memory routing hint"));
    }

    #[tokio::test]
    async fn logic_overlay_boosts_source_over_shallow_docs_for_first_moves_work() {
        let temp = tempfile::tempdir().expect("temp dir");
        let project = temp.path().join("repo");
        let codex_home = temp.path().join("codex-home");
        std::fs::create_dir_all(project.join("codex-rs/first-moves/src")).expect("src dir");
        std::fs::create_dir_all(project.join("docs")).expect("docs dir");
        std::fs::write(project.join("Cargo.toml"), "[package]\nname = \"repo\"\n")
            .expect("cargo toml");
        std::fs::write(
            project.join("codex-rs/first-moves/src/predict.rs"),
            "pub fn predict() {}\n",
        )
        .expect("predict source");
        std::fs::write(
            project.join("docs/first-moves-logic-overlay.md"),
            "# First moves logic overlay\n",
        )
        .expect("docs");

        let bundle = predict(PredictRequest {
            project_root: &project,
            codex_home: &codex_home,
            prompt: "implement first moves logic overlay",
            session_id: Some("session"),
            config: FirstMovesConfig::default(),
            already_loaded_paths: Vec::new(),
            record_prediction: false,
        })
        .await
        .expect("prediction");

        let first = bundle.moves.first().expect("at least one move");
        assert_eq!(
            first.path,
            Some(PathBuf::from("codex-rs/first-moves/src/predict.rs"))
        );
        assert_eq!(first.source_layer, "probabilistic_evidence");
        assert!(
            first
                .reason
                .contains("probabilistic evidence: intent/path fit")
        );
    }

    #[tokio::test]
    async fn record_prediction_writes_shadow_without_changing_bundle() {
        let temp = tempfile::tempdir().expect("temp dir");
        let project = temp.path().join("repo");
        let codex_home = temp.path().join("codex-home");
        std::fs::create_dir_all(project.join("src")).expect("src dir");
        std::fs::write(project.join("src/lib.rs"), "pub fn target() {}\n").expect("lib");

        let bundle = predict(PredictRequest {
            project_root: &project,
            codex_home: &codex_home,
            prompt: "inspect src/lib.rs",
            session_id: Some("session"),
            config: FirstMovesConfig::default(),
            already_loaded_paths: Vec::new(),
            record_prediction: true,
        })
        .await
        .expect("prediction");

        assert_eq!(bundle.moves[0].path, Some(PathBuf::from("src/lib.rs")));
        let shadow_path = codex_home
            .join("first-moves-shadow")
            .join(&bundle.repo_key)
            .join("shadow.jsonl");
        let shadow = std::fs::read_to_string(shadow_path).expect("shadow jsonl");
        assert!(shadow.contains("\"type\":\"first_moves_shadow\""));
        assert!(shadow.contains("\"path_lexical\""));
        assert!(shadow.contains("\"logic_evidence\""));
        assert!(shadow.contains("\"content_seeded_component_merge\""));
    }
}
