use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use walkdir::DirEntry;
use walkdir::WalkDir;

// Graphify/Aider-style repo-map narrowing, promoted after
// context-reducer-lab's 2026-05-08 real Codex canaries showed better routing
// for fresh root turns and spawn_agent prompts.
const DEFAULT_PATH_BUDGET: usize = 16;
const MAX_CONTENT_SCORE_FILES: usize = 256;
const MAX_READ_BYTES: usize = 32_000;

#[derive(Debug, Clone)]
pub struct ContextPackRequest<'a> {
    pub project_root: &'a Path,
    pub prompt: &'a str,
    pub path_budget: usize,
}

impl<'a> ContextPackRequest<'a> {
    pub fn new(project_root: &'a Path, prompt: &'a str) -> Self {
        Self {
            project_root,
            prompt,
            path_budget: DEFAULT_PATH_BUDGET,
        }
    }
}

pub fn render_graphify_scout_pack(request: &ContextPackRequest<'_>) -> Option<String> {
    if !should_render_context_pack(request.prompt) {
        return None;
    }
    let mut files = repo_inventory(request.project_root, request.prompt);
    if files.is_empty() {
        return None;
    }
    let terms = prompt_terms(request.prompt);
    for file in &mut files {
        file.score += path_prompt_score(&file.path, &terms);
        file.score += operational_path_boost(request.prompt, &file.path);
    }
    files.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    for file in files.iter_mut().take(MAX_CONTENT_SCORE_FILES) {
        file.score += content_prompt_score(request.project_root, &file.path, &terms);
    }
    files.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    let selected = files
        .into_iter()
        .filter(|file| file.score > 0)
        .take(request.path_budget.max(1))
        .map(|file| file.path)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return None;
    }
    Some(render_pack(&selected))
}

pub fn prepend_context_pack_to_message(
    project_root: &Path,
    message: &str,
    path_budget: usize,
) -> String {
    if has_context_pack(message) {
        return message.to_string();
    }
    let request = ContextPackRequest {
        project_root,
        prompt: message,
        path_budget,
    };
    let Some(pack) = render_graphify_scout_pack(&request) else {
        return message.to_string();
    };
    format!("{pack}\n\n{message}")
}

pub fn has_context_pack(message: &str) -> bool {
    message.to_ascii_lowercase().contains("<context_pack")
}

pub fn has_context_pack_or_scout(message: &str) -> bool {
    has_context_pack(message) || has_scout_context(message)
}

pub fn has_scout_context(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("scout_evidence:")
        || lower.contains("first_moves_evidence:")
        || lower.contains("routing_evidence:")
        || lower.contains("context_scout_evidence:")
        || lower.contains("first_moves_predict")
        || lower.contains("repo_context_scout")
}

pub fn is_explicit_repo_routing_prompt(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    contains_any(
        &lower,
        &[
            "map this repo",
            "map the repo",
            "map this codebase",
            "explore this repo",
            "inspect this repo",
            "inspect the repo",
            "inspect the codebase",
            "where is ",
            "where should ",
            "which files",
            "what files",
            "repo exploration",
            "codebase exploration",
        ],
    ) && !looks_exact_file_only(&lower)
}

fn looks_exact_file_only(lower: &str) -> bool {
    let has_file_path = [
        ".rs", ".md", ".toml", ".json", ".yaml", ".yml", ".ps1", ".ts", ".tsx", ".js", ".py",
        ".cpp", ".h", ".hpp",
    ]
    .iter()
    .any(|ext| lower.contains(ext));
    let asks_for_repo = contains_any(lower, &["repo", "codebase", "project", "which files"]);
    has_file_path && !asks_for_repo
}

fn contains_any(section: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| section.contains(term))
}

