use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use walkdir::DirEntry;
use walkdir::WalkDir;

// Graphify/Aider-style repo-map narrowing, promoted after
// context-reducer-lab's 2026-05-08 real Codex canaries showed better routing
// for fresh root turns and spawn_agent prompts.
const DEFAULT_PATH_BUDGET: usize = 16;
const EXACT_MATCH_LIMIT: usize = 3;
const MAX_CONTENT_SCORE_FILES: usize = 256;
const MAX_READ_BYTES: usize = 32_000;
const MIN_EMIT_SCORE: i64 = 50;
const IDF_CAP: f64 = 4.0;
const ENTRY_POINT_RESERVED_SLOTS: usize = 3;

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
    let has_explicit_file_hint = prompt_has_candidate_extension(request.prompt);
    if (!has_explicit_file_hint && prompt_terms(request.prompt).len() < 2)
        || has_context_pack_or_scout(request.prompt)
    {
        return None;
    }
    if !has_explicit_file_hint && !should_render_context_pack(request.prompt) {
        return None;
    }
    let mut files = repo_inventory(request.project_root, request.prompt);
    if files.is_empty() {
        return None;
    }
    if has_explicit_file_hint {
        let explicit_paths = resolve_explicit_paths(
            &files,
            request.prompt,
            request.path_budget.clamp(1, EXACT_MATCH_LIMIT),
        );
        if !explicit_paths.is_empty() {
            return Some(render_exact_pack(&explicit_paths));
        }
    }
    if !should_render_context_pack(request.prompt) {
        return None;
    }
    let terms = prompt_terms(request.prompt);
    let idf = build_path_idf(&files);
    for file in &mut files {
        file.score += path_prompt_score(&file.path, &terms, &idf);
        file.score += operational_path_boost(request.prompt, &file.path);
    }
    files.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    for file in files.iter_mut().take(MAX_CONTENT_SCORE_FILES) {
        file.score += content_prompt_score(request.project_root, &file.path, &terms, &idf);
    }
    files.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    let mut selected = files
        .into_iter()
        .filter(|file| file.score >= MIN_EMIT_SCORE)
        .take(request.path_budget.max(1))
        .map(|file| file.path)
        .collect::<Vec<_>>();
    if is_generic_exploration_query(request.prompt) {
        selected = reserve_entry_points(request.project_root, selected, request.path_budget.max(1));
    }
    if selected.is_empty() {
        return None;
    }
    Some(render_scout_pack(&selected))
}

