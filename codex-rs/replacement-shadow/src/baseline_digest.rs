use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

const DEFAULT_BASELINE_SUMMARY_LIMIT: usize = 80;
const MAX_DIAGNOSTIC_LINES: usize = 40;
const PROCESS_TOP_ROW_LIMIT: usize = 8;
const INTERESTING_PROCESS_NEEDLES: &[&str] =
    &["codex", "cargo", "rustc", "link", "powershell", "pwsh"];

pub(super) fn render_changed_files_compact(
    operation: &str,
    baseline_model_visible_output: &str,
) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let paths = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("warning:"))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let selected_paths = select_diverse_path_sample(&paths, DEFAULT_BASELINE_SUMMARY_LIMIT);
    let omitted = paths.len().saturating_sub(selected_paths.len());
    let mut extension_counts = BTreeMap::new();
    let mut group_counts = BTreeMap::new();
    for path in &paths {
        let extension = Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .filter(|extension| !extension.is_empty())
            .unwrap_or("(none)")
            .to_string();
        *extension_counts.entry(extension).or_default() += 1;
        *group_counts.entry(path_group(path)).or_default() += 1;
    }

    let mut lines = vec![
        operation.to_string(),
        format!("paths_total: {}", paths.len()),
    ];
    if !extension_counts.is_empty() {
        lines.push(format!("extensions: {}", render_counts(&extension_counts)));
    }
    if !group_counts.is_empty() {
        lines.push(format!(
            "top_dirs: {}",
            render_top_counts(&group_counts, 12)
        ));
    }
    if paths.is_empty() {
        lines.push("status: no_paths".to_string());
        return lines.join("\n");
    }
    lines.push(format!(
        "paths: {} shown, {omitted} omitted",
        selected_paths.len()
    ));
    if omitted > 0 {
        lines.push("fallback_required: true".to_string());
        lines.push("fallback_reason: max_paths".to_string());
    }
    lines.extend(selected_paths);
    lines.join("\n")
}

pub(super) fn render_git_diffstat_compact(baseline_model_visible_output: &str) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let rows = output
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with("warning:"))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let omitted = rows.len().saturating_sub(DEFAULT_BASELINE_SUMMARY_LIMIT);
    let summary = rows
        .iter()
        .rev()
        .find(|line| line.contains(" changed"))
        .cloned();
    let file_rows = rows
        .iter()
        .filter(|line| line.contains('|') && !line.contains(" changed"))
        .count();

    let mut lines = vec![
        "git_diffstat_compact".to_string(),
        format!("diffstat_lines: {}", rows.len()),
        format!("files_with_stat_rows: {file_rows}"),
    ];
    if let Some(summary) = summary {
        lines.push(format!("summary: {}", summary.trim()));
    }
    if rows.is_empty() {
        lines.push("status: no_diffstat".to_string());
        return lines.join("\n");
    }
    lines.push(format!(
        "lines: {} shown, {omitted} omitted",
        rows.len().min(DEFAULT_BASELINE_SUMMARY_LIMIT)
    ));
    if omitted > 0 {
        lines.push("fallback_required: true".to_string());
        lines.push("fallback_reason: max_diffstat_lines".to_string());
    }
    lines.extend(rows.into_iter().take(DEFAULT_BASELINE_SUMMARY_LIMIT));
    lines.join("\n")
}

pub(super) fn render_diff_hunk_summary(baseline_model_visible_output: &str) -> String {
    struct DiffFile {
        path: String,
        hunks: usize,
    }

    let output = baseline_output_text(baseline_model_visible_output);
    let mut files = Vec::<DiffFile>::new();
    for line in output.lines() {
        if line.starts_with("diff --git ") {
            let path = line
                .split_whitespace()
                .nth(3)
                .unwrap_or("")
                .trim_start_matches("b/")
                .trim_matches('"')
                .to_string();
            files.push(DiffFile { path, hunks: 0 });
        } else if line.starts_with("@@")
            && let Some(file) = files.last_mut()
        {
            file.hunks += 1;
        }
    }

    let omitted = files.len().saturating_sub(DEFAULT_BASELINE_SUMMARY_LIMIT);
    let total_hunks = files.iter().map(|file| file.hunks).sum::<usize>();
    let mut lines = vec![
        "diff_hunk_summary".to_string(),
        format!("files: {}", files.len()),
        format!("hunks: {total_hunks}"),
    ];
    if files.is_empty() {
        lines.push("status: no_diff_hunks_detected".to_string());
        return lines.join("\n");
    }
    lines.push(format!(
        "files_list: {} shown, {omitted} omitted",
        files.len().min(DEFAULT_BASELINE_SUMMARY_LIMIT)
    ));
    lines.push("fallback_required: true".to_string());
    lines.push("fallback_reason: lossy_diff_summary".to_string());
    lines.extend(
        files
            .into_iter()
            .take(DEFAULT_BASELINE_SUMMARY_LIMIT)
            .map(|file| format!("file: {}; hunks: {}", file.path, file.hunks)),
    );
    lines.join("\n")
}