fn render_pack(paths: &[String]) -> String {
    let mut lines = vec![
        "<context_pack variant=\"graphify_scout_pack\" source=\"context-reducer-lab-2026-05-08-canary\">".to_string(),
        "SCOUT_EVIDENCE:".to_string(),
    ];
    for path in paths {
        lines.push(format!("- {path}"));
    }
    lines.push("FIRST_READS: read the listed paths first when they fit the task.".to_string());
    lines.push(
        "FRESHNESS: derived from current repo scan; read exact files before editing.".to_string(),
    );
    lines.push("VERIFICATION: treat these paths as first reads, not answers.".to_string());
    lines.push("</context_pack>".to_string());
    lines.join("\n")
}

#[derive(Debug, Clone)]
struct FileCandidate {
    path: String,
    score: i64,
}

fn should_render_context_pack(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    let taskish = [
        "implement",
        "fix",
        "review",
        "debug",
        "test",
        "identify",
        "find",
        "inspect",
        "where",
        "which files",
        "what files",
        "map",
        "explore",
        "codebase",
        "repo",
        "project",
        "integrate",
        "bootstrap",
        "installer",
        "spawn_agent",
        "fresh turn",
        "context pack",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    taskish && prompt_terms(prompt).len() >= 2
}

fn repo_inventory(root: &Path, prompt: &str) -> Vec<FileCandidate> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| should_visit_entry(root, entry, prompt))
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let rel = entry.path().strip_prefix(root).ok()?;
            let path = normalize_slashes(rel.display().to_string());
            (!is_generated_path(&path)
                && !is_low_value_pack_path(&path, prompt)
                && is_candidate_file(&path))
            .then_some(FileCandidate { path, score: 0 })
        })
        .collect()
}

fn should_visit_entry(root: &Path, entry: &DirEntry, prompt: &str) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return true;
    }
    let Ok(rel) = entry.path().strip_prefix(root) else {
        return false;
    };
    let path = normalize_slashes(rel.display().to_string());
    !is_generated_path(&path) && !is_low_value_pack_path(&path, prompt)
}

fn path_prompt_score(path: &str, terms: &BTreeSet<String>) -> i64 {
    let lower = path.to_ascii_lowercase();
    terms
        .iter()
        .filter(|term| lower.contains(term.as_str()))
        .count()
        .min(24) as i64
        * 25
}

fn content_prompt_score(root: &Path, path: &str, terms: &BTreeSet<String>) -> i64 {
    if terms.is_empty() {
        return 0;
    }
    let full_path = root.join(path);
    let Ok(bytes) = fs::read(full_path) else {
        return 0;
    };
    let len = bytes.len().min(MAX_READ_BYTES);
    let text = String::from_utf8_lossy(&bytes[..len]).to_ascii_lowercase();
    terms
        .iter()
        .filter(|term| text.contains(term.as_str()))
        .count()
        .min(24) as i64
        * 18
}

fn operational_path_boost(prompt: &str, path: &str) -> i64 {
    let prompt = prompt.to_ascii_lowercase();
    let path = path.to_ascii_lowercase();
    let mut boost = 0;
    if prompt.contains("context pack") || prompt.contains("context-pack") {
        if path.contains("context_pack") || path.contains("context-pack") {
            boost += 400;
        }
    }
    if prompt.contains("fresh root")
        || prompt.contains("fresh turn")
        || prompt.contains("root turn")
    {
        if path.ends_with("core/src/session/turn.rs") {
            boost += 500;
        }
        if path.ends_with("core/src/session/first_moves.rs") {
            boost += 300;
        }
    }
    if prompt.contains("spawn_agent") || prompt.contains("spawn agent") {
        if path.ends_with("core/src/tools/handlers/multi_agents_v2/spawn.rs") {
            boost += 500;
        }
        if path.ends_with("tools/src/agent_tool.rs") {
            boost += 350;
        }
    }
    if prompt.contains("bootstrap") || prompt.contains("installer") || prompt.contains("install") {
        if path.contains("install")
            || path.contains("bootstrap")
            || path.contains("setup")
            || path.contains("requirements")
            || path.ends_with(".bat")
            || path.ends_with(".cmd")
        {
            boost += 450;
        }
    }
    boost
}