pub fn prepend_context_pack_to_message(
    project_root: &Path,
    message: &str,
    path_budget: usize,
) -> String {
    if has_context_pack_or_scout(message) {
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

pub fn render_entrypoint_hint(project_root: &Path, path_budget: usize) -> Option<String> {
    let paths = entry_point_paths(project_root, path_budget.clamp(1, DEFAULT_PATH_BUDGET));
    if paths.is_empty() {
        return None;
    }
    let mut lines = vec![
        "graphify_entrypoint_hint".to_string(),
        "canonical first-reads for broad repo exploration:".to_string(),
    ];
    for path in paths {
        let role = context_pack_path_role(path.as_str());
        lines.push(format!(
            "- {path} | role={role} | relation_reason={}",
            context_pack_relation_reason(role)
        ));
    }
    lines.push(
        "usage: read one or two only if the current broad search is not already enough."
            .to_string(),
    );
    Some(lines.join("\n"))
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
        || lower.contains("scout_hint:")
        || lower.contains("scout_hint (")
        || lower.contains("exact_match:")
        || lower.contains("exact_match (")
        || lower.contains("first_moves_evidence:")
        || lower.contains("routing_evidence:")
        || lower.contains("context_scout_evidence:")
        || lower.contains("agent_graph_scout")
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

const TRIGGER_WORDS: &[&str] = &[
    "implement",
    "fix",
    "review",
    "debug",
    "test",
    "tests",
    "verify",
    "validate",
    "ensure",
    "confirm",
    "convert",
    "refactor",
    "remove",
    "delete",
    "wire",
    "trace",
    "audit",
    "port",
    "rewrite",
    "rename",
    "move",
    "split",
    "merge",
    "fold",
    "extract",
    "introduce",
    "drop",
    "clean",
    "prune",
    "purge",
    "sync",
    "align",
    "mirror",
    "build",
    "compile",
    "rebuild",
    "integrate",
    "bootstrap",
    "installer",
    "identify",
    "find",
    "inspect",
    "explore",
    "show",
    "list",
    "check",
    "tell",
    "explain",
    "describe",
    "summarize",
    "summarise",
    "outline",
    "compare",
    "scan",
    "study",
    "analyze",
    "analyse",
    "diagnose",
    "investigate",
];

const CODE_NOUNS: &[&str] = &[
    "facade", "kernel", "pipeline", "workflow", "module", "function", "class", "method", "symbol",
    "hook", "codebase", "repo", "project",
];

const PHRASE_TRIGGERS: &[&str] = &[
    "where is",
    "where should",
    "where does",
    "where do ",
    "what is the",
    "what does",
    "what do ",
    "which files",
    "what files",
    "how does",
    "how do ",
    "how is ",
    "how are ",
    "why does",
    "why do ",
    "why is",
    "when does",
    "when do ",
    "spawn_agent",
    "fresh turn",
    "context pack",
    "context-pack",
    "look at",
    "walk through",
];

const GENERIC_EXPLORATION_PHRASES: &[&str] = &[
    "give me a tour",
    "give me an overview",
    "tour of",
    "tour this",
    "tour the",
    "first reads",
    "first read",
    "entry point",
    "entry points",
    "main components",
    "high-level structure",
    "high level structure",
    "what's in this",
    "what's in here",
    "whats in this",
    "explain this repo",
    "explore this repo",
    "map this repo",
    "map the repo",
    "map this codebase",
    "inspect this repo",
    "inspect the repo",
    "inspect the codebase",
    "repo exploration",
    "codebase exploration",
    "overview of",
];

const ENTRY_POINT_CANDIDATES: &[&str] = &[
    "CLAUDE.md",
    "AGENTS.md",
    "README.md",
    "README.rst",
    "README",
    "docs/repo_navigation_index.md",
    "docs/mcp_navigation_index.md",
    "docs/architecture.md",
    "docs/ARCHITECTURE.md",
    "ARCHITECTURE.md",
    "pyproject.toml",
    "package.json",
    "Cargo.toml",
    "CMakeLists.txt",
    "go.mod",
    "Makefile",
    "src/main.py",
    "src/main.cpp",
    "src/main.rs",
    "src/main.go",
    "src/lib.rs",
    "src/index.ts",
    "src/index.js",
    "main.py",
    "main.cpp",
    "main.rs",
    "main.go",
];

fn render_scout_pack(paths: &[String]) -> String {
    let mut lines = vec![
        "<context_pack variant=\"graphify_scout_pack\" source=\"context-reducer-lab-2026-05-08-canary\" mode=\"scout\">".to_string(),
        "SCOUT_HINT (candidate paths from a static term-matching heuristic):".to_string(),
    ];
    for path in paths {
        let role = context_pack_path_role(path);
        lines.push(format!(
            "- {path} | role={role} | relation_reason={}",
            context_pack_relation_reason(role)
        ));
    }
    lines.push(
        "USAGE: open these only if existing context is insufficient; do not pre-emptively read every listed path.".to_string(),
    );
    lines.push(
        "FRESHNESS: derived from current repo scan; read exact files before editing.".to_string(),
    );
    lines.push("VERIFICATION: treat these paths as orientation hints, not answers.".to_string());
    lines.push("</context_pack>".to_string());
    lines.join("\n")
}

fn render_exact_pack(paths: &[String]) -> String {
    let mut lines = vec![
        "<context_pack variant=\"graphify_scout_pack\" source=\"context-reducer-lab-2026-05-08-canary\" mode=\"exact\">".to_string(),
        "EXACT_MATCH (user named this path in the prompt):".to_string(),
    ];
    for path in paths {
        lines.push(format!("- {path}"));
    }
    lines.push(
        "USAGE: open the named path if needed; do not expand this into broad repo discovery."
            .to_string(),
    );
    lines.push(
        "FRESHNESS: derived from current repo scan; read exact files before editing.".to_string(),
    );
    lines.push("VERIFICATION: treat these paths as orientation hints, not answers.".to_string());
    lines.push("</context_pack>".to_string());
    lines.join("\n")
}

#[derive(Debug, Clone)]
struct FileCandidate {
    path: String,
    score: i64,
}

fn context_pack_path_role(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.contains("/test") || lower.contains("_test") || lower.ends_with("tests.rs") {
        "test"
    } else if lower.ends_with(".md") || lower.contains("/docs/") {
        "docs"
    } else if lower.ends_with(".toml")
        || lower.ends_with(".json")
        || lower.ends_with(".yml")
        || lower.ends_with(".yaml")
    {
        "config"
    } else if lower.ends_with("mod.rs") || lower.ends_with("lib.rs") || lower.ends_with("main.rs") {
        "entrypoint"
    } else if lower.ends_with(".h") || lower.ends_with(".hpp") || lower.contains("/protocol/") {
        "interface"
    } else {
        "implementation"
    }
}

fn context_pack_relation_reason(role: &str) -> &'static str {
    match role {
        "entrypoint" => "likely module or runtime entrypoint",
        "interface" => "likely API or type boundary",
        "implementation" => "likely behavior implementation",
        "test" => "likely verification surface",
        "config" => "likely build or runtime configuration",
        "docs" => "likely design or usage context",
        _ => "candidate path relation",
    }
}

fn resolve_explicit_paths(files: &[FileCandidate], prompt: &str, limit: usize) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    let mut selected = BTreeSet::new();
    for token in explicit_file_tokens(prompt) {
        if token.contains('/') {
            for file in files {
                if file.path.eq_ignore_ascii_case(&token) {
                    selected.insert(file.path.clone());
                    if selected.len() >= limit {
                        return selected.into_iter().collect();
                    }
                }
            }
        } else {
            let mut basename_matches = files
                .iter()
                .filter(|file| path_basename(&file.path).eq_ignore_ascii_case(&token))
                .map(|file| file.path.clone())
                .collect::<Vec<_>>();
            basename_matches.sort();
            for path in basename_matches {
                selected.insert(path);
                if selected.len() >= limit {
                    return selected.into_iter().collect();
                }
            }
        }
    }
    selected.into_iter().collect()
}

fn explicit_file_tokens(prompt: &str) -> Vec<String> {
    prompt_path_tokens(prompt)
        .into_iter()
        .filter(|token| has_candidate_extension_token(token))
        .collect()
}

fn prompt_path_tokens(prompt: &str) -> Vec<String> {
    prompt
        .split(|ch: char| {
            !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | '\\'))
        })
        .filter_map(clean_path_token)
        .collect()
}

