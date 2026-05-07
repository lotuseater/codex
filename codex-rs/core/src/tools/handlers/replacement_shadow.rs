use chrono::Utc;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::time::Instant;

use super::context_ops;

const DEFAULT_GIT_LIMIT: usize = 80;
const DEFAULT_MAX_FILES: usize = 50;
const DEFAULT_MAX_MATCHES_PER_FILE: usize = 5;
const DEFAULT_MAX_OUTLINE_ITEMS: usize = 200;
const SHELL_CONTROL_MARKERS: &[&str] = &["\n", "\r", "&&", "||", "|", ";", ">", "<", "$("];

pub(crate) struct ShellShadowRequest {
    pub(crate) tool_name: String,
    pub(crate) call_id: String,
    pub(crate) command: String,
    pub(crate) cwd: PathBuf,
    pub(crate) baseline_model_visible_output: String,
    pub(crate) log_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplacementCandidate {
    GitWorktreeSummary {
        workdir: Option<PathBuf>,
    },
    SearchText {
        workdir: Option<PathBuf>,
        pattern: String,
        glob: Option<String>,
    },
    FileOutline {
        path: PathBuf,
    },
}

impl ReplacementCandidate {
    fn name(&self) -> &'static str {
        match self {
            Self::GitWorktreeSummary { .. } => "git_worktree_summary",
            Self::SearchText { .. } => "search_text",
            Self::FileOutline { .. } => "file_outline",
        }
    }

