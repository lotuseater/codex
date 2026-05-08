use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tokio::process::Command;

use crate::ContextOpsError;

pub const DEFAULT_MAX_FILES: usize = 50;
const MAX_FILES: usize = 500;
pub const DEFAULT_MAX_MATCHES_PER_FILE: usize = 5;
const MAX_MATCHES_PER_FILE: usize = 50;
const MAX_RENDERED_LINE_CHARS: usize = 240;
const MAX_RENDERED_PATTERN_CHARS: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchMatch {
    path: String,
    line_number: u64,
    line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchFileMatches {
    path: String,
    matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchTextResult {
    files: Vec<SearchFileMatches>,
    files_omitted_lower_bound: usize,
}

pub async fn search_text(
    workdir: &Path,
    pattern: &str,
    globs: &[String],
    paths: &[String],
    max_files: usize,
    max_matches_per_file: usize,
) -> Result<String, ContextOpsError> {
    let max_files = clamp_max_files(max_files);
    let max_matches_per_file = clamp_max_matches_per_file(max_matches_per_file);
    let result = run_rg_search(
        workdir,
        pattern,
        globs,
        paths,
        max_files,
        max_matches_per_file,
    )
    .await?;
    Ok(render_search_text(
        workdir,
        pattern,
        globs,
        paths,
        max_matches_per_file,
        &result,
    ))
}

pub fn search_text_from_rg_json_output<'a>(
    workdir: &Path,
    pattern: &str,
    globs: &[String],
    paths: &[String],
    max_files: usize,
    max_matches_per_file: usize,
    lines: impl IntoIterator<Item = &'a str>,
) -> String {
    let max_files = clamp_max_files(max_files);
    let max_matches_per_file = clamp_max_matches_per_file(max_matches_per_file);
    let result = parse_rg_json_output(lines, max_files);
    render_search_text(
        workdir,
        pattern,
        globs,
        paths,
        max_matches_per_file,
        &result,
    )
}

async fn run_rg_search(
    workdir: &Path,
    pattern: &str,
    globs: &[String],
    paths: &[String],
    max_files: usize,
    max_matches_per_file: usize,
) -> Result<SearchTextResult, ContextOpsError> {
    let args = rg_args(pattern, globs, paths, max_matches_per_file + 1);
    let mut command = Command::new(&args[0]);
    command.current_dir(workdir).args(&args[1..]);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|err| ContextOpsError::new(format!("failed to run rg: {err}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ContextOpsError::new("failed to capture rg stdout".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ContextOpsError::new("failed to capture rg stderr".to_string()))?;
    let stderr_task = tokio::spawn(async move {
        let mut stderr = BufReader::new(stderr);
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text).await;
        text
    });

    let mut lines = BufReader::new(stdout).lines();
    let mut result = SearchTextResult {
        files: Vec::new(),
        files_omitted_lower_bound: 0,
    };
    let mut path_to_index = HashMap::new();
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|err| ContextOpsError::new(format!("failed to read rg: {err}")))?
    {
        let Some(search_match) = search_match_from_json_line(&line) else {
            continue;
        };
        if record_search_match(&mut result, &mut path_to_index, search_match, max_files) {
            continue;
        }
        result.files_omitted_lower_bound += 1;
        let _ = child.kill().await;
        break;
    }

    let status = child
        .wait()
        .await
        .map_err(|err| ContextOpsError::new(format!("failed to wait for rg: {err}")))?;
    let stderr = stderr_task.await.unwrap_or_default();
    if result.files_omitted_lower_bound == 0 && !status.success() && status.code() != Some(1) {
        let stderr = stderr.trim();
        let message = if stderr.is_empty() {
            format!("rg exited with status {status}")
        } else {
            stderr.to_string()
        };
        return Err(ContextOpsError::new(message));
    }

    Ok(result)
}

pub fn clamp_max_files(max_files: usize) -> usize {
    max_files.clamp(1, MAX_FILES)
}

pub fn clamp_max_matches_per_file(max_matches_per_file: usize) -> usize {
    max_matches_per_file.clamp(1, MAX_MATCHES_PER_FILE)
}

pub fn rg_args(
    pattern: &str,
    globs: &[String],
    paths: &[String],
    max_matches_per_file: usize,
) -> Vec<String> {
    let mut args = vec![
        "rg".to_string(),
        "--json".to_string(),
        "--color".to_string(),
        "never".to_string(),
        "--max-count".to_string(),
        max_matches_per_file.to_string(),
        "--max-columns".to_string(),
        MAX_RENDERED_LINE_CHARS.to_string(),
        "--max-columns-preview".to_string(),
    ];
    for glob in globs.iter().filter(|glob| !glob.is_empty()) {
        args.push("--glob".to_string());
        args.push(glob.to_string());
    }
    args.push("--".to_string());
    args.push(pattern.to_string());
    args.extend(paths.iter().filter(|path| !path.is_empty()).cloned());
    args
}

fn parse_rg_json_output<'a>(
    lines: impl IntoIterator<Item = &'a str>,
    max_files: usize,
) -> SearchTextResult {
    let mut result = SearchTextResult {
        files: Vec::new(),
        files_omitted_lower_bound: 0,
    };
    let mut path_to_index = HashMap::new();
    for line in lines {
        let Some(search_match) = search_match_from_json_line(line) else {
            continue;
        };
        if record_search_match(&mut result, &mut path_to_index, search_match, max_files) {
            continue;
        }
        result.files_omitted_lower_bound += 1;
        break;
    }
    result
}