fn clean_path_token(raw: &str) -> Option<String> {
    let mut token = normalize_slashes(raw.trim_matches(|ch: char| {
        matches!(
            ch,
            '"' | '\'' | '`' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | ':'
        )
    }));
    while let Some(stripped) = token.strip_prefix("./") {
        token = stripped.to_string();
    }
    token = token.trim_start_matches('/').to_string();
    if !has_candidate_extension_token(&token) {
        token = token.trim_end_matches('.').to_string();
    }
    (!token.is_empty() && token.len() >= 3).then_some(token)
}

fn has_candidate_extension_token(token: &str) -> bool {
    language_for_path(token) != "unknown"
}

fn path_basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn should_render_context_pack(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    let terms = prompt_terms(prompt);
    if terms.len() < 2 || has_context_pack_or_scout(prompt) {
        return false;
    }
    if is_docs_or_report_broad_task(&lower)
        || is_broad_common_search_prompt(&lower)
        || is_weak_single_symbol_prompt(prompt)
    {
        return false;
    }
    if is_explicit_repo_routing_prompt(prompt)
        || prompt_has_candidate_extension(prompt)
        || prompt_has_directory_path(prompt)
        || is_generic_exploration_query(prompt)
        || contains_any(&lower, PHRASE_TRIGGERS)
    {
        return true;
    }
    let words = prompt_words(prompt);
    let has_trigger_word = TRIGGER_WORDS.iter().any(|word| words.contains(*word));
    let has_code_noun = CODE_NOUNS.iter().any(|word| words.contains(*word));
    has_trigger_word && (terms.len() >= 3 || has_code_noun)
}

