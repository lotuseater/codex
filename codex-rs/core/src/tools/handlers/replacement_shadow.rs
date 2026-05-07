use chrono::Utc;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::time::Instant;

use super::context_ops;

mod baseline_digest;
mod classify;

const DEFAULT_GIT_LIMIT: usize = 80;
const DEFAULT_MAX_FILES: usize = 50;
const DEFAULT_MAX_MATCHES_PER_FILE: usize = 5;
const DEFAULT_MAX_OUTLINE_ITEMS: usize = 200;
const MIN_REPLACE_SAVED_PERCENT: f64 = 30.0;
const MIN_REPLACE_SAVED_TOKENS: isize = 32;

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
    GitDiffStatCompact,
    GitStatusCompact,
    GitChangedFiles,
    RgFilesCompact,
    DiffHunkSummary,
    RunCheckDigest,
    FileExcerptDigest,
    SelectStringDigest,
    RgCountDigest,
    RgFileSetDigest,
    RgJsonDigest,
    GitNameStatusCompact,
    GitNumstatCompact,
    GitFilteredDiffDigest,
    GitHistoryDigest,
    DirectoryListingCompact,
    ProcessTableCompact,
    SearchText {
        pattern: String,
        glob: Option<String>,
        paths: Vec<String>,
    },
    FileOutline {
        path: PathBuf,
    },
}

impl ReplacementCandidate {
    fn name(&self) -> &'static str {
        match self {
            Self::GitWorktreeSummary { .. } => "git_worktree_summary",
            Self::GitDiffStatCompact => "git_diffstat_compact",
            Self::GitStatusCompact => "git_status_compact",
            Self::GitChangedFiles => "git_changed_files",
            Self::RgFilesCompact => "rg_files_compact",
            Self::DiffHunkSummary => "diff_hunk_summary",
            Self::RunCheckDigest => "run_check_digest",
            Self::FileExcerptDigest => "file_excerpt_digest",
            Self::SelectStringDigest => "select_string_digest",
            Self::RgCountDigest => "rg_count_digest",
            Self::RgFileSetDigest => "rg_file_set_digest",
            Self::RgJsonDigest => "rg_json_digest",
            Self::GitNameStatusCompact => "git_name_status_compact",
            Self::GitNumstatCompact => "git_numstat_compact",
            Self::GitFilteredDiffDigest => "git_filtered_diff_digest",
            Self::GitHistoryDigest => "git_history_digest",
            Self::DirectoryListingCompact => "directory_listing_compact",
            Self::ProcessTableCompact => "process_table_compact",
            Self::SearchText { .. } => "search_text",
            Self::FileOutline { .. } => "file_outline",
        }
    }

    fn strategy(&self) -> &'static str {
        match self {
            Self::GitWorktreeSummary { .. }
            | Self::SearchText { .. }
            | Self::FileOutline { .. } => "context_op_rerun",
            Self::GitDiffStatCompact
            | Self::GitStatusCompact
            | Self::GitChangedFiles
            | Self::RgFilesCompact
            | Self::DiffHunkSummary
            | Self::RunCheckDigest
            | Self::FileExcerptDigest
            | Self::SelectStringDigest
            | Self::RgCountDigest
            | Self::RgFileSetDigest
            | Self::RgJsonDigest
            | Self::GitNameStatusCompact
            | Self::GitNumstatCompact
            | Self::GitFilteredDiffDigest
            | Self::GitHistoryDigest
            | Self::DirectoryListingCompact
            | Self::ProcessTableCompact => "baseline_digest",
        }
    }

    async fn run(&self, cwd: &Path, baseline_model_visible_output: &str) -> Result<String, String> {
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
            Self::GitStatusCompact => Ok(baseline_digest::render_git_status_compact(
                baseline_model_visible_output,
            )),
            Self::GitDiffStatCompact => Ok(baseline_digest::render_git_diffstat_compact(
                baseline_model_visible_output,
            )),
            Self::GitChangedFiles => Ok(baseline_digest::render_changed_files_compact(
                "git_changed_files",
                baseline_model_visible_output,
            )),
            Self::RgFilesCompact => Ok(baseline_digest::render_changed_files_compact(
                "rg_files_compact",
                baseline_model_visible_output,
            )),
            Self::DiffHunkSummary => Ok(baseline_digest::render_diff_hunk_summary(
                baseline_model_visible_output,
            )),
            Self::RunCheckDigest => Ok(baseline_digest::render_run_check_digest(
                baseline_model_visible_output,
            )),
            Self::FileExcerptDigest => Ok(baseline_digest::render_file_excerpt_digest(
                baseline_model_visible_output,
            )),
            Self::SelectStringDigest => Ok(baseline_digest::render_select_string_digest(
                baseline_model_visible_output,
            )),
            Self::RgCountDigest => Ok(baseline_digest::render_rg_count_digest(
                baseline_model_visible_output,
            )),
            Self::RgFileSetDigest => Ok(baseline_digest::render_changed_files_compact(
                "rg_file_set_digest",
                baseline_model_visible_output,
            )),
            Self::RgJsonDigest => Ok(baseline_digest::render_rg_json_digest(
                baseline_model_visible_output,
            )),
            Self::GitNameStatusCompact => Ok(baseline_digest::render_git_name_status_compact(
                baseline_model_visible_output,
            )),
            Self::GitNumstatCompact => Ok(baseline_digest::render_git_numstat_compact(
                baseline_model_visible_output,
            )),
            Self::GitFilteredDiffDigest => Ok(baseline_digest::render_git_filtered_diff_digest(
                baseline_model_visible_output,
            )),
            Self::GitHistoryDigest => Ok(baseline_digest::render_git_history_digest(
                baseline_model_visible_output,
            )),
            Self::DirectoryListingCompact => Ok(baseline_digest::render_directory_listing_compact(
                baseline_model_visible_output,
            )),
            Self::ProcessTableCompact => Ok(baseline_digest::render_process_table_compact(
                baseline_model_visible_output,
            )),
            Self::SearchText {
                pattern,
                glob,
                paths,
            } => context_ops::search_text::search_text(
                cwd,
                pattern,
                glob.as_deref(),
                paths,
                DEFAULT_MAX_FILES,
                DEFAULT_MAX_MATCHES_PER_FILE,
            )
            .await
            .map_err(|err| err.to_string()),
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
    shadow_strategy: &'static str,
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
    let Some(candidate) = classify::classify_shell_replacement(&request.command) else {
        return;
    };

    tokio::spawn(async move {
        if let Err(err) = run_shell_shadow(request, candidate).await {
            tracing::debug!("context ops shadow failed: {err}");
        }
    });
}