pub(super) fn render_run_check_digest(baseline_model_visible_output: &str) -> String {
    let exit_code = baseline_model_visible_output
        .lines()
        .find_map(|line| line.strip_prefix("Exit code: "))
        .unwrap_or("unknown");
    let wall_time = baseline_model_visible_output
        .lines()
        .find_map(|line| line.strip_prefix("Wall time: "))
        .unwrap_or("unknown");
    let output = baseline_output_text(baseline_model_visible_output);
    let output_lines = output.lines().count();
    let diagnostics = output
        .lines()
        .map(str::trim)
        .filter(|line| is_diagnostic_line(line))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let omitted = diagnostics.len().saturating_sub(MAX_DIAGNOSTIC_LINES);
    let mut lines = vec![
        "run_check_digest".to_string(),
        format!("exit_code: {exit_code}"),
        format!("wall_time: {wall_time}"),
        format!("output_lines: {output_lines}"),
        format!(
            "diagnostics: {} shown, {omitted} omitted",
            diagnostics.len().min(MAX_DIAGNOSTIC_LINES)
        ),
    ];
    lines.push("fallback_required: true".to_string());
    lines.push(format!(
        "fallback_reason: {}",
        if omitted > 0 {
            "max_diagnostics"
        } else {
            "lossy_check_output"
        }
    ));
    if diagnostics.is_empty() {
        lines.push("status: no_diagnostics_detected".to_string());
    } else {
        lines.extend(diagnostics.into_iter().take(MAX_DIAGNOSTIC_LINES));
    }
    lines.join("\n")
}

pub(super) fn render_file_excerpt_digest(baseline_model_visible_output: &str) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let output_lines = output.lines().map(ToString::to_string).collect::<Vec<_>>();
    let omitted = output_lines
        .len()
        .saturating_sub(DEFAULT_BASELINE_SUMMARY_LIMIT);
    let mut lines = vec![
        "file_excerpt_digest".to_string(),
        format!("lines_total: {}", output_lines.len()),
        format!(
            "lines: {} shown, {omitted} omitted",
            output_lines.len().min(DEFAULT_BASELINE_SUMMARY_LIMIT)
        ),
    ];
    if omitted > 0 {
        lines.push("fallback_required: true".to_string());
        lines.push("fallback_reason: max_lines".to_string());
    }
    if output_lines.is_empty() {
        lines.push("status: empty_output".to_string());
    } else {
        lines.extend(
            output_lines
                .into_iter()
                .take(DEFAULT_BASELINE_SUMMARY_LIMIT),
        );
    }
    lines.join("\n")
}

pub(super) fn render_select_string_digest(baseline_model_visible_output: &str) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let matches = output
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let omitted = matches.len().saturating_sub(DEFAULT_BASELINE_SUMMARY_LIMIT);
    let mut path_counts = BTreeMap::new();
    for line in &matches {
        if let Some(path) = select_string_path(line)
            && !path.is_empty()
        {
            *path_counts.entry(path.to_string()).or_default() += 1;
        }
    }

    let mut lines = vec![
        "select_string_digest".to_string(),
        format!("matches_total: {}", matches.len()),
        format!(
            "matches: {} shown, {omitted} omitted",
            matches.len().min(DEFAULT_BASELINE_SUMMARY_LIMIT)
        ),
    ];
    if !path_counts.is_empty() {
        lines.push(format!("paths: {}", render_counts(&path_counts)));
    }
    if omitted > 0 {
        lines.push("fallback_required: true".to_string());
        lines.push("fallback_reason: max_matches".to_string());
    }
    if matches.is_empty() {
        lines.push("status: no_matches".to_string());
    } else {
        lines.extend(matches.into_iter().take(DEFAULT_BASELINE_SUMMARY_LIMIT));
    }
    lines.join("\n")
}