fn prompt_words(prompt: &str) -> BTreeSet<String> {
    prompt
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 2)
        .map(str::to_ascii_lowercase)
        .collect()
}

fn prompt_has_candidate_extension(prompt: &str) -> bool {
    !explicit_file_tokens(prompt).is_empty()
}

fn prompt_has_directory_path(prompt: &str) -> bool {
    prompt_path_tokens(prompt).into_iter().any(|token| {
        token.contains('/') && token.split('/').filter(|part| !part.is_empty()).count() >= 2
    })
}

fn is_docs_or_report_broad_task(lower: &str) -> bool {
    let words = prompt_words(lower);
    let mentions_docs = contains_any(
        lower,
        &[
            "docs",
            "documentation",
            "report",
            "reports",
            "markdown",
            "prose",
        ],
    );
    let task = contains_any(
        lower,
        &[
            "summarize",
            "summarise",
            "compare",
            "review",
            "explain",
            "describe",
            "report on",
            "report about",
        ],
    );
    let codeish_word = [
        "code",
        "codebase",
        "repo",
        "project",
        "implementation",
        "function",
        "symbol",
    ]
    .iter()
    .any(|word| words.contains(*word));
    let codeish_path = contains_any(lower, &["src/", "tests/", ".rs", ".py", ".cpp"]);
    let codeish = codeish_word || codeish_path;
    mentions_docs && task && !codeish
}

fn is_broad_common_search_prompt(lower: &str) -> bool {
    let broad = contains_any(
        lower,
        &[
            "search broadly",
            "broad search",
            "search the repo for",
            "search the project for",
        ],
    );
    let common = contains_any(
        lower,
        &[
            "config", "error", "test", "todo", "readme", "use", "mod", "file", "data",
        ],
    );
    broad && common
}