    async fn run(&self, cwd: &Path) -> Result<String, String> {
        match self {
            Self::GitWorktreeSummary { workdir } => {
                let workdir = resolve_optional_workdir(cwd, workdir.as_deref());
                context_ops::git_worktree_summary::git_worktree_summary(
                    workdir.as_path(),
                    DEFAULT_GIT_LIMIT,
                )
                .await
                .map_err(|err| err.to_string())
            }
            Self::SearchText {
                workdir,
                pattern,
                glob,
            } => {
                let workdir = resolve_optional_workdir(cwd, workdir.as_deref());
                context_ops::search_text::search_text(
                    workdir.as_path(),
                    pattern,
                    glob.as_deref(),
                    DEFAULT_MAX_FILES,
                    DEFAULT_MAX_MATCHES_PER_FILE,
                )
                .await
                .map_err(|err| err.to_string())
            }
            Self::FileOutline { path } => {
                let path = resolve_optional_workdir(cwd, Some(path.as_path()));
                context_ops::file_outline::file_outline(path.as_path(), DEFAULT_MAX_OUTLINE_ITEMS)
                    .await
                    .map_err(|err| err.to_string())
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct ReplacementBenchRecord {
    r#type: &'static str,
    mode: &'static str,
    timestamp: String,
    tool_name: String,
    call_id: String,
    cwd: String,
    baseline_command: String,
    replacement_operation: &'static str,
    baseline_model_visible_bytes: usize,
    replacement_model_visible_bytes: Option<usize>,
    baseline_model_visible_tokens: usize,
    replacement_model_visible_tokens: Option<usize>,
    replacement_fallback_required: Option<bool>,
    saved_model_visible_tokens: Option<isize>,
    saved_model_visible_percent: Option<f64>,
    wall_time_ms: u128,
    baseline_artifact_path: String,
    replacement_artifact_path: Option<String>,
    replacement_error: Option<String>,
    verdict: &'static str,
    fallback_reason: Option<&'static str>,
}

pub(crate) fn maybe_spawn_shell_shadow(request: ShellShadowRequest) {
    let Some(candidate) = classify_shell_replacement(&request.command) else {
        return;
    };

    tokio::spawn(async move {
        if let Err(err) = run_shell_shadow(request, candidate).await {
            tracing::debug!("context ops shadow failed: {err}");
        }
    });
}

async fn run_shell_shadow(
    request: ShellShadowRequest,
    candidate: ReplacementCandidate,
) -> Result<(), String> {
    let started = Instant::now();
    let shadow_dir = request.log_dir.join("replacement-shadow");
    let artifact_dir = shadow_dir.join("artifacts");
    tokio::fs::create_dir_all(&artifact_dir)
        .await
        .map_err(|err| format!("failed to create shadow artifact dir: {err}"))?;

    let stamp = Utc::now().format("%Y%m%dT%H%M%S%3fZ").to_string();
    let safe_call_id = sanitize_for_filename(&request.call_id);
    let baseline_artifact = artifact_dir.join(format!("{stamp}-{safe_call_id}-baseline.txt"));
    tokio::fs::write(&baseline_artifact, &request.baseline_model_visible_output)
        .await
        .map_err(|err| format!("failed to write shadow baseline artifact: {err}"))?;

    let baseline_tokens = estimate_tokens(&request.baseline_model_visible_output);
    let candidate_name = candidate.name();
    let candidate_output = candidate.run(request.cwd.as_path()).await;
    let wall_time_ms = started.elapsed().as_millis();
    let (
        replacement_artifact_path,
        replacement_error,
        replacement_bytes,
        replacement_tokens,
        replacement_fallback_required,
    ) = match candidate_output {
        Ok(output) => {
            let fallback_required = output.contains("fallback_required: true");
            let replacement_artifact =
                artifact_dir.join(format!("{stamp}-{safe_call_id}-{candidate_name}.txt"));
            tokio::fs::write(&replacement_artifact, &output)
                .await
                .map_err(|err| format!("failed to write shadow candidate artifact: {err}"))?;
            (
                Some(replacement_artifact.to_string_lossy().into_owned()),
                None,
                Some(output.len()),
                Some(estimate_tokens(&output)),
                Some(fallback_required),
            )
        }
        Err(err) => (None, Some(err), None, None, None),
    };

    let saved_tokens = replacement_tokens.map(|tokens| baseline_tokens as isize - tokens as isize);
    let saved_percent = replacement_tokens.and_then(|tokens| {
        (baseline_tokens > 0).then_some(
            ((baseline_tokens as isize - tokens as isize) as f64 / baseline_tokens as f64) * 100.0,
        )
    });
    let (verdict, fallback_reason) = match (
        replacement_error.as_ref(),
        replacement_fallback_required,
        saved_tokens,
    ) {
        (Some(_), _, _) => ("fallback_required", Some("replacement_error")),
        (None, Some(true), _) => (
            "fallback_required",
            Some("candidate_marked_fallback_required"),
        ),
        (None, _, Some(saved)) if saved > 0 => ("pass", None),
        (None, _, Some(_)) => ("fail_tokens", Some("no_token_savings")),
        (None, _, None) => ("needs_human_review", Some("missing_replacement_tokens")),
    };

    let record = ReplacementBenchRecord {
        r#type: "replacement_bench",
        mode: "shadow",
        timestamp: Utc::now().to_rfc3339(),
        tool_name: request.tool_name,
        call_id: request.call_id,
        cwd: request.cwd.to_string_lossy().into_owned(),
        baseline_command: request.command,
        replacement_operation: candidate_name,
        baseline_model_visible_bytes: request.baseline_model_visible_output.len(),
        replacement_model_visible_bytes: replacement_bytes,
        baseline_model_visible_tokens: baseline_tokens,
        replacement_model_visible_tokens: replacement_tokens,
        replacement_fallback_required,
        saved_model_visible_tokens: saved_tokens,
        saved_model_visible_percent: saved_percent,
        wall_time_ms,
        baseline_artifact_path: baseline_artifact.to_string_lossy().into_owned(),
        replacement_artifact_path,
        replacement_error,
        verdict,
        fallback_reason,
    };

    append_jsonl(
        shadow_dir
            .join(format!(
                "replacement-bench-{}.jsonl",
                Utc::now().format("%Y%m%d")
            ))
            .as_path(),
        &record,
    )
    .await
}

async fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
        .map_err(|err| format!("failed to open shadow bench log: {err}"))?;
    let line = serde_json::to_string(value)
        .map_err(|err| format!("failed to serialize shadow bench record: {err}"))?;
    file.write_all(line.as_bytes())
        .await
        .map_err(|err| format!("failed to write shadow bench record: {err}"))?;
    file.write_all(b"\n")
        .await
        .map_err(|err| format!("failed to finish shadow bench record: {err}"))
}

fn classify_shell_replacement(command: &str) -> Option<ReplacementCandidate> {
    if has_shell_control(command) {
        return None;
    }
    let tokens = shell_tokens(command)?;
    classify_git_candidate(&tokens)
        .or_else(|| classify_rg_candidate(&tokens))
        .or_else(|| classify_file_outline_candidate(&tokens))
}

fn has_shell_control(command: &str) -> bool {
    SHELL_CONTROL_MARKERS
        .iter()
        .any(|marker| command.contains(marker))
}

fn shell_tokens(command: &str) -> Option<Vec<String>> {
    let tokens = if command.contains('\\') {
        command
            .split_whitespace()
            .map(ToString::to_string)
            .collect()
    } else {
        shlex::split(command).unwrap_or_else(|| {
            command
                .split_whitespace()
                .map(ToString::to_string)
                .collect()
        })
    };
    (!tokens.is_empty()).then_some(tokens)
}

fn classify_git_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    if tokens.first()? != "git" {
        return None;
    }

    let mut index = 1;
    let mut workdir = None;
    if tokens.get(index).is_some_and(|token| token == "-C") {
        workdir = tokens.get(index + 1).map(PathBuf::from);
        index += 2;
    }

    match tokens.get(index).map(String::as_str)? {
        "status" if git_status_args_are_shadowable(&tokens[index + 1..]) => {
            Some(ReplacementCandidate::GitWorktreeSummary { workdir })
        }
        "diff" if git_diff_args_are_shadowable(&tokens[index + 1..]) => {
            Some(ReplacementCandidate::GitWorktreeSummary { workdir })
        }
        _ => None,
    }
}

fn git_status_args_are_shadowable(args: &[String]) -> bool {
    !args.is_empty()
        && args.iter().all(|arg| {
            matches!(
                arg.as_str(),
                "--short"
                    | "-s"
                    | "--branch"
                    | "-b"
                    | "--porcelain"
                    | "--porcelain=v1"
                    | "--untracked-files=all"
                    | "-uall"
            )
        })
}

fn git_diff_args_are_shadowable(args: &[String]) -> bool {
    !args.is_empty()
        && args.iter().all(|arg| {
            matches!(
                arg.as_str(),
                "--stat" | "--shortstat" | "--name-only" | "--cached" | "--staged" | "--" | "."
            )
        })
        && args
            .iter()
            .any(|arg| matches!(arg.as_str(), "--stat" | "--shortstat" | "--name-only"))
}

fn classify_rg_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    if tokens.first()? != "rg" {
        return None;
    }

    let mut pattern = None;
    let mut workdir = None;
    let mut glob = None;
    let mut index = 1;
    while let Some(token) = tokens.get(index) {
        if token == "--" {
            index += 1;
            pattern = tokens.get(index).cloned();
            workdir = tokens.get(index + 1).map(PathBuf::from);
            break;
        }
        if pattern.is_none() && token.starts_with('-') {
            match token.as_str() {
                "--files"
                | "-l"
                | "--files-with-matches"
                | "--files-without-match"
                | "--count"
                | "--replace"
                | "--json"
                | "-A"
                | "-B"
                | "-C"
                | "--after-context"
                | "--before-context"
                | "--context" => return None,
                "-g" | "--glob" => {
                    index += 1;
                    glob = tokens.get(index).cloned();
                }
                "-e" | "--regexp" => {
                    index += 1;
                    pattern = tokens.get(index).cloned();
                }
                "-m" | "--max-count" | "--max-columns" | "--type" | "-t" => {
                    index += 1;
                }
                _ if token.starts_with("--glob=") => {
                    glob = token.strip_prefix("--glob=").map(ToString::to_string);
                }
                _ if token.starts_with("--max-count=")
                    || token.starts_with("--max-columns=")
                    || token.starts_with("--type=") => {}
                _ => {}
            }
        } else if pattern.is_none() {
            pattern = Some(token.clone());
        } else if workdir.is_none() {
            workdir = Some(PathBuf::from(token));
        } else {
            return None;
        }
        index += 1;
    }

    pattern.and_then(|pattern| {
        (!pattern.trim().is_empty()).then_some(ReplacementCandidate::SearchText {
            workdir,
            pattern,
            glob,
        })
    })
}

fn classify_file_outline_candidate(tokens: &[String]) -> Option<ReplacementCandidate> {
    let command = tokens.first()?.to_ascii_lowercase();
    if !matches!(
        command.as_str(),
        "cat" | "type" | "gc" | "get-content" | "get-content.exe"
    ) {
        return None;
    }

    let mut path = None;
    let mut index = 1;
    while let Some(token) = tokens.get(index) {
        let lower = token.to_ascii_lowercase();
        match lower.as_str() {
            "-tail" | "-totalcount" | "-first" | "-last" | "-head" => return None,
            "-path" | "-literalpath" => {
                index += 1;
                path = tokens.get(index).cloned();
            }
            _ if token.starts_with('-') => {}
            _ if path.is_none() => path = Some(token.clone()),
            _ => return None,
        }
        index += 1;
    }

    path.and_then(|path| {
        (!path.contains('*') && !path.contains('?') && !path.contains('[')).then_some(
            ReplacementCandidate::FileOutline {
                path: PathBuf::from(path),
            },
        )
    })
}

fn resolve_optional_workdir(cwd: &Path, workdir: Option<&Path>) -> PathBuf {
    match workdir {
        Some(workdir) if workdir.is_absolute() => workdir.to_path_buf(),
        Some(workdir) => cwd.join(workdir),
        None => cwd.to_path_buf(),
    }
}

fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4).max(1)
}