fn prompt_terms(prompt: &str) -> BTreeSet<String> {
    prompt
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .map(str::trim)
        .filter(|token| token.len() >= 3)
        .map(str::to_ascii_lowercase)
        .collect()
}

fn normalize_slashes(path: impl AsRef<str>) -> String {
    path.as_ref().replace('\\', "/")
}

fn is_candidate_file(path: &str) -> bool {
    matches!(
        language_for_path(path),
        "rust"
            | "powershell"
            | "cpp"
            | "python"
            | "typescript"
            | "batch"
            | "toml"
            | "markdown"
            | "json"
    )
}

fn language_for_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".rs") {
        "rust"
    } else if lower.ends_with(".ps1") || lower.ends_with(".psm1") || lower.ends_with(".psd1") {
        "powershell"
    } else if lower.ends_with(".c")
        || lower.ends_with(".cc")
        || lower.ends_with(".cpp")
        || lower.ends_with(".cxx")
        || lower.ends_with(".h")
        || lower.ends_with(".hpp")
        || lower.ends_with(".hh")
        || lower.ends_with(".hxx")
    {
        "cpp"
    } else if lower.ends_with(".py") {
        "python"
    } else if lower.ends_with(".ts") || lower.ends_with(".tsx") || lower.ends_with(".js") {
        "typescript"
    } else if lower.ends_with(".bat") || lower.ends_with(".cmd") {
        "batch"
    } else if lower.ends_with(".toml") {
        "toml"
    } else if lower.ends_with(".md") {
        "markdown"
    } else if lower.ends_with(".json") {
        "json"
    } else {
        "unknown"
    }
}

fn is_generated_path(path: &str) -> bool {
    let normalized = normalize_slashes(path).to_ascii_lowercase();
    let parts = normalized.split('/').collect::<Vec<_>>();
    parts.iter().any(|part| {
        matches!(
            *part,
            ".git"
                | ".cache"
                | ".gsd"
                | ".gradle"
                | ".idea"
                | ".next"
                | ".turbo"
                | ".venv"
                | "__pycache__"
                | "_deps"
                | "bazel-bin"
                | "bazel-out"
                | "bazel-testlogs"
                | "build"
                | "build_standalone"
                | "dist"
                | "logs"
                | "node_modules"
                | "out"
                | "target"
        ) || part.starts_with("cmake-build")
    })
}

fn is_low_value_pack_path(path: &str, prompt: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    if lower == "reports" || lower.starts_with("reports/") {
        return !prompt_explicitly_mentions_path(prompt, path);
    }
    lower.contains("/current_scout_cache/")
        || lower.contains("/op-compare-")
        || lower.starts_with("docs/_generated")
}

