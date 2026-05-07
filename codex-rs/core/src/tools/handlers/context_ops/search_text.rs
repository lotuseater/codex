use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tokio::process::Command;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::handlers::parse_arguments;

const DEFAULT_MAX_FILES: usize = 50;
const MAX_FILES: usize = 500;
const DEFAULT_MAX_MATCHES_PER_FILE: usize = 5;
const MAX_MATCHES_PER_FILE: usize = 50;
const MAX_RENDERED_LINE_CHARS: usize = 240;
const MAX_RENDERED_PATTERN_CHARS: usize = 120;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchTextArgs {
    pattern: String,
    #[serde(default)]
    workdir: Option<String>,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default)]
    max_files: Option<usize>,
    #[serde(default)]
    max_matches_per_file: Option<usize>,
}

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
    truncated: bool,
}

pub(super) async fn handle(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: SearchTextArgs = parse_arguments(arguments)?;
    if args.pattern.trim().is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "pattern must not be empty".to_string(),
        ));
    }

    let workdir = invocation.turn.resolve_path(args.workdir);
    let max_files = args
        .max_files
        .unwrap_or(DEFAULT_MAX_FILES)
        .clamp(1, MAX_FILES);
    let max_matches_per_file = args
        .max_matches_per_file
        .unwrap_or(DEFAULT_MAX_MATCHES_PER_FILE)
        .clamp(1, MAX_MATCHES_PER_FILE);
    let result = run_rg_search(
        workdir.as_path(),
        &args.pattern,
        args.glob.as_deref(),
        max_files,
        max_matches_per_file,
    )
    .await?;
    Ok(FunctionToolOutput::from_text(
        render_search_text(
            workdir.as_path(),
            &args.pattern,
            args.glob.as_deref(),
            max_matches_per_file,
            &result,
        ),
        Some(true),
    ))
}

pub(crate) async fn search_text(
    workdir: &Path,
    pattern: &str,
    glob: Option<&str>,
    max_files: usize,
    max_matches_per_file: usize,
) -> Result<String, FunctionCallError> {
    let max_files = max_files.clamp(1, MAX_FILES);
    let max_matches_per_file = max_matches_per_file.clamp(1, MAX_MATCHES_PER_FILE);
    let result = run_rg_search(workdir, pattern, glob, max_files, max_matches_per_file).await?;
    Ok(render_search_text(
        workdir,
        pattern,
        glob,
        max_matches_per_file,
        &result,
    ))
}

async fn run_rg_search(
    workdir: &Path,
    pattern: &str,
    glob: Option<&str>,
    max_files: usize,
    max_matches_per_file: usize,
) -> Result<SearchTextResult, FunctionCallError> {
    let mut command = Command::new("rg");
    command
        .current_dir(workdir)
        .arg("--json")
        .arg("--color")
        .arg("never")
        .arg("--max-count")
        .arg(max_matches_per_file.to_string())
        .arg("--max-columns")
        .arg(MAX_RENDERED_LINE_CHARS.to_string())
        .arg("--max-columns-preview");
    if let Some(glob) = glob.filter(|glob| !glob.is_empty()) {
        command.arg("--glob").arg(glob);
    }
    command.arg("--").arg(pattern);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|err| FunctionCallError::RespondToModel(format!("failed to run rg: {err}")))?;
    let stdout = child.stdout.take().ok_or_else(|| {
        FunctionCallError::RespondToModel("failed to capture rg stdout".to_string())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        FunctionCallError::RespondToModel("failed to capture rg stderr".to_string())
    })?;
    let stderr_task = tokio::spawn(async move {
        let mut stderr = BufReader::new(stderr);
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text).await;
        text
    });

    let mut lines = BufReader::new(stdout).lines();
    let mut result = SearchTextResult {
        files: Vec::new(),
        truncated: false,
    };
    let mut path_to_index = HashMap::new();
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|err| FunctionCallError::RespondToModel(format!("failed to read rg: {err}")))?
    {
        let Some(search_match) = search_match_from_json_line(&line) else {
            continue;
        };
        if record_search_match(&mut result, &mut path_to_index, search_match, max_files) {
            continue;
        }
        result.truncated = true;
        let _ = child.kill().await;
        break;
    }

    let status = child.wait().await.map_err(|err| {
        FunctionCallError::RespondToModel(format!("failed to wait for rg: {err}"))
    })?;
    let stderr = stderr_task.await.unwrap_or_default();
    if !result.truncated && !status.success() && status.code() != Some(1) {
        let stderr = stderr.trim();
        let message = if stderr.is_empty() {
            format!("rg exited with status {status}")
        } else {
            stderr.to_string()
        };
        return Err(FunctionCallError::RespondToModel(message));
    }

    Ok(result)
}

fn search_match_from_json_line(line: &str) -> Option<SearchMatch> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "match" {
        return None;
    }
    let data = value.get("data")?;
    let path = data.get("path")?.get("text")?.as_str()?.to_string();
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
    glob: Option<&str>,
    max_matches_per_file: usize,
    result: &SearchTextResult,
) -> String {
    let total_matches = result
        .files
        .iter()
        .map(|file| file.matches.len())
        .sum::<usize>();
    let mut lines = vec![
        "search_text".to_string(),
        format!("workdir: {}", workdir.display()),
        format!(
            "pattern: {}",
            truncate_text(pattern, MAX_RENDERED_PATTERN_CHARS)
        ),
        format!("files: {}", result.files.len()),
        format!("matches: {total_matches}"),
        format!("max_matches_per_file: {max_matches_per_file}"),
    ];
    if let Some(glob) = glob.filter(|glob| !glob.is_empty()) {
        lines.push(format!("glob: {glob}"));
    }
    if result.truncated {
        lines.push("fallback_required: true".to_string());
    }
    if result.files.is_empty() {
        lines.push("status: no_matches".to_string());
        return lines.join("\n");
    }

    for file in &result.files {
        lines.push(format!("file: {}", file.path));
        lines.extend(file.matches.iter().map(|search_match| {
            format!(
                "L{} {}",
                search_match.line_number,
                truncate_text(&search_match.line, MAX_RENDERED_LINE_CHARS)
            )
        }));
    }

    lines.join("\n")
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
            truncated: false,
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
}