pub(crate) async fn maybe_compact_shell_output(
    command: &str,
    cwd: &Path,
    baseline_model_visible_output: &str,
) -> Option<String> {
    let candidate = classify::classify_promoted_replacement(command)?;
    let candidate_output = candidate
        .run(cwd, baseline_model_visible_output)
        .await
        .ok()?;
    if candidate_output.contains("fallback_required: true") {
        return None;
    }

    let replacement_model_visible_output =
        render_replacement_output(command, candidate.name(), &candidate_output);
    if should_replace_model_output(
        baseline_model_visible_output,
        &replacement_model_visible_output,
    ) {
        Some(replacement_model_visible_output)
    } else {
        None
    }
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
    let candidate_output = candidate
        .run(
            request.cwd.as_path(),
            request.baseline_model_visible_output.as_str(),
        )
        .await;
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
        shadow_strategy: candidate.strategy(),
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

fn render_replacement_output(command: &str, operation: &str, replacement_output: &str) -> String {
    format!(
        "context_ops_replace: {operation}\nraw_command: {command}\nraw_output: omitted; rerun the raw command if exact output is needed.\n{replacement_output}"
    )
}

fn should_replace_model_output(
    baseline_model_visible_output: &str,
    replacement_model_visible_output: &str,
) -> bool {
    let baseline_tokens = estimate_tokens(baseline_model_visible_output);
    let replacement_tokens = estimate_tokens(replacement_model_visible_output);
    let saved_tokens = baseline_tokens as isize - replacement_tokens as isize;
    if saved_tokens < MIN_REPLACE_SAVED_TOKENS {
        return false;
    }
    let saved_percent = saved_tokens as f64 / baseline_tokens as f64 * 100.0;
    saved_percent >= MIN_REPLACE_SAVED_PERCENT
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
    fn estimates_tokens_without_zero() {
        assert_eq!(estimate_tokens(""), 1);
        assert_eq!(estimate_tokens("12345"), 2);
    }

    #[test]
    fn replacement_requires_meaningful_token_savings() {
        assert!(should_replace_model_output(
            "x".repeat(1_000).as_str(),
            "context_ops_replace\nsmall"
        ));
        assert!(!should_replace_model_output("tiny", "larger replacement"));
        assert!(!should_replace_model_output(
            "x".repeat(1_000).as_str(),
            "y".repeat(760).as_str()
        ));
    }

    #[test]
    fn reports_shadow_strategy_for_rerun_and_baseline_candidates() {
        assert_eq!(
            ReplacementCandidate::GitWorktreeSummary { workdir: None }.strategy(),
            "context_op_rerun"
        );
        assert_eq!(
            ReplacementCandidate::GitStatusCompact.strategy(),
            "baseline_digest"
        );
        assert_eq!(
            ReplacementCandidate::GitDiffStatCompact.strategy(),
            "baseline_digest"
        );
        assert_eq!(
            ReplacementCandidate::GitFilteredDiffDigest.strategy(),
            "baseline_digest"
        );
    }
}