fn prompt_explicitly_mentions_path(prompt: &str, path: &str) -> bool {
    let prompt = normalize_slashes(prompt).to_ascii_lowercase();
    let path = normalize_slashes(path).to_ascii_lowercase();
    prompt.contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn preserves_existing_context_pack_message() {
        let message =
            "<context_pack>\nSCOUT_EVIDENCE:\n- src/lib.rs\n</context_pack>\n\nDo the task.";
        assert_eq!(
            prepend_context_pack_to_message(Path::new("."), message, 16),
            message
        );
    }

    #[test]
    fn prepends_pack_when_prompt_already_has_first_reads() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src/session")).expect("mkdir");
        fs::write(
            temp.path().join("src/session/turn.rs"),
            "spawn_agent context pack FIRST_READS",
        )
        .expect("write");
        let message = "CONTEXT_AREA: src/session\nFIRST_READS: src/session/turn.rs\nImplement context pack routing.";
        let packed = prepend_context_pack_to_message(temp.path(), message, 16);

        assert!(packed.starts_with("<context_pack"));
        assert!(packed.contains("FIRST_READS: read the listed paths first"));
        assert!(packed.ends_with(message));
    }

    #[test]
    fn renders_context_pack_for_prompt() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src/session")).expect("mkdir");
        fs::write(
            temp.path().join("src/session/turn.rs"),
            "fresh root turn prompt context spawn_agent",
        )
        .expect("write");
        let request = ContextPackRequest::new(
            temp.path(),
            "Identify fresh root turn context pack integration files",
        );
        let pack = render_graphify_scout_pack(&request).expect("pack");
        assert!(pack.contains("src/session/turn.rs"));
        assert!(pack.contains("<context_pack"));
        assert!(pack.contains("SCOUT_EVIDENCE:"));
        assert!(pack.contains("FIRST_READS:"));
        assert!(pack.contains("FRESHNESS:"));
        assert!(pack.contains("VERIFICATION:"));
    }

    #[test]
    fn limits_content_scoring_to_bounded_candidate_window() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir");
        for index in 0..300 {
            let body = if matches!(index, 255 | 299) {
                "raremarker behavior"
            } else {
                "ordinary behavior"
            };
            fs::write(temp.path().join(format!("src/file{index:03}.rs")), body).expect("write");
        }

        let request = ContextPackRequest::new(temp.path(), "Find raremarker behavior");
        let pack = render_graphify_scout_pack(&request).expect("pack");
        assert!(pack.contains("src/file255.rs"));
        assert!(!pack.contains("src/file299.rs"));
    }

    #[test]
    fn prunes_generated_directories_before_scoring() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir src");
        fs::create_dir_all(temp.path().join("target/generated")).expect("mkdir target");
        fs::write(temp.path().join("src/useful.rs"), "context pack routing").expect("write src");
        fs::write(
            temp.path().join("target/generated/raremarker.rs"),
            "raremarker context pack routing target",
        )
        .expect("write generated");

        let inventory = repo_inventory(temp.path(), "Find raremarker context pack routing");
        assert!(
            inventory
                .iter()
                .all(|file| !file.path.starts_with("target/"))
        );

        let request = ContextPackRequest::new(temp.path(), "Find raremarker context pack routing");
        let pack = render_graphify_scout_pack(&request).expect("pack");
        assert!(!pack.contains("target/"));
        assert!(pack.contains("src/useful.rs"));
    }

    #[test]
    fn preserves_explicitly_requested_report_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("reports")).expect("mkdir reports");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir src");
        fs::write(
            temp.path()
                .join("reports/codex-context-pack-canary-results-2026-05-08.md"),
            "context pack canary results",
        )
        .expect("write report");
        fs::write(temp.path().join("src/context_pack.rs"), "context pack").expect("write src");

        let request = ContextPackRequest::new(
            temp.path(),
            "please inspect reports/codex-context-pack-canary-results-2026-05-08.md",
        );
        let pack = render_graphify_scout_pack(&request).expect("pack");

        assert!(pack.contains("reports/codex-context-pack-canary-results-2026-05-08.md"));
    }

    #[test]
    fn prunes_unmentioned_report_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("reports")).expect("mkdir reports");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir src");
        fs::write(
            temp.path()
                .join("reports/codex-context-pack-canary-results-2026-05-08.md"),
            "context pack canary results",
        )
        .expect("write report");
        fs::write(temp.path().join("src/context_pack.rs"), "context pack").expect("write src");

        let request = ContextPackRequest::new(temp.path(), "please inspect context pack routing");
        let pack = render_graphify_scout_pack(&request).expect("pack");

        assert!(pack.contains("src/context_pack.rs"));
        assert!(!pack.contains("reports/"));
    }

    #[test]
    fn repo_routing_prompt_detection_skips_exact_file_prompts() {
        assert!(is_explicit_repo_routing_prompt(
            "where is spawn_agent implemented in this repo?"
        ));
        assert!(!is_explicit_repo_routing_prompt(
            "inspect codex-rs/core/src/session/first_moves.rs"
        ));
    }
}