pub(super) fn render_rg_count_digest(baseline_model_visible_output: &str) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let counts = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let omitted = counts.len().saturating_sub(DEFAULT_BASELINE_SUMMARY_LIMIT);
    let total_matches = counts
        .iter()
        .filter_map(|line| {
            line.rsplit_once(':')
                .map_or(line.as_str(), |(_, count)| count)
                .parse::<usize>()
                .ok()
        })
        .sum::<usize>();
    let mut lines = vec![
        "rg_count_digest".to_string(),
        format!("count_lines_total: {}", counts.len()),
        format!("matches_total_from_counts: {total_matches}"),
        format!(
            "count_lines: {} shown, {omitted} omitted",
            counts.len().min(DEFAULT_BASELINE_SUMMARY_LIMIT)
        ),
    ];
    if omitted > 0 {
        lines.push("fallback_required: true".to_string());
        lines.push("fallback_reason: max_lines".to_string());
    }
    if counts.is_empty() {
        lines.push("status: no_counts".to_string());
    } else {
        lines.extend(counts.into_iter().take(DEFAULT_BASELINE_SUMMARY_LIMIT));
    }
    lines.join("\n")
}

pub(super) fn render_rg_json_digest(baseline_model_visible_output: &str) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let mut match_events = 0usize;
    let mut context_events = 0usize;
    let mut summary_events = 0usize;
    let mut parse_errors = 0usize;
    let mut path_counts = BTreeMap::new();
    let mut samples = Vec::new();

    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            parse_errors += 1;
            continue;
        };
        match value.get("type").and_then(Value::as_str) {
            Some("match") => {
                match_events += 1;
                let data = value.get("data").unwrap_or(&Value::Null);
                let path = json_text_at(data, &["path", "text"]).unwrap_or("(unknown)");
                *path_counts.entry(path.to_string()).or_default() += 1;
                if samples.len() < MAX_DIAGNOSTIC_LINES {
                    let line_number = data
                        .get("line_number")
                        .and_then(Value::as_u64)
                        .map_or("?".to_string(), |line| line.to_string());
                    let submatches = data
                        .get("submatches")
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len);
                    samples.push(format!("{path}:{line_number}: submatches={submatches}"));
                }
            }
            Some("context") => context_events += 1,
            Some("summary") => summary_events += 1,
            _ => {}
        }
    }

    let omitted = match_events.saturating_sub(samples.len());
    let mut lines = vec![
        "rg_json_digest".to_string(),
        format!("match_events: {match_events}"),
        format!("context_events: {context_events}"),
        format!("summary_events: {summary_events}"),
        format!("parse_errors: {parse_errors}"),
    ];
    if !path_counts.is_empty() {
        lines.push(format!("paths: {}", render_counts(&path_counts)));
    }
    lines.push(format!(
        "samples: {} shown, {omitted} omitted",
        samples.len()
    ));
    lines.push("fallback_required: true".to_string());
    lines.push(format!(
        "fallback_reason: {}",
        if parse_errors > 0 {
            "json_parse_error"
        } else {
            "lossy_rg_json"
        }
    ));
    if samples.is_empty() {
        lines.push("status: no_match_samples".to_string());
    } else {
        lines.extend(samples);
    }
    lines.join("\n")
}

pub(super) fn render_git_name_status_compact(baseline_model_visible_output: &str) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let rows = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("warning:"))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let omitted = rows.len().saturating_sub(DEFAULT_BASELINE_SUMMARY_LIMIT);
    let mut status_counts = BTreeMap::new();
    for row in &rows {
        let status = row.split_whitespace().next().unwrap_or("?").to_string();
        *status_counts.entry(status).or_default() += 1;
    }
    let mut lines = vec![
        "git_name_status_compact".to_string(),
        format!("paths_total: {}", rows.len()),
    ];
    if !status_counts.is_empty() {
        lines.push(format!("status_counts: {}", render_counts(&status_counts)));
    }
    lines.push(format!(
        "paths: {} shown, {omitted} omitted",
        rows.len().min(DEFAULT_BASELINE_SUMMARY_LIMIT)
    ));
    if omitted > 0 {
        lines.push("fallback_required: true".to_string());
        lines.push("fallback_reason: max_paths".to_string());
    }
    if rows.is_empty() {
        lines.push("status: no_paths".to_string());
    } else {
        lines.extend(rows.into_iter().take(DEFAULT_BASELINE_SUMMARY_LIMIT));
    }
    lines.join("\n")
}

