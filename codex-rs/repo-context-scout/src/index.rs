use std::collections::BTreeMap;
use std::fs;
use std::path::Component;
use std::path::Path;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use walkdir::WalkDir;

use crate::git::current_git_head;
use crate::git::relative_slash_path;
use crate::types::Anchor;
use crate::types::ChangedAreas;
use crate::types::FileRecord;
use crate::types::RepoContextScoutConfig;
use crate::types::RepoIndex;
use crate::types::Result;

const SCHEMA_VERSION: u32 = 1;

pub(crate) fn load_index(path: &Path) -> Result<Option<RepoIndex>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&text)?))
}

pub(crate) fn save_index(path: &Path, index: &RepoIndex) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(index)?)?;
    Ok(())
}

pub(crate) fn build_index(root: &Path, config: &RepoContextScoutConfig) -> Result<RepoIndex> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut files = Vec::new();
    let mut file_limit_reached = false;
    for entry in WalkDir::new(root.as_path())
        .into_iter()
        .filter_entry(|entry| !is_skipped_entry(root.as_path(), entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(relative) = relative_slash_path(root.as_path(), entry.path()) else {
            continue;
        };
        if is_skipped_path(&relative) {
            continue;
        }
        if files.len() >= config.max_files {
            file_limit_reached = true;
            break;
        }
        files.push(read_file_record(root.as_path(), &relative, config)?);
    }
    Ok(RepoIndex {
        schema_version: SCHEMA_VERSION,
        project_root: root.clone(),
        generated_at_unix: unix_now(),
        git_head: current_git_head(root.as_path()),
        files,
        file_limit_reached,
    })
}

pub(crate) fn with_changed_overlay(
    mut index: RepoIndex,
    changed: &ChangedAreas,
    config: &RepoContextScoutConfig,
) -> Result<RepoIndex> {
    let mut by_path = index
        .files
        .iter()
        .enumerate()
        .map(|(idx, file)| (file.path.clone(), idx))
        .collect::<BTreeMap<_, _>>();
    for changed_path in &changed.paths {
        if by_path.contains_key(&changed_path.path) || is_skipped_path(&changed_path.path) {
            continue;
        }
        let abs = index.project_root.join(&changed_path.path);
        if !abs.is_file() {
            continue;
        }
        let record = read_file_record(index.project_root.as_path(), &changed_path.path, config)?;
        by_path.insert(record.path.clone(), index.files.len());
        index.files.push(record);
    }
    Ok(index)
}

fn read_file_record(
    root: &Path,
    relative: &str,
    config: &RepoContextScoutConfig,
) -> Result<FileRecord> {
    let path = root.join(relative);
    let metadata = fs::metadata(&path)?;
    let language = language_for_path(relative).to_string();
    let modified_unix = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let mut line_count = 0;
    let mut anchors = Vec::new();
    if metadata.len() <= config.max_file_bytes
        && let Ok(text) = fs::read_to_string(&path)
    {
        line_count = text.lines().count();
        anchors = anchors_for_text(&language, &text, config.max_anchors_per_file);
    }
    Ok(FileRecord {
        path: relative.to_string(),
        size: metadata.len(),
        modified_unix,
        language,
        line_count,
        anchors,
    })
}

fn anchors_for_text(language: &str, text: &str, limit: usize) -> Vec<Anchor> {
    if language == "cpp" {
        return cpp_anchors_for_text(text, limit);
    }
    let mut anchors = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let keep = match language {
            "rust" => starts_any(
                trimmed,
                &[
                    "pub fn ",
                    "fn ",
                    "pub struct ",
                    "struct ",
                    "pub enum ",
                    "enum ",
                    "pub trait ",
                    "trait ",
                    "impl ",
                    "mod ",
                ],
            ),
            "javascript" | "typescript" => starts_any(
                trimmed,
                &[
                    "export function ",
                    "function ",
                    "export class ",
                    "class ",
                    "export interface ",
                    "interface ",
                    "export type ",
                    "type ",
                    "const ",
                ],
            ),
            "python" => starts_any(trimmed, &["def ", "class "]),
            "powershell" => starts_any(
                trimmed,
                &["function ", "param(", "class ", "using ", "Import-Module"],
            ),
            "markdown" => trimmed.starts_with('#'),
            "toml" => trimmed.starts_with('['),
            _ => false,
        };
        if keep {
            anchors.push(Anchor {
                line: idx + 1,
                text: trimmed.chars().take(160).collect(),
            });
            if anchors.len() >= limit {
                break;
            }
        }
    }
    anchors
}

fn cpp_anchors_for_text(text: &str, limit: usize) -> Vec<Anchor> {
    let mut anchors = Vec::new();
    let mut brace_depth = 0usize;
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let keep = starts_any(
            trimmed,
            &["#include", "namespace ", "class ", "struct ", "enum "],
        ) || (brace_depth <= 1 && looks_like_cpp_function(trimmed));
        if keep {
            anchors.push(Anchor {
                line: idx + 1,
                text: trimmed.chars().take(160).collect(),
            });
            if anchors.len() >= limit {
                break;
            }
        }
        brace_depth = update_brace_depth(brace_depth, trimmed);
    }
    anchors
}

fn looks_like_cpp_function(line: &str) -> bool {
    if !line.contains('(') || !line.contains(')') {
        return false;
    }
    if starts_any(line, &["if ", "for ", "while ", "switch ", "return "]) {
        return false;
    }
    line.ends_with('{') || line.ends_with(';')
}

fn update_brace_depth(depth: usize, line: &str) -> usize {
    let opens = line.chars().filter(|ch| *ch == '{').count();
    let closes = line.chars().filter(|ch| *ch == '}').count();
    depth.saturating_add(opens).saturating_sub(closes)
}

fn starts_any(text: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| text.starts_with(prefix))
}

pub(crate) fn is_skipped_path(relative: &str) -> bool {
    let path = relative.replace('\\', "/");
    path.split('/').any(|part| {
        matches!(
            part,
            ".git"
                | ".hg"
                | ".svn"
                | "target"
                | "node_modules"
                | "dist"
                | "build"
                | "coverage"
                | ".cache"
                | ".pytest_cache"
                | "__pycache__"
                | "_deps"
                | ".venv"
                | "logs"
                | "build_standalone"
                | "graphify-out"
                | "repomix-output"
                | ".gsd"
        ) || part.starts_with("cmake-build")
    })
}

fn is_skipped_entry(root: &Path, path: &Path) -> bool {
    if path == root {
        return false;
    }
    let Some(relative) = relative_slash_path(root, path) else {
        return false;
    };
    path.components().any(|component| match component {
        Component::Normal(name) => name.to_str().is_some_and(is_skipped_path),
        _ => false,
    }) || is_skipped_path(&relative)
}

fn language_for_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".rs") {
        "rust"
    } else if lower.ends_with(".ts") || lower.ends_with(".tsx") {
        "typescript"
    } else if lower.ends_with(".js") || lower.ends_with(".jsx") || lower.ends_with(".mjs") {
        "javascript"
    } else if lower.ends_with(".py") {
        "python"
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
    } else if lower.ends_with(".md") || lower.ends_with(".markdown") {
        "markdown"
    } else if lower.ends_with(".toml") {
        "toml"
    } else if lower.ends_with(".json") {
        "json"
    } else if lower.ends_with(".yml") || lower.ends_with(".yaml") {
        "yaml"
    } else {
        "text"
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
