use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use serde_json::Value;
use sha1::Digest;
use sha1::Sha1;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const DEFAULT_MIN_REDUCE_CHARS: usize = 2_000;
const DEFAULT_PATH_LIST_THRESHOLD: usize = 12;
const DEFAULT_MIN_SAVED_TOKENS: usize = 128;
const DEFAULT_PRESERVE_RECENT_ITEMS: usize = 4;
const EXCERPT_CHARS: usize = 220;

/// Configuration for prompt-only reduction.
///
/// The reducer mutates only the prompt clone sent to the model. Callers must
/// keep persisted rollout history unchanged so exact artifacts can be
/// recovered or re-read on demand.
#[derive(Debug, Clone)]
pub struct PromptReductionConfig {
    pub artifact_dir: PathBuf,
    pub min_reduce_chars: usize,
    pub path_list_threshold: usize,
    pub min_saved_tokens: usize,
    pub preserve_recent_items: usize,
}

impl PromptReductionConfig {
    pub fn for_turn(turn_id: &str) -> Self {
        let safe_turn_id = safe_label(turn_id);
        Self {
            artifact_dir: std::env::temp_dir().join("codex-prompt-reducer").join(
                if safe_turn_id.is_empty() {
                    std::process::id().to_string()
                } else {
                    safe_turn_id
                },
            ),
            min_reduce_chars: DEFAULT_MIN_REDUCE_CHARS,
            path_list_threshold: DEFAULT_PATH_LIST_THRESHOLD,
            min_saved_tokens: DEFAULT_MIN_SAVED_TOKENS,
            preserve_recent_items: DEFAULT_PRESERVE_RECENT_ITEMS,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromptReductionStats {
    pub original_tokens: usize,
    pub reduced_tokens: usize,
    pub saved_tokens: usize,
    pub artifacts: usize,
    pub reductions: usize,
}

#[derive(Debug, Clone)]
struct CandidateReduction {
    reason: &'static str,
    digest: String,
}

pub fn reduce_prompt_items(
    items: &mut [ResponseItem],
    config: &PromptReductionConfig,
) -> std::io::Result<PromptReductionStats> {
    fs::create_dir_all(&config.artifact_dir)?;
    let total_text_slots = count_text_slots(items);
    let recent_text_start = total_text_slots.saturating_sub(config.preserve_recent_items);
    let mut text_slot_index = 0usize;
    let mut seen_hashes = HashMap::<String, String>::new();
    let mut call_sources = HashMap::<String, String>::new();
    let mut stats = PromptReductionStats::default();

    for item in items {
        match item {
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            } => {
                call_sources.insert(call_id.clone(), call_source(name, arguments));
            }
            ResponseItem::CustomToolCall {
                call_id,
                name,
                input,
                ..
            } => {
                call_sources.insert(
                    call_id.clone(),
                    format!("custom_tool_output:{name}:{input}"),
                );
            }
            ResponseItem::Message { role, content, .. } => {
                for content_item in content {
                    let text = match content_item {
                        ContentItem::InputText { text } | ContentItem::OutputText { text } => text,
                        ContentItem::InputImage { .. } => continue,
                    };
                    let recent_prompt_item = text_slot_index >= recent_text_start;
                    text_slot_index += 1;
                    reduce_text_slot(
                        text,
                        &format!("message:{role}"),
                        text_slot_index,
                        recent_prompt_item,
                        config,
                        &mut seen_hashes,
                        &mut stats,
                    )?;
                }
            }
            ResponseItem::FunctionCallOutput {
                call_id, output, ..
            }
            | ResponseItem::CustomToolCallOutput {
                call_id, output, ..
            } => {
                let source = call_sources
                    .get(call_id)
                    .cloned()
                    .unwrap_or_else(|| "tool_output".to_string());
                if let Some(text) = output.text_content_mut() {
                    let recent_prompt_item = text_slot_index >= recent_text_start;
                    text_slot_index += 1;
                    reduce_text_slot(
                        text,
                        &source,
                        text_slot_index,
                        recent_prompt_item,
                        config,
                        &mut seen_hashes,
                        &mut stats,
                    )?;
                }
            }
            ResponseItem::ToolSearchOutput { tools, .. } => {
                reduce_tool_search_output(tools, text_slot_index, config, &mut stats)?;
            }
            _ => {}
        }
    }

    stats.saved_tokens = stats.original_tokens.saturating_sub(stats.reduced_tokens);
    Ok(stats)
}

fn count_text_slots(items: &[ResponseItem]) -> usize {
    items
        .iter()
        .map(|item| match item {
            ResponseItem::Message { content, .. } => content
                .iter()
                .filter(|content_item| {
                    matches!(
                        content_item,
                        ContentItem::InputText { .. } | ContentItem::OutputText { .. }
                    )
                })
                .count(),
            ResponseItem::FunctionCallOutput { output, .. }
            | ResponseItem::CustomToolCallOutput { output, .. } => {
                usize::from(output.text_content().is_some())
            }
            ResponseItem::ToolSearchOutput { tools, .. } => usize::from(!tools.is_empty()),
            _ => 0,
        })
        .sum()
}

#[allow(clippy::too_many_arguments)]
fn reduce_text_slot(
    text: &mut String,
    source: &str,
    text_slot_index: usize,
    recent_prompt_item: bool,
    config: &PromptReductionConfig,
    seen_hashes: &mut HashMap<String, String>,
    stats: &mut PromptReductionStats,
) -> std::io::Result<()> {
    let original = text.clone();
    let original_tokens = approx_tokens(&original);
    stats.original_tokens = stats.original_tokens.saturating_add(original_tokens);

    let sha1 = sha1_hex(&original);
    let exact_preserve_reason = exact_preserve_reason(source, &original);
    let candidate = classify_candidate(
        source,
        &original,
        config,
        seen_hashes,
        exact_preserve_reason,
        recent_prompt_item,
    );
    seen_hashes.insert(sha1.clone(), format!("text-slot-{text_slot_index}"));

    let Some(candidate) = candidate else {
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(original_tokens);
        return Ok(());
    };

    let artifact_path = artifact_path_for(
        &config.artifact_dir,
        text_slot_index,
        candidate.reason,
        &sha1,
    );
    let replacement = render_replacement(
        candidate.reason,
        &candidate.digest,
        &artifact_path,
        original.chars().count(),
        original_tokens,
        &sha1,
    );
    let reduced_tokens = approx_tokens(&replacement);
    if original_tokens.saturating_sub(reduced_tokens) < config.min_saved_tokens {
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(original_tokens);
        return Ok(());
    }

    write_artifact(&artifact_path, &original)?;
    *text = replacement;
    stats.reduced_tokens = stats.reduced_tokens.saturating_add(reduced_tokens);
    stats.artifacts += 1;
    stats.reductions += 1;
    Ok(())
}

fn reduce_tool_search_output(
    tools: &mut Vec<Value>,
    text_slot_index: usize,
    config: &PromptReductionConfig,
    stats: &mut PromptReductionStats,
) -> std::io::Result<()> {
    let Ok(original) = serde_json::to_string_pretty(tools) else {
        return Ok(());
    };
    let original_tokens = approx_tokens(&original);
    stats.original_tokens = stats.original_tokens.saturating_add(original_tokens);
    if original.chars().count() < config.min_reduce_chars {
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(original_tokens);
        return Ok(());
    }
    let sha1 = sha1_hex(&original);
    let digest = json_digest(&original).unwrap_or_else(|| {
        format!(
            "tool_search_digest\nitems_total: {}\nexcerpt:\n{}",
            tools.len(),
            excerpt(&original)
        )
    });
    let artifact_path =
        artifact_path_for(&config.artifact_dir, text_slot_index, "tool_search", &sha1);
    let replacement = render_replacement(
        "tool_search_digest",
        &digest,
        &artifact_path,
        original.chars().count(),
        original_tokens,
        &sha1,
    );
    let reduced_tokens = approx_tokens(&replacement);
    if original_tokens.saturating_sub(reduced_tokens) < config.min_saved_tokens {
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(original_tokens);
        return Ok(());
    }

    write_artifact(&artifact_path, &original)?;
    *tools = vec![serde_json::json!({
        "type": "prompt_reduction",
        "reason": "tool_search_digest",
        "artifact": artifact_path.display().to_string(),
        "summary": replacement,
    })];
    stats.reduced_tokens = stats.reduced_tokens.saturating_add(reduced_tokens);
    stats.artifacts += 1;
    stats.reductions += 1;
    Ok(())
}

fn classify_candidate(
    source: &str,
    text: &str,
    config: &PromptReductionConfig,
    seen_hashes: &HashMap<String, String>,
    exact_preserve_reason: Option<&'static str>,
    recent_prompt_item: bool,
) -> Option<CandidateReduction> {
    if text.chars().count() < config.min_reduce_chars {
        return None;
    }
    let sha1 = sha1_hex(text);
    if let Some(first_item) = seen_hashes.get(&sha1) {
        return Some(CandidateReduction {
            reason: "duplicate_block",
            digest: format!("Exact duplicate of earlier prompt item `{first_item}`."),
        });
    }
    if recent_prompt_item {
        return None;
    }
    match exact_preserve_reason {
        Some("source_read") => {
            return Some(CandidateReduction {
                reason: "source_read_digest",
                digest: source_read_digest(source, text),
            });
        }
        Some("diff_hunk") => {
            return Some(CandidateReduction {
                reason: "diff_hunk_digest",
                digest: diff_hunk_digest(text),
            });
        }
        Some("compiler_diagnostic") => {
            return Some(CandidateReduction {
                reason: "compiler_diagnostic_digest",
                digest: compiler_diagnostic_digest(text),
            });
        }
        Some(_) => return None,
        None => {}
    }
    if is_self_review_anchor(text) {
        return Some(CandidateReduction {
            reason: "self_review_inventory",
            digest: self_review_digest(text),
        });
    }
    let path_set = inventory_paths(text);
    if path_set.len() >= config.path_list_threshold {
        return Some(CandidateReduction {
            reason: "path_inventory",
            digest: render_compact_path_list("path_inventory_digest", &path_set, 24),
        });
    }
    if let Some(digest) = json_digest(text) {
        return Some(CandidateReduction {
            reason: "json_digest",
            digest,
        });
    }
    if looks_like_command_log(text) {
        return Some(CandidateReduction {
            reason: "command_log_digest",
            digest: command_log_digest(text),
        });
    }
    None
}

fn call_source(name: &str, arguments: &str) -> String {
    if name == "shell_command"
        && let Ok(value) = serde_json::from_str::<Value>(arguments)
        && let Some(command) = value.get("command").and_then(Value::as_str)
    {
        return format!("shell_output:{command}");
    }
    format!("tool_output:{name}")
}

fn render_replacement(
    reason: &str,
    digest: &str,
    artifact_path: &Path,
    original_chars: usize,
    original_tokens: usize,
    sha1: &str,
) -> String {
    format!(
        "[prompt reduction: {reason}]\noriginal_chars: {original_chars}\noriginal_tokens_estimate: {original_tokens}\nsha1: {sha1}\nartifact: `{}`\nrecovery: read artifact before using exact lines.\n\n{digest}",
        artifact_path.display()
    )
}

fn artifact_path_for(artifact_dir: &Path, index: usize, reason: &str, sha1: &str) -> PathBuf {
    artifact_dir.join(format!(
        "prompt-item-{index:04}-{reason}-{}.txt",
        &sha1[..12]
    ))
}

fn write_artifact(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

fn safe_label(label: &str) -> String {
    label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn sha1_hex(text: &str) -> String {
    let digest = Sha1::digest(text.as_bytes());
    let mut hex = String::with_capacity(40);
    for byte in digest {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn approx_tokens(text: &str) -> usize {
    text.chars().count().max(1).div_ceil(4)
}

fn normalize_slashes(path: impl AsRef<str>) -> String {
    path.as_ref().replace('\\', "/")
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= max_chars {
            output.push_str("...");
            break;
        }
        output.push(ch);
    }
    output
}

fn exact_preserve_reason(source: &str, text: &str) -> Option<&'static str> {
    let lower_source = source.to_ascii_lowercase();
    let lower = text.to_ascii_lowercase();
    if lower_source.contains("apply_patch")
        || lower.contains("*** begin patch")
        || lower.contains("*** end patch")
    {
        return Some("patch_output");
    }
    if text.contains("diff --git ") || text.lines().any(|line| line.starts_with("@@ ")) {
        return Some("diff_hunk");
    }
    if lower.contains("error[e")
        || lower.contains("traceback (most recent call last)")
        || lower.contains("panic at ")
    {
        return Some("compiler_diagnostic");
    }
    if lower.contains("process running with session id")
        || lower.contains("background process started")
    {
        return Some("active_process");
    }
    if lower_source.contains("get-content")
        || lower_source.contains("select-object -skip")
        || lower_source.starts_with("shell_output:cat ")
        || lower_source.contains(" cat ")
        || lower_source.contains(" sed ")
        || lower_source.contains(" nl ")
    {
        return Some("source_read");
    }
    None
}

fn is_self_review_anchor(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("automatic self-review")
        && (lower.contains("dirty tracked files")
            || lower.contains("dirty-at-anchor")
            || lower.contains("exact diff commands"))
}

fn self_review_digest(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let inventory_paths = inventory_paths(text);
    let diff_commands = lower.matches("git diff").count();
    let no_index_commands = lower.matches("git diff --no-index").count();
    format!(
        "self_review_inventory_digest\npaths_total: {}\ndiff_commands: {}\nno_index_diff_commands: {}\nexcerpt:\n{}",
        inventory_paths.len(),
        diff_commands,
        no_index_commands,
        excerpt(text)
    )
}

fn inventory_paths(text: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for line in text.lines() {
        let trimmed = clean_path_candidate(line);
        if looks_like_inventory_path(&trimmed) {
            paths.insert(normalize_slashes(trimmed));
        }
        for token in line.split([',', ';']) {
            for part in token.split(" -> ") {
                let cleaned = clean_path_candidate(part);
                if looks_like_inventory_path(&cleaned) {
                    paths.insert(normalize_slashes(cleaned));
                }
            }
        }
    }
    paths
}

fn clean_path_candidate(text: &str) -> String {
    text.trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .trim_start_matches("?? ")
        .trim_start_matches("M ")
        .trim_start_matches("D ")
        .trim_start_matches("A ")
        .trim_start_matches("R ")
        .trim()
        .to_string()
}

fn looks_like_inventory_path(text: &str) -> bool {
    let normalized = normalize_slashes(text);
    if normalized.len() < 5 || normalized.contains(' ') {
        return false;
    }
    if normalized.starts_with("C:/") {
        return true;
    }
    let lower = normalized.to_ascii_lowercase();
    lower.contains('/')
        && [
            ".rs", ".toml", ".json", ".md", ".snap", ".ps1", ".txt", ".lock", ".yaml", ".yml",
        ]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn render_compact_path_list(operation: &str, paths: &BTreeSet<String>, limit: usize) -> String {
    let selected_paths = paths.iter().take(limit).cloned().collect::<Vec<_>>();
    let omitted = paths.len().saturating_sub(selected_paths.len());
    let mut extension_counts = BTreeMap::<String, usize>::new();
    for path in paths {
        let extension = Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .filter(|extension| !extension.is_empty())
            .unwrap_or("(none)")
            .to_string();
        *extension_counts.entry(extension).or_default() += 1;
    }
    let mut lines = vec![
        operation.to_string(),
        format!("paths_total: {}", paths.len()),
        format!("extensions: {}", render_counts(&extension_counts)),
        format!("paths: {} shown, {omitted} omitted", selected_paths.len()),
    ];
    if omitted > 0 {
        lines.push("fallback_required: true".to_string());
        lines.push("fallback_reason: max_paths".to_string());
    }
    lines.extend(selected_paths);
    lines.join("\n")
}

fn render_counts(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn json_digest(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return None;
    }
    let value = serde_json::from_str::<Value>(trimmed).ok()?;
    let digest = match value {
        Value::Array(items) => {
            let samples = items
                .iter()
                .take(5)
                .map(json_sample)
                .collect::<Vec<_>>()
                .join("; ");
            format!(
                "json_digest\narray_len: {}\nsamples: {samples}",
                items.len()
            )
        }
        Value::Object(map) => {
            let keys = map.keys().take(30).cloned().collect::<Vec<_>>();
            format!(
                "json_digest\nobject_keys_total: {}\nkeys: {}",
                map.len(),
                keys.join(", ")
            )
        }
        other => format!("json_digest\nscalar: {}", json_sample(&other)),
    };
    Some(digest)
}

fn json_sample(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let keys = map.keys().take(8).cloned().collect::<Vec<_>>();
            format!("object({})", keys.join(","))
        }
        Value::Array(items) => format!("array(len={})", items.len()),
        Value::String(text) => format!("string({})", truncate(text, 60)),
        Value::Number(number) => number.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => "null".to_string(),
    }
}

fn looks_like_command_log(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    text.lines().count() >= 40
        || lower.contains("exit code:")
        || lower.contains("wall time:")
        || lower.contains("stdout")
        || lower.contains("stderr")
}

fn command_log_digest(text: &str) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let mut status_lines = Vec::new();
    for line in &lines {
        let lower = line.to_ascii_lowercase();
        if lower.contains("exit code:")
            || lower.contains("wall time:")
            || lower.starts_with("error:")
            || lower.starts_with("warning:")
        {
            status_lines.push((*line).to_string());
        }
        if status_lines.len() >= 12 {
            break;
        }
    }
    format!(
        "command_log_digest\nlines_total: {}\nstatus_lines: {}\nexcerpt:\n{}",
        lines.len(),
        if status_lines.is_empty() {
            "(none)".to_string()
        } else {
            status_lines.join(" | ")
        },
        excerpt(text)
    )
}

fn source_read_digest(source: &str, text: &str) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let paths = inventory_paths(source);
    let path_summary = if paths.is_empty() {
        "(unknown)".to_string()
    } else {
        paths.into_iter().take(12).collect::<Vec<_>>().join(", ")
    };
    format!(
        "source_read_digest\nsource: {}\nlines_total: {}\nchars_total: {}\npaths: {}\nexcerpt:\n{}",
        truncate(source, 220),
        lines.len(),
        text.chars().count(),
        path_summary,
        excerpt(text)
    )
}

fn diff_hunk_digest(text: &str) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let files = lines
        .iter()
        .filter_map(|line| line.strip_prefix("diff --git "))
        .take(20)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let hunk_count = lines.iter().filter(|line| line.starts_with("@@ ")).count();
    let additions = lines
        .iter()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .count();
    let removals = lines
        .iter()
        .filter(|line| line.starts_with('-') && !line.starts_with("---"))
        .count();
    format!(
        "diff_hunk_digest\nlines_total: {}\nfiles: {}\nhunks: {}\nadditions: {}\nremovals: {}\nexcerpt:\n{}",
        lines.len(),
        if files.is_empty() {
            "(unknown)".to_string()
        } else {
            files.join(" | ")
        },
        hunk_count,
        additions,
        removals,
        excerpt(text)
    )
}