pub(super) fn render_git_numstat_compact(baseline_model_visible_output: &str) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let rows = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("warning:"))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let omitted = rows.len().saturating_sub(DEFAULT_BASELINE_SUMMARY_LIMIT);
    let mut added = 0usize;
    let mut deleted = 0usize;
    let mut binary_files = 0usize;
    for row in &rows {
        let mut parts = row.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some("-"), Some("-")) => binary_files += 1,
            (Some(add), Some(del)) => {
                added += add.parse::<usize>().unwrap_or(0);
                deleted += del.parse::<usize>().unwrap_or(0);
            }
            _ => {}
        }
    }
    let mut lines = vec![
        "git_numstat_compact".to_string(),
        format!("files_total: {}", rows.len()),
        format!("added_lines: {added}"),
        format!("deleted_lines: {deleted}"),
        format!("binary_files: {binary_files}"),
        format!(
            "files: {} shown, {omitted} omitted",
            rows.len().min(DEFAULT_BASELINE_SUMMARY_LIMIT)
        ),
    ];
    if omitted > 0 {
        lines.push("fallback_required: true".to_string());
        lines.push("fallback_reason: max_paths".to_string());
    }
    if rows.is_empty() {
        lines.push("status: no_numstat".to_string());
    } else {
        lines.extend(rows.into_iter().take(DEFAULT_BASELINE_SUMMARY_LIMIT));
    }
    lines.join("\n")
}

pub(super) fn render_git_filtered_diff_digest(baseline_model_visible_output: &str) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let raw_lines = output.lines().map(ToString::to_string).collect::<Vec<_>>();
    let samples = raw_lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .take(MAX_DIAGNOSTIC_LINES)
        .cloned()
        .collect::<Vec<_>>();
    let omitted = raw_lines.len().saturating_sub(samples.len());
    let mut lines = vec![
        "git_filtered_diff_digest".to_string(),
        format!("lines_total: {}", raw_lines.len()),
        format!(
            "diff_headers: {}",
            raw_lines
                .iter()
                .filter(|line| line.starts_with("diff --git "))
                .count()
        ),
        format!(
            "hunk_headers: {}",
            raw_lines
                .iter()
                .filter(|line| line.starts_with("@@"))
                .count()
        ),
        format!(
            "added_lines: {}",
            raw_lines
                .iter()
                .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
                .count()
        ),
        format!(
            "removed_lines: {}",
            raw_lines
                .iter()
                .filter(|line| line.starts_with('-') && !line.starts_with("---"))
                .count()
        ),
        format!("samples: {} shown, {omitted} omitted", samples.len()),
        "fallback_required: true".to_string(),
        "fallback_reason: lossy_diff_filter".to_string(),
    ];
    if samples.is_empty() {
        lines.push("status: no_filtered_diff_lines".to_string());
    } else {
        lines.extend(samples);
    }
    lines.join("\n")
}

pub(super) fn render_git_history_digest(baseline_model_visible_output: &str) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let raw_lines = output.lines().map(ToString::to_string).collect::<Vec<_>>();
    let mut samples = Vec::new();
    for line in &raw_lines {
        let trimmed = line.trim_start();
        if line.starts_with("commit ")
            || trimmed.starts_with("Author:")
            || trimmed.starts_with("Date:")
            || line.starts_with("diff --git ")
            || trimmed.contains(" file changed")
            || trimmed.contains(" files changed")
            || looks_like_name_status_or_numstat(trimmed)
        {
            samples.push(line.clone());
            if samples.len() >= MAX_DIAGNOSTIC_LINES {
                break;
            }
        }
    }
    let omitted = raw_lines.len().saturating_sub(samples.len());
    let mut lines = vec![
        "git_history_digest".to_string(),
        format!(
            "commits: {}",
            raw_lines
                .iter()
                .filter(|line| line.starts_with("commit "))
                .count()
        ),
        format!(
            "diff_files: {}",
            raw_lines
                .iter()
                .filter(|line| line.starts_with("diff --git "))
                .count()
        ),
        format!("lines_total: {}", raw_lines.len()),
        format!(
            "metadata_samples: {} shown, {omitted} omitted",
            samples.len()
        ),
        "fallback_required: true".to_string(),
        "fallback_reason: lossy_git_history".to_string(),
    ];
    if samples.is_empty() {
        lines.push("status: no_history_metadata_detected".to_string());
    } else {
        lines.extend(samples);
    }
    lines.join("\n")
}