fn search_match_from_json_line(line: &str) -> Option<SearchMatch> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "match" {
        return None;
    }
    let data = value.get("data")?;
    let path = data.get("path")?.get("text")?.as_str()?.replace('\\', "/");
    let line_number = data.get("line_number")?.as_u64()?;
    let line = data
        .get("lines")?
        .get("text")?
        .as_str()?
        .trim_end_matches(['\r', '\n'])
        .to_string();
    Some(SearchMatch {
        path,
        line_number,
        line,
    })
}

fn record_search_match(
    result: &mut SearchTextResult,
    path_to_index: &mut HashMap<String, usize>,
    search_match: SearchMatch,
    max_files: usize,
) -> bool {
    if let Some(index) = path_to_index.get(&search_match.path) {
        result.files[*index].matches.push(search_match);
        return true;
    }
    if result.files.len() >= max_files {
        return false;
    }

    let index = result.files.len();
    path_to_index.insert(search_match.path.clone(), index);
    result.files.push(SearchFileMatches {
        path: search_match.path.clone(),
        matches: vec![search_match],
    });
    true
}

fn render_search_text(
    workdir: &Path,
    pattern: &str,
    globs: &[String],
    paths: &[String],
    max_matches_per_file: usize,
    result: &SearchTextResult,
) -> String {
    let total_matches_shown = result
        .files
        .iter()
        .map(|file| file.matches.len().min(max_matches_per_file))
        .sum::<usize>();
    let matches_omitted_lower_bound = result
        .files
        .iter()
        .map(|file| file.matches.len().saturating_sub(max_matches_per_file))
        .sum::<usize>();
    let mut lines = vec![
        "search_text".to_string(),
        format!("workdir: {}", workdir.display()),
        format!(
            "pattern: {}",
            truncate_text(pattern, MAX_RENDERED_PATTERN_CHARS)
        ),
        format!("files: {}", result.files.len()),
        format!("matches: {total_matches_shown}"),
        format!("max_matches_per_file: {max_matches_per_file}"),
    ];
    if matches_omitted_lower_bound > 0 {
        lines.push(format!(
            "matches_omitted_lower_bound: {matches_omitted_lower_bound}"
        ));
    }
    if result.files_omitted_lower_bound > 0 {
        lines.push(format!(
            "files_omitted_lower_bound: {}",
            result.files_omitted_lower_bound
        ));
    }
    let rendered_globs = globs
        .iter()
        .filter(|glob| !glob.is_empty())
        .take(12)
        .cloned()
        .collect::<Vec<_>>();
    if !rendered_globs.is_empty() {
        let glob_count = globs.iter().filter(|glob| !glob.is_empty()).count();
        let omitted = glob_count.saturating_sub(rendered_globs.len());
        lines.push(format!(
            "globs: {} shown, {omitted} omitted: {}",
            rendered_globs.len(),
            rendered_globs.join(", ")
        ));
    }
    let fallback_reasons = search_fallback_reasons(result, max_matches_per_file);
    let rendered_paths = paths
        .iter()
        .filter(|path| !path.is_empty())
        .take(12)
        .cloned()
        .collect::<Vec<_>>();
    if !rendered_paths.is_empty() {
        let path_count = paths.iter().filter(|path| !path.is_empty()).count();
        let omitted = path_count.saturating_sub(rendered_paths.len());
        lines.push(format!(
            "paths: {} shown, {omitted} omitted: {}",
            rendered_paths.len(),
            rendered_paths.join(", ")
        ));
    }
    let path_prefix = common_path_prefix(&result.files);
    if !path_prefix.is_empty() {
        lines.push(format!("path_prefix: {path_prefix}"));
    }
    if !fallback_reasons.is_empty() {
        lines.push("fallback_required: true".to_string());
        lines.push(format!("fallback_reason: {}", fallback_reasons.join(",")));
    }
    if result.files.is_empty() {
        lines.push("status: no_matches".to_string());
        return lines.join("\n");
    }

    let repeated_groups = repeated_line_groups(result, max_matches_per_file, &path_prefix);
    let grouped_positions = repeated_groups
        .values()
        .flat_map(|positions| positions.iter().cloned())
        .collect::<BTreeSet<_>>();
    for (line, positions) in repeated_groups {
        lines.push(format!(
            "text: {}",
            truncate_text(&line, MAX_RENDERED_LINE_CHARS)
        ));
        lines.push(format!("at: {}", positions.join(", ")));
    }

    for file in &result.files {
        let ungrouped = file
            .matches
            .iter()
            .take(max_matches_per_file)
            .filter(|search_match| {
                !grouped_positions.contains(&format!(
                    "{}:L{}",
                    render_path(&file.path, &path_prefix),
                    search_match.line_number
                ))
            })
            .collect::<Vec<_>>();
        if ungrouped.is_empty() {
            continue;
        }
        if ungrouped.len() == 1 {
            let search_match = ungrouped[0];
            lines.push(format!(
                "{}:L{} {}",
                render_path(&file.path, &path_prefix),
                search_match.line_number,
                truncate_text(&search_match.line, MAX_RENDERED_LINE_CHARS)
            ));
            continue;
        }
        lines.push(format!("file: {}", render_path(&file.path, &path_prefix)));
        lines.extend(ungrouped.into_iter().map(|search_match| {
            format!(
                "L{} {}",
                search_match.line_number,
                truncate_text(&search_match.line, MAX_RENDERED_LINE_CHARS)
            )
        }));
    }

    lines.join("\n")
}