fn compiler_diagnostic_digest(text: &str) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let mut diagnostic_lines = Vec::new();
    for line in &lines {
        let lower = line.to_ascii_lowercase();
        if lower.contains("error[")
            || lower.starts_with("error:")
            || lower.starts_with("warning:")
            || lower.contains("failed")
            || lower.contains("panicked")
        {
            diagnostic_lines.push((*line).to_string());
        }
        if diagnostic_lines.len() >= 20 {
            break;
        }
    }
    let error_count = lines
        .iter()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("error[") || lower.starts_with("error:")
        })
        .count();
    let warning_count = lines
        .iter()
        .filter(|line| line.to_ascii_lowercase().starts_with("warning:"))
        .count();
    format!(
        "compiler_diagnostic_digest\nlines_total: {}\nerrors: {}\nwarnings: {}\nprimary_lines: {}\nexcerpt:\n{}",
        lines.len(),
        error_count,
        warning_count,
        if diagnostic_lines.is_empty() {
            "(none)".to_string()
        } else {
            diagnostic_lines.join(" | ")
        },
        excerpt(text)
    )
}

fn excerpt(text: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= EXCERPT_CHARS * 2 {
        return text.to_string();
    }
    let head = text.chars().take(EXCERPT_CHARS).collect::<String>();
    let tail = text
        .chars()
        .rev()
        .take(EXCERPT_CHARS)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{head}\n...\n{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::FunctionCallOutputPayload;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    #[test]
    fn reduces_old_source_reads_but_preserves_recent_ones() {
        let old_source = (0..180)
            .map(|index| format!("pub fn function_{index}() -> usize {{ {index} }}"))
            .collect::<Vec<_>>()
            .join("\n");
        let recent_source = (0..180)
            .map(|index| format!("pub fn recent_function_{index}() -> usize {{ {index} }}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut items = vec![
            shell_call("call-old", "Get-Content -LiteralPath src/lib.rs"),
            shell_output("call-old", old_source),
            shell_call("call-recent", "Get-Content -LiteralPath src/main.rs"),
            shell_output("call-recent", recent_source.clone()),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 1);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected output");
        };
        assert!(
            output
                .text_content()
                .unwrap()
                .contains("source_read_digest")
        );
        let ResponseItem::FunctionCallOutput { output, .. } = &items[3] else {
            panic!("expected output");
        };
        assert_eq!(output.text_content().unwrap(), recent_source);
    }

    #[test]
    fn reduces_duplicate_recent_blocks() {
        let paths = (0..80)
            .map(|index| format!("codex-rs/core/src/file_{index}.rs"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut items = vec![
            shell_call("call-a", "rg --files"),
            shell_output("call-a", paths.clone()),
            shell_call("call-b", "rg --files"),
            shell_output("call-b", paths),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 1);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 2);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected output");
        };
        assert!(output.text_content().unwrap().contains("path_inventory"));
        let ResponseItem::FunctionCallOutput { output, .. } = &items[3] else {
            panic!("expected output");
        };
        assert!(output.text_content().unwrap().contains("duplicate_block"));
    }

    #[test]
    fn disabled_by_threshold_keeps_small_outputs_inline() {
        let mut items = vec![
            shell_call("call", "rg --files"),
            shell_output("call", "src/lib.rs".to_string()),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected output");
        };
        assert_eq!(output.text_content().unwrap(), "src/lib.rs");
    }

    fn test_config(path: &Path, preserve_recent_items: usize) -> PromptReductionConfig {
        PromptReductionConfig {
            artifact_dir: path.to_path_buf(),
            min_reduce_chars: 100,
            path_list_threshold: 8,
            min_saved_tokens: 1,
            preserve_recent_items,
        }
    }

    fn shell_call(call_id: &str, command: &str) -> ResponseItem {
        ResponseItem::FunctionCall {
            id: None,
            name: "shell_command".to_string(),
            namespace: None,
            arguments: serde_json::json!({ "command": command }).to_string(),
            call_id: call_id.to_string(),
        }
    }

    fn shell_output(call_id: &str, text: String) -> ResponseItem {
        ResponseItem::FunctionCallOutput {
            call_id: call_id.to_string(),
            output: FunctionCallOutputPayload::from_text(text),
        }
    }
}