pub(super) fn render_directory_listing_compact(baseline_model_visible_output: &str) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let entries = output
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let omitted = entries.len().saturating_sub(DEFAULT_BASELINE_SUMMARY_LIMIT);
    let mut extension_counts = BTreeMap::new();
    for entry in &entries {
        let name = entry.split_whitespace().last().unwrap_or(entry);
        let extension = Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .filter(|extension| !extension.is_empty())
            .unwrap_or("(none)")
            .to_string();
        *extension_counts.entry(extension).or_default() += 1;
    }
    let mut lines = vec![
        "directory_listing_compact".to_string(),
        format!("entries_total: {}", entries.len()),
        format!(
            "directories_detected: {}",
            entries
                .iter()
                .filter(|entry| {
                    let trimmed = entry.trim_start();
                    trimmed.starts_with('d') || trimmed.contains("<DIR>")
                })
                .count()
        ),
    ];
    if !extension_counts.is_empty() {
        lines.push(format!("extensions: {}", render_counts(&extension_counts)));
    }
    lines.push(format!(
        "entries: {} shown, {omitted} omitted",
        entries.len().min(DEFAULT_BASELINE_SUMMARY_LIMIT)
    ));
    lines.push("fallback_required: true".to_string());
    lines.push(format!(
        "fallback_reason: {}",
        if omitted > 0 {
            "max_lines"
        } else {
            "lossy_directory_listing"
        }
    ));
    if entries.is_empty() {
        lines.push("status: no_entries".to_string());
    } else {
        lines.extend(entries.into_iter().take(DEFAULT_BASELINE_SUMMARY_LIMIT));
    }
    lines.join("\n")
}

pub(super) fn render_process_table_compact(baseline_model_visible_output: &str) -> String {
    let output = baseline_output_text(baseline_model_visible_output);
    let rows = output
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !line.trim_start().starts_with("---"))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut process_counts = BTreeMap::new();
    for row in &rows {
        let trimmed = row.trim_start();
        if trimmed.starts_with("Name")
            || trimmed.starts_with("Image Name")
            || trimmed.starts_with("Handles")
        {
            continue;
        }
        if let Some(name) = trimmed.split_whitespace().next()
            && !name.is_empty()
        {
            *process_counts.entry(name.to_string()).or_default() += 1;
        }
    }
    let selected_rows = select_process_rows(&rows);
    let omitted = rows.len().saturating_sub(selected_rows.len());
    let mut lines = vec![
        "process_table_compact".to_string(),
        format!("rows_total: {}", rows.len()),
    ];
    if !process_counts.is_empty() {
        lines.push(format!("process_names: {}", render_counts(&process_counts)));
    }
    lines.push(format!(
        "rows: {} shown, {omitted} omitted",
        selected_rows.len()
    ));
    lines.push("fallback_required: true".to_string());
    lines.push(format!(
        "fallback_reason: {}",
        if omitted > 0 {
            "max_processes"
        } else {
            "lossy_process_table"
        }
    ));
    if rows.is_empty() {
        lines.push("status: no_process_rows".to_string());
    } else {
        lines.extend(selected_rows);
    }
    lines.join("\n")
}

fn select_process_rows(rows: &[String]) -> Vec<String> {
    let mut selected = Vec::new();
    let mut seen = BTreeSet::new();
    for row in rows.iter().take(PROCESS_TOP_ROW_LIMIT) {
        push_process_row(&mut selected, &mut seen, row);
    }
    for row in rows {
        if process_row_is_interesting(row) {
            push_process_row(&mut selected, &mut seen, row);
        }
    }
    selected
}

fn process_row_is_interesting(row: &str) -> bool {
    let lower = row.to_ascii_lowercase();
    INTERESTING_PROCESS_NEEDLES
        .iter()
        .any(|needle| lower.contains(needle))
}

fn push_process_row(selected: &mut Vec<String>, seen: &mut BTreeSet<String>, row: &str) {
    if seen.insert(row.to_string()) {
        selected.push(row.to_string());
    }
}