fn is_weak_single_symbol_prompt(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    if !contains_any(
        &lower,
        &["tell me about ", "what is ", "what does ", "describe "],
    ) {
        return false;
    }
    let tokens = prompt
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens.len() > 4 {
        return false;
    }
    let symbolish = tokens
        .iter()
        .filter(|token| token.contains('_') || token.chars().any(|ch| ch.is_ascii_uppercase()))
        .count();
    symbolish == 1
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

fn path_prompt_score(path: &str, terms: &BTreeSet<String>, idf: &BTreeMap<String, f64>) -> i64 {
    let lower = path.to_ascii_lowercase();
    let weight = terms
        .iter()
        .filter(|term| lower.contains(term.as_str()))
        .map(|term| idf.get(term).copied().unwrap_or(1.0))
        .sum::<f64>();
    (weight.min(24.0) * 25.0).round() as i64
}

fn content_prompt_score(
    root: &Path,
    path: &str,
    terms: &BTreeSet<String>,
    idf: &BTreeMap<String, f64>,
) -> i64 {
    if terms.is_empty() {
        return 0;
    }
    let full_path = root.join(path);
    let Ok(bytes) = fs::read(full_path) else {
        return 0;
    };
    let len = bytes.len().min(MAX_READ_BYTES);
    let text = String::from_utf8_lossy(&bytes[..len]).to_ascii_lowercase();
    let text = strip_code_comments(path, &text);
    let weight = terms
        .iter()
        .filter(|term| text.contains(term.as_str()))
        .map(|term| idf.get(term).copied().unwrap_or(1.0))
        .sum::<f64>();
    (weight.min(24.0) * 18.0).round() as i64
}

fn build_path_idf(files: &[FileCandidate]) -> BTreeMap<String, f64> {
    let mut df = BTreeMap::<String, usize>::new();
    for file in files {
        for component in path_components(&file.path) {
            *df.entry(component).or_default() += 1;
        }
    }
    let n = files.len() as f64;
    df.into_iter()
        .map(|(token, freq)| {
            let raw = ((n + 1.0) / (freq as f64 + 1.0)).ln() + 1.0;
            (token, raw.min(IDF_CAP))
        })
        .collect()
}

fn path_components(path: &str) -> BTreeSet<String> {
    path.to_ascii_lowercase()
        .split(['/', '\\', '.', '_', '-'])
        .filter(|part| part.len() >= 3)
        .map(str::to_string)
        .collect()
}

fn strip_code_comments(path: &str, text: &str) -> String {
    if !should_strip_comments(path) {
        return text.to_string();
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"/*") {
            index += 2;
            while index + 1 < bytes.len() && !bytes[index..].starts_with(b"*/") {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
        } else if bytes[index..].starts_with(b"\"\"\"") || bytes[index..].starts_with(b"'''") {
            let marker = &bytes[index..index + 3].to_vec();
            index += 3;
            while index + 2 < bytes.len() && &bytes[index..index + 3] != marker.as_slice() {
                index += 1;
            }
            index = (index + 3).min(bytes.len());
        } else if bytes[index..].starts_with(b"//") || bytes[index] == b'#' {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            if index < bytes.len() {
                out.push(bytes[index]);
                index += 1;
            }
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn should_strip_comments(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        ".py", ".pyi", ".js", ".jsx", ".mjs", ".ts", ".tsx", ".c", ".cc", ".cpp", ".cxx", ".h",
        ".hh", ".hpp", ".hxx", ".rs", ".go", ".java", ".kt", ".swift", ".cs", ".php",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
}

fn reserve_entry_points(root: &Path, selected: Vec<String>, budget: usize) -> Vec<String> {
    let entry_points = entry_point_paths(root, budget);
    if entry_points.is_empty() || budget <= 1 {
        return selected;
    }
    let reserved = entry_points
        .len()
        .min(ENTRY_POINT_RESERVED_SLOTS)
        .min(budget.saturating_sub(1).max(1));
    let chosen = entry_points.into_iter().take(reserved).collect::<Vec<_>>();
    let chosen_set = chosen.iter().collect::<BTreeSet<_>>();
    let mut result = selected
        .into_iter()
        .filter(|path| !chosen_set.contains(path))
        .take(budget.saturating_sub(chosen.len()))
        .collect::<Vec<_>>();
    result.extend(chosen);
    result
}

fn entry_point_paths(root: &Path, limit: usize) -> Vec<String> {
    ENTRY_POINT_CANDIDATES
        .iter()
        .filter(|path| root.join(path).is_file())
        .take(limit)
        .map(|path| (*path).to_string())
        .collect()
}

fn is_generic_exploration_query(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    contains_any(&lower, GENERIC_EXPLORATION_PHRASES)
}

fn operational_path_boost(prompt: &str, path: &str) -> i64 {
    let prompt = prompt.to_ascii_lowercase();
    let path = path.to_ascii_lowercase();
    let mut boost = 0;
    if (prompt.contains("context pack") || prompt.contains("context-pack"))
        && (path.contains("context_pack") || path.contains("context-pack"))
    {
        boost += 400;
    }
    if (prompt.contains("first moves") || prompt.contains("first_moves"))
        && path.contains("first_moves")
    {
        boost += 250;
    }
    if prompt.contains("hook")
        && (path.contains("/hooks/") || path.contains("_hook.py") || path.contains("_hook.ps1"))
    {
        boost += 350;
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
    if (prompt.contains("bootstrap") || prompt.contains("installer") || prompt.contains("install"))
        && (path.contains("install")
            || path.contains("bootstrap")
            || path.contains("setup")
            || path.contains("requirements")
            || path.ends_with(".bat")
            || path.ends_with(".cmd"))
    {
        boost += 450;
    }
    boost
}

fn prompt_terms(prompt: &str) -> BTreeSet<String> {
    prompt
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .map(str::trim)
        .filter(|token| token.len() >= 3)
        .filter(|token| !is_prompt_stopword(&token.to_ascii_lowercase()))
        .map(str::to_ascii_lowercase)
        .collect()
}

fn is_prompt_stopword(term: &str) -> bool {
    matches!(
        term,
        "the"
            | "this"
            | "that"
            | "these"
            | "those"
            | "its"
            | "their"
            | "and"
            | "but"
            | "for"
            | "nor"
            | "yet"
            | "with"
            | "from"
            | "into"
            | "onto"
            | "out"
            | "off"
            | "via"
            | "than"
            | "then"
            | "also"
            | "you"
            | "your"
            | "yours"
            | "they"
            | "them"
            | "him"
            | "her"
            | "his"
            | "hers"
            | "our"
            | "ours"
            | "who"
            | "whom"
            | "whose"
            | "what"
            | "which"
            | "are"
            | "was"
            | "were"
            | "been"
            | "being"
            | "have"
            | "has"
            | "had"
            | "having"
            | "did"
            | "does"
            | "doing"
            | "can"
            | "could"
            | "should"
            | "would"
            | "will"
            | "shall"
            | "may"
            | "might"
            | "must"
            | "let"
            | "make"
            | "made"
            | "use"
            | "used"
            | "uses"
            | "using"
            | "get"
            | "got"
            | "give"
            | "given"
            | "take"
            | "took"
            | "want"
            | "wants"
            | "how"
            | "why"
            | "when"
            | "where"
            | "tell"
            | "say"
            | "said"
            | "all"
            | "any"
            | "some"
            | "one"
            | "two"
            | "three"
            | "many"
            | "much"
            | "more"
            | "most"
            | "less"
            | "few"
            | "fewer"
            | "not"
            | "now"
            | "just"
            | "only"
            | "still"
            | "ever"
            | "even"
            | "each"
            | "every"
            | "both"
            | "either"
            | "neither"
            | "here"
            | "there"
            | "thus"
            | "very"
            | "really"
            | "quite"
    )
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
                | ".hg"
                | ".svn"
                | ".cache"
                | ".gsd"
                | ".gradle"
                | ".idea"
                | ".mypy_cache"
                | ".next"
                | ".pytest_cache"
                | ".ruff_cache"
                | ".tox"
                | ".turbo"
                | ".venv"
                | "__pycache__"
                | "_deps"
                | "bazel-bin"
                | "bazel-out"
                | "bazel-testlogs"
                | "build"
                | "build_standalone"
                | "coverage"
                | "dist"
                | "graphify-out"
                | "htmlcov"
                | "logs"
                | "node_modules"
                | "out"
                | "repomix-output"
                | "target"
                | "venv"
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
        let pack_prefix = packed.split("\n\n").next().expect("pack prefix");
        assert!(pack_prefix.contains("mode=\"exact\""));
        assert!(pack_prefix.contains("USAGE:"));
        assert!(!pack_prefix.contains("FIRST_READS"));
        assert!(packed.ends_with(message));
    }

    #[test]
    fn preserves_existing_scout_hint_message() {
        let message = "SCOUT_HINT (candidate paths from a static term-matching heuristic):\n- src/lib.rs\n\nDo the task.";
        assert_eq!(
            prepend_context_pack_to_message(Path::new("."), message, 16),
            message
        );
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
        assert!(pack.contains("mode=\"scout\""));
        assert!(pack.contains("SCOUT_HINT"));
        assert!(pack.contains("USAGE:"));
        assert!(!pack.contains("FIRST_READS:"));
        assert!(pack.contains("FRESHNESS:"));
        assert!(pack.contains("VERIFICATION:"));
    }

    #[test]
    fn skips_substring_false_positive_prompt() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir");
        fs::write(temp.path().join("src/login_flow.rs"), "fn login() {}").expect("write");

        let request = ContextPackRequest::new(temp.path(), "the contest result was inconclusive");
        assert!(render_graphify_scout_pack(&request).is_none());
    }

    #[test]
    fn skips_docs_report_summary_without_explicit_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("docs")).expect("mkdir docs");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir src");
        fs::write(temp.path().join("docs/report.md"), "summary").expect("write docs");
        fs::write(temp.path().join("src/report.rs"), "fn report() {}").expect("write src");

        let request =
            ContextPackRequest::new(temp.path(), "Summarize the docs and compare reports");
        assert!(render_graphify_scout_pack(&request).is_none());
    }

    #[test]
    fn skips_broad_common_search_prompt() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir src");
        fs::write(temp.path().join("src/config.rs"), "fn config() {}").expect("write");

        let request =
            ContextPackRequest::new(temp.path(), "Search broadly for config in this repo");
        assert!(render_graphify_scout_pack(&request).is_none());
    }

    #[test]
    fn skips_weak_single_symbol_prompt() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir src");
        fs::write(
            temp.path().join("src/pipeline.rs"),
            "fn run_pipeline_kernel_mode() {}",
        )
        .expect("write");

        let request =
            ContextPackRequest::new(temp.path(), "tell me about run_pipeline_kernel_mode");
        assert!(render_graphify_scout_pack(&request).is_none());
    }

    #[test]
    fn renders_exact_pack_for_repo_relative_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("codex-rs/context-pack/src")).expect("mkdir");
        fs::write(
            temp.path().join("codex-rs/context-pack/src/lib.rs"),
            "pub fn render() {}",
        )
        .expect("write");
        fs::create_dir_all(temp.path().join("codex-rs/core/src")).expect("mkdir core");
        fs::write(
            temp.path().join("codex-rs/core/src/lib.rs"),
            "pub fn core() {}",
        )
        .expect("write core");

        let request = ContextPackRequest::new(
            temp.path(),
            "what does codex-rs/context-pack/src/lib.rs do?",
        );
        let pack = render_graphify_scout_pack(&request).expect("pack");
        assert!(pack.contains("mode=\"exact\""));
        assert!(pack.contains("EXACT_MATCH"));
        assert!(pack.contains("codex-rs/context-pack/src/lib.rs"));
        assert!(!pack.contains("codex-rs/core/src/lib.rs"));
        assert!(!pack.contains("SCOUT_HINT"));
    }

    #[test]
    fn renders_exact_pack_for_duplicate_basename() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("core")).expect("mkdir core");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir src");
        fs::write(temp.path().join("core/pipeline_runner.cpp"), "// core").expect("write core");
        fs::write(temp.path().join("src/pipeline_runner.cpp"), "// src").expect("write src");
        fs::write(temp.path().join("src/helper.cpp"), "// helper").expect("write helper");

        let request = ContextPackRequest::new(temp.path(), "summarise pipeline_runner.cpp");
        let pack = render_graphify_scout_pack(&request).expect("pack");
        assert!(pack.contains("mode=\"exact\""));
        assert!(pack.contains("core/pipeline_runner.cpp"));
        assert!(pack.contains("src/pipeline_runner.cpp"));
        assert!(!pack.contains("src/helper.cpp"));
    }

    #[test]
    fn limits_content_scoring_to_bounded_candidate_window() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir");
        for index in 0..300 {
            let body = if matches!(index, 255 | 299) {
                "find raremarker behavior"
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
    fn prunes_python_tooling_and_vcs_metadata_dirs() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir src");
        for ignored in [
            ".pytest_cache",
            ".mypy_cache",
            ".ruff_cache",
            ".tox",
            ".hg",
            ".svn",
            "coverage",
            "htmlcov",
            "venv",
            "graphify-out",
            "repomix-output",
        ] {
            let dir = temp.path().join(ignored);
            fs::create_dir_all(&dir).expect("mkdir ignored");
            fs::write(dir.join("noise.py"), "raremarker noise").expect("write");
        }
        fs::write(
            temp.path().join("src/payload.py"),
            "raremarker context pack",
        )
        .expect("write src");

        let inventory = repo_inventory(temp.path(), "Find raremarker payload");
        for ignored in [
            ".pytest_cache/",
            ".mypy_cache/",
            ".ruff_cache/",
            ".tox/",
            ".hg/",
            ".svn/",
            "coverage/",
            "htmlcov/",
            "venv/",
            "graphify-out/",
            "repomix-output/",
        ] {
            assert!(
                inventory.iter().all(|file| !file.path.contains(ignored)),
                "inventory should not contain {ignored}; got {:?}",
                inventory.iter().map(|f| &f.path).collect::<Vec<_>>()
            );
        }

        let request = ContextPackRequest::new(temp.path(), "Find raremarker payload context pack");
        let pack = render_graphify_scout_pack(&request).expect("pack");
        assert!(pack.contains("src/payload.py"));
        for ignored in [
            ".pytest_cache",
            ".mypy_cache",
            ".ruff_cache",
            ".tox",
            ".hg",
            ".svn",
            "/coverage/",
            "htmlcov",
            "venv/",
            "graphify-out",
            "repomix-output",
        ] {
            assert!(
                !pack.contains(ignored),
                "pack should not contain {ignored}: {pack}"
            );
        }
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

        assert!(pack.contains("mode=\"exact\""));
        assert!(pack.contains("EXACT_MATCH"));
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