fn sanitize_for_filename(value: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.chars().take(64) {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        "call".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn classifies_git_summary_commands() {
        assert_eq!(
            classify_shell_replacement("git status --short"),
            Some(ReplacementCandidate::GitWorktreeSummary { workdir: None })
        );
        assert_eq!(
            classify_shell_replacement("git -C codex-rs diff --stat"),
            Some(ReplacementCandidate::GitWorktreeSummary {
                workdir: Some(PathBuf::from("codex-rs"))
            })
        );
    }

    #[test]
    fn classifies_rg_search_without_shell_control() {
        assert_eq!(
            classify_shell_replacement("rg -n --glob '*.rs' context_ops codex-rs/core"),
            Some(ReplacementCandidate::SearchText {
                workdir: Some(PathBuf::from("codex-rs/core")),
                pattern: "context_ops".to_string(),
                glob: Some("*.rs".to_string())
            })
        );
        assert_eq!(classify_shell_replacement("rg --files | head -n 20"), None);
    }

    #[test]
    fn classifies_whole_file_reads_only() {
        assert_eq!(
            classify_shell_replacement("Get-Content -Path codex-rs/core/src/tools/mod.rs"),
            Some(ReplacementCandidate::FileOutline {
                path: PathBuf::from("codex-rs/core/src/tools/mod.rs")
            })
        );
        assert_eq!(
            classify_shell_replacement(
                "Get-Content -Path codex-rs/core/src/tools/mod.rs -TotalCount 40"
            ),
            None
        );
        assert_eq!(
            classify_shell_replacement(r"Get-Content -Path codex-rs\core\src\tools\mod.rs"),
            Some(ReplacementCandidate::FileOutline {
                path: PathBuf::from(r"codex-rs\core\src\tools\mod.rs")
            })
        );
    }

    #[test]
    fn estimates_tokens_without_zero() {
        assert_eq!(estimate_tokens(""), 1);
        assert_eq!(estimate_tokens("12345"), 2);
    }
}