fn baseline_output_text(baseline_model_visible_output: &str) -> Cow<'_, str> {
    if let Some((_, output)) = baseline_model_visible_output
        .split_once("\r\nOutput:\r\n")
        .or_else(|| baseline_model_visible_output.split_once("\nOutput:\n"))
    {
        return Cow::Borrowed(output);
    }
    if let Ok(value) = serde_json::from_str::<Value>(baseline_model_visible_output)
        && let Some(output) = value.get("output").and_then(Value::as_str)
    {
        return Cow::Owned(output.to_string());
    }
    Cow::Borrowed(baseline_model_visible_output)
}

fn json_text_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

fn looks_like_name_status_or_numstat(line: &str) -> bool {
    let mut parts = line.split_whitespace();
    let Some(first) = parts.next() else {
        return false;
    };
    if matches!(first, "A" | "M" | "D" | "R" | "C" | "T" | "U") {
        return parts.next().is_some();
    }
    let Some(second) = parts.next() else {
        return false;
    };
    (first == "-" && second == "-")
        || (first.parse::<usize>().is_ok() && second.parse::<usize>().is_ok())
}

fn select_string_path(line: &str) -> Option<&str> {
    let colon = line.find(':')?;
    if colon == 1
        && line
            .as_bytes()
            .get(2)
            .is_some_and(|separator| matches!(*separator, b'\\' | b'/'))
    {
        let rest = &line[3..];
        let next_colon = rest.find(':')?;
        return Some(line[..3 + next_colon].trim());
    }
    Some(line[..colon].trim())
}

fn select_diverse_path_sample(paths: &[String], limit: usize) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    let mut selected = Vec::new();
    let mut selected_set = BTreeSet::new();
    for path in paths.iter().filter(|path| important_repo_file(path)) {
        selected.push(path.clone());
        selected_set.insert(path.clone());
        if selected.len() >= limit {
            return selected;
        }
    }

    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    for path in paths {
        if selected_set.contains(path) {
            continue;
        }
        grouped
            .entry(path_group(path))
            .or_default()
            .push(path.clone());
    }
    let mut offsets = BTreeMap::<String, usize>::new();
    while selected.len() < limit {
        let mut advanced = false;
        for group in grouped.keys().cloned().collect::<Vec<_>>() {
            if selected.len() >= limit {
                break;
            }
            let offset = offsets.get(&group).copied().unwrap_or_default();
            let Some(path) = grouped.get(&group).and_then(|paths| paths.get(offset)) else {
                continue;
            };
            selected.push(path.clone());
            selected_set.insert(path.clone());
            offsets.insert(group, offset + 1);
            advanced = true;
        }
        if !advanced {
            break;
        }
    }
    selected
}

fn important_repo_file(path: &str) -> bool {
    let lower = normalize_slashes(path).to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "agents.md"
            | "build.rs"
            | "cargo.lock"
            | "cargo.toml"
            | "cmakelists.txt"
            | "package-lock.json"
            | "package.json"
            | "pnpm-lock.yaml"
            | "pyproject.toml"
            | "readme.md"
            | "yarn.lock"
    ) || lower.ends_with("/agents.md")
        || lower.ends_with("/cargo.lock")
        || lower.ends_with("/cargo.toml")
        || lower.ends_with("/cmakelists.txt")
        || lower.ends_with("/package-lock.json")
        || lower.ends_with("/package.json")
        || lower.ends_with("/pnpm-lock.yaml")
        || lower.ends_with("/pyproject.toml")
        || lower.ends_with("/readme.md")
        || lower.ends_with("/yarn.lock")
}

fn path_group(path: &str) -> String {
    let normalized = normalize_slashes(path);
    let parts = normalized.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        [] | [_] => "(root)".to_string(),
        [first, _] => (*first).to_string(),
        [first, second, ..] => format!("{first}/{second}"),
    }
}

fn render_top_counts(counts: &BTreeMap<String, usize>, limit: usize) -> String {
    let mut items = counts.iter().collect::<Vec<_>>();
    items.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    let omitted = items.len().saturating_sub(limit);
    let mut parts = items
        .into_iter()
        .take(limit)
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>();
    if omitted > 0 {
        parts.push(format!("...+{omitted}"));
    }
    parts.join(", ")
}