fn repeated_line_groups(
    result: &SearchTextResult,
    max_matches_per_file: usize,
    path_prefix: &str,
) -> BTreeMap<String, Vec<String>> {
    let mut groups = BTreeMap::<String, Vec<String>>::new();
    for file in &result.files {
        for search_match in file.matches.iter().take(max_matches_per_file) {
            let line = search_match.line.trim().to_string();
            groups.entry(line).or_default().push(format!(
                "{}:L{}",
                render_path(&file.path, path_prefix),
                search_match.line_number
            ));
        }
    }
    groups
        .into_iter()
        .filter(|(_, positions)| positions.len() >= 3)
        .collect()
}

fn common_path_prefix(files: &[SearchFileMatches]) -> String {
    let mut iter = files.iter().map(|file| file.path.as_str());
    let Some(first) = iter.next() else {
        return String::new();
    };
    let mut prefix = first.to_string();
    for path in iter {
        let mut byte_len = 0usize;
        for ((index, left), right) in prefix.char_indices().zip(path.chars()) {
            if left != right {
                break;
            }
            byte_len = index + left.len_utf8();
        }
        prefix.truncate(byte_len);
    }
    if let Some(index) = prefix.rfind('/') {
        prefix.truncate(index + 1);
    } else {
        prefix.clear();
    }
    if prefix.len() < 10 {
        return String::new();
    }
    prefix
}

fn render_path(path: &str, prefix: &str) -> String {
    path.strip_prefix(prefix).unwrap_or(path).to_string()
}

fn search_fallback_reasons(
    result: &SearchTextResult,
    max_matches_per_file: usize,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if result.files_omitted_lower_bound > 0 {
        reasons.push("max_files");
    }
    if result
        .files
        .iter()
        .any(|file| file.matches.len() > max_matches_per_file)
    {
        reasons.push("max_matches_per_file");
    }
    reasons
}