fn normalize_slashes(path: &str) -> String {
    path.replace('\\', "/")
}

fn render_counts(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn is_diagnostic_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("error:")
        || lower.contains("error[")
        || lower.contains("warning:")
        || lower.contains("warning[")
        || lower.contains("failed")
        || lower.contains("panic")
        || lower.contains("test result:")
        || lower.starts_with("failures:")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn renders_status_and_diff_summaries() {
        assert_eq!(
            render_git_diffstat_compact(
                "Exit code: 0\nWall time: 1s\nOutput:\n src/lib.rs | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n"
            ),
            "git_diffstat_compact\ndiffstat_lines: 2\nfiles_with_stat_rows: 1\nsummary: 1 file changed, 1 insertion(+), 1 deletion(-)\nlines: 2 shown, 0 omitted\n src/lib.rs | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)"
        );

        let diff = "Exit code: 0\nWall time: 1s\nOutput:\ndiff --git a/src/lib.rs b/src/lib.rs\n@@ -1 +1 @@\ndiff --git a/src/main.rs b/src/main.rs\n@@ -2 +2 @@\n@@ -9 +9 @@\n";
        assert_eq!(
            render_diff_hunk_summary(diff),
            "diff_hunk_summary\nfiles: 2\nhunks: 3\nfiles_list: 2 shown, 0 omitted\nfallback_required: true\nfallback_reason: lossy_diff_summary\nfile: src/lib.rs; hunks: 1\nfile: src/main.rs; hunks: 2"
        );
    }

    #[test]
    fn renders_expansion_digests() {
        assert_eq!(
            render_changed_files_compact(
                "rg_file_set_digest",
                r#"{"output":"src/lib.rs\nsrc/main.rs\n","metadata":{"exit_code":0,"duration_seconds":0.1}}"#,
            ),
            "rg_file_set_digest\npaths_total: 2\nextensions: rs=2\ntop_dirs: src=2\npaths: 2 shown, 0 omitted\nsrc/lib.rs\nsrc/main.rs"
        );
        assert_eq!(
            render_file_excerpt_digest("Exit code: 0\nWall time: 1s\nOutput:\nalpha\nbeta\n"),
            "file_excerpt_digest\nlines_total: 2\nlines: 2 shown, 0 omitted\nalpha\nbeta"
        );
        assert_eq!(
            render_select_string_digest(
                "Exit code: 0\nWall time: 1s\nOutput:\nsrc/lib.rs:12:replacement_shadow\n"
            ),
            "select_string_digest\nmatches_total: 1\nmatches: 1 shown, 0 omitted\npaths: src/lib.rs=1\nsrc/lib.rs:12:replacement_shadow"
        );
        assert_eq!(
            render_select_string_digest(
                "Exit code: 0\nWall time: 1s\nOutput:\nC:\\repo\\src\\lib.rs:12:replacement_shadow\n"
            ),
            "select_string_digest\nmatches_total: 1\nmatches: 1 shown, 0 omitted\npaths: C:\\repo\\src\\lib.rs=1\nC:\\repo\\src\\lib.rs:12:replacement_shadow"
        );
        assert_eq!(
            render_rg_count_digest(
                "Exit code: 0\nWall time: 1s\nOutput:\nsrc/lib.rs:3\nsrc/main.rs:2\n"
            ),
            "rg_count_digest\ncount_lines_total: 2\nmatches_total_from_counts: 5\ncount_lines: 2 shown, 0 omitted\nsrc/lib.rs:3\nsrc/main.rs:2"
        );
        assert_eq!(
            render_changed_files_compact(
                "rg_file_set_digest",
                "Exit code: 0\nWall time: 1s\nOutput:\nsrc/lib.rs\nsrc/main.rs\n"
            ),
            "rg_file_set_digest\npaths_total: 2\nextensions: rs=2\ntop_dirs: src=2\npaths: 2 shown, 0 omitted\nsrc/lib.rs\nsrc/main.rs"
        );
        assert_eq!(
            render_git_name_status_compact(
                "Exit code: 0\nWall time: 1s\nOutput:\nM\tsrc/lib.rs\nA\tsrc/main.rs\n"
            ),
            "git_name_status_compact\npaths_total: 2\nstatus_counts: A=1, M=1\npaths: 2 shown, 0 omitted\nM\tsrc/lib.rs\nA\tsrc/main.rs"
        );
        assert_eq!(
            render_git_numstat_compact(
                "Exit code: 0\nWall time: 1s\nOutput:\n10\t2\tsrc/lib.rs\n-\t-\tbin.dat\n"
            ),
            "git_numstat_compact\nfiles_total: 2\nadded_lines: 10\ndeleted_lines: 2\nbinary_files: 1\nfiles: 2 shown, 0 omitted\n10\t2\tsrc/lib.rs\n-\t-\tbin.dat"
        );
    }

    #[test]
    fn compact_path_lists_sample_across_directories_after_important_files() {
        let paths = vec![
            "AGENTS.md".to_string(),
            "alpha/one.rs".to_string(),
            "alpha/two.rs".to_string(),
            "beta/one.rs".to_string(),
            "beta/two.rs".to_string(),
            "gamma/one.rs".to_string(),
        ];

        assert_eq!(
            select_diverse_path_sample(&paths, 4),
            vec![
                "AGENTS.md".to_string(),
                "alpha/one.rs".to_string(),
                "beta/one.rs".to_string(),
                "gamma/one.rs".to_string(),
            ]
        );
    }

    #[test]
    fn renders_lossy_expansion_digests_with_fallbacks() {
        let rg_json = render_rg_json_digest(
            "Exit code: 0\nWall time: 1s\nOutput:\n{\"type\":\"match\",\"data\":{\"path\":{\"text\":\"src/lib.rs\"},\"line_number\":7,\"submatches\":[{\"match\":{\"text\":\"replacement\"}}]}}\nnot-json\n",
        );
        assert!(rg_json.contains("rg_json_digest"));
        assert!(rg_json.contains("parse_errors: 1"));
        assert!(rg_json.contains("fallback_required: true"));
        assert!(rg_json.contains("fallback_reason: json_parse_error"));

        let filtered = render_git_filtered_diff_digest(
            "Exit code: 0\nWall time: 1s\nOutput:\n@@ -1 +1 @@\n+replacement_shadow\n",
        );
        assert!(filtered.contains("git_filtered_diff_digest"));
        assert!(filtered.contains("fallback_reason: lossy_diff_filter"));

        let check = render_run_check_digest(
            "Exit code: 0\nWall time: 1s\nOutput:\nwarning: slow check\nfinished\n",
        );
        assert!(check.contains("run_check_digest"));
        assert!(check.contains("fallback_required: true"));
        assert!(check.contains("fallback_reason: lossy_check_output"));

        let history = render_git_history_digest(
            "Exit code: 0\nWall time: 1s\nOutput:\ncommit abc\nAuthor: A\nDate: Today\n 1 file changed, 2 insertions(+)\n",
        );
        assert!(history.contains("git_history_digest"));
        assert!(history.contains("fallback_reason: lossy_git_history"));

        let listing = render_directory_listing_compact(
            "Exit code: 0\nWall time: 1s\nOutput:\n-a--- 10 lib.rs\n",
        );
        assert!(listing.contains("directory_listing_compact"));
        assert!(listing.contains("fallback_reason: lossy_directory_listing"));

        let processes = render_process_table_compact(
            "Exit code: 0\nWall time: 1s\nOutput:\nName Id CPU\ncodex 1 0.1\n",
        );
        assert!(processes.contains("process_table_compact"));
        assert!(processes.contains("fallback_reason: lossy_process_table"));
    }

    #[test]
    fn process_table_compact_keeps_interesting_rows_after_top_sample() {
        let mut output = String::from("Exit code: 0\nWall time: 1s\nOutput:\nName Id CPU\n");
        for index in 0..40 {
            output.push_str(&format!("chrome {index} 0.1\n"));
        }
        output.push_str("rustc 9001 80.0\n");
        output.push_str("codex 9002 5.0\n");

        let processes = render_process_table_compact(&output);

        assert!(processes.contains("rustc 9001"));
        assert!(processes.contains("codex 9002"));
        assert!(processes.contains("fallback_reason: max_processes"));
    }

    #[test]
    fn process_table_compact_keeps_small_generic_targeted_rows() {
        let processes = render_process_table_compact(
            "Exit code: 0\nWall time: 1s\nOutput:\nName Id CPU\nchrome 2301 120.0\nnode 2302 40.0\n",
        );

        assert!(processes.contains("chrome 2301"));
        assert!(processes.contains("node 2302"));
    }
}