pub fn combined_globs(glob: Option<&str>, globs: &[String]) -> Vec<String> {
    glob.into_iter()
        .map(ToString::to_string)
        .chain(globs.iter().cloned())
        .filter(|glob| !glob.is_empty())
        .collect()
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= max_chars {
            output.push_str("...");
            return output;
        }
        output.push(ch);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_rg_json_match_lines() {
        let search_match = search_match_from_json_line(
            r#"{"type":"match","data":{"path":{"text":"src/lib.rs"},"line_number":7,"lines":{"text":"pub fn run() {}\n"}}}"#,
        )
        .expect("match");

        assert_eq!(
            search_match,
            SearchMatch {
                path: "src/lib.rs".to_string(),
                line_number: 7,
                line: "pub fn run() {}".to_string(),
            }
        );
    }

    #[test]
    fn record_search_match_caps_new_files() {
        let mut result = SearchTextResult {
            files: Vec::new(),
            files_omitted_lower_bound: 0,
        };
        let mut path_to_index = HashMap::new();

        assert!(record_search_match(
            &mut result,
            &mut path_to_index,
            SearchMatch {
                path: "a.rs".to_string(),
                line_number: 1,
                line: "a".to_string(),
            },
            1,
        ));
        assert!(!record_search_match(
            &mut result,
            &mut path_to_index,
            SearchMatch {
                path: "b.rs".to_string(),
                line_number: 1,
                line: "b".to_string(),
            },
            1,
        ));
    }

    #[test]
    fn rg_args_pass_paths_after_pattern() {
        assert_eq!(
            rg_args(
                "needle",
                &["*.rs".to_string(), "!target/**".to_string()],
                &["codex-rs/core".to_string(), "README.md".to_string()],
                5,
            ),
            vec![
                "rg",
                "--json",
                "--color",
                "never",
                "--max-count",
                "5",
                "--max-columns",
                "240",
                "--max-columns-preview",
                "--glob",
                "*.rs",
                "--glob",
                "!target/**",
                "--",
                "needle",
                "codex-rs/core",
                "README.md",
            ]
        );
    }

    #[test]
    fn render_does_not_mark_exact_match_cap_as_fallback() {
        let result = SearchTextResult {
            files: vec![SearchFileMatches {
                path: "src/lib.rs".to_string(),
                matches: vec![SearchMatch {
                    path: "src/lib.rs".to_string(),
                    line_number: 1,
                    line: "needle".to_string(),
                }],
            }],
            files_omitted_lower_bound: 0,
        };

        assert_eq!(
            render_search_text(
                Path::new("."),
                "needle",
                &["*.rs".to_string()],
                &["src".to_string()],
                1,
                &result,
            ),
            "search_text\nworkdir: .\npattern: needle\nfiles: 1\nmatches: 1\nmax_matches_per_file: 1\nglobs: 1 shown, 0 omitted: *.rs\npaths: 1 shown, 0 omitted: src\nsrc/lib.rs:L1 needle"
        );
    }

    #[test]
    fn render_marks_actual_file_and_match_caps_as_fallback() {
        let result = SearchTextResult {
            files: vec![SearchFileMatches {
                path: "src/lib.rs".to_string(),
                matches: vec![
                    SearchMatch {
                        path: "src/lib.rs".to_string(),
                        line_number: 1,
                        line: "needle".to_string(),
                    },
                    SearchMatch {
                        path: "src/lib.rs".to_string(),
                        line_number: 2,
                        line: "needle again".to_string(),
                    },
                ],
            }],
            files_omitted_lower_bound: 1,
        };

        assert_eq!(
            render_search_text(
                Path::new("."),
                "needle",
                &[],
                &["src".to_string()],
                1,
                &result,
            ),
            "search_text\nworkdir: .\npattern: needle\nfiles: 1\nmatches: 1\nmax_matches_per_file: 1\nmatches_omitted_lower_bound: 1\nfiles_omitted_lower_bound: 1\npaths: 1 shown, 0 omitted: src\nfallback_required: true\nfallback_reason: max_files,max_matches_per_file\nsrc/lib.rs:L1 needle"
        );
    }

    #[test]
    fn render_groups_repeated_lines_with_common_prefix() {
        let result = SearchTextResult {
            files: vec![
                repeated_file("src/a.rs", 3, "same line"),
                repeated_file("src/b.rs", 7, "same line"),
                repeated_file("src/c.rs", 9, "same line"),
            ],
            files_omitted_lower_bound: 0,
        };

        assert_eq!(
            render_search_text(Path::new("."), "same", &[], &[], 5, &result),
            "search_text\nworkdir: .\npattern: same\nfiles: 3\nmatches: 3\nmax_matches_per_file: 5\ntext: same line\nat: src/a.rs:L3, src/b.rs:L7, src/c.rs:L9"
        );
    }

    fn repeated_file(path: &str, line_number: u64, line: &str) -> SearchFileMatches {
        SearchFileMatches {
            path: path.to_string(),
            matches: vec![SearchMatch {
                path: path.to_string(),
                line_number,
                line: line.to_string(),
            }],
        }
    }
}
