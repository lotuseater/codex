use chrono::Utc;
use codex_replacement_shadow::ReplacementCandidate;
use codex_replacement_shadow::classify_promoted_replacement;
use codex_replacement_shadow::classify_shell_replacement;
use codex_replacement_shadow::estimate_tokens;
use codex_replacement_shadow::render_replacement_output;
use codex_replacement_shadow::should_replace_model_output;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::time::Instant;

const DEFAULT_MAX_FILES: usize = 50;
const DEFAULT_MAX_MATCHES_PER_FILE: usize = 5;
const DEFAULT_MAX_OUTLINE_ITEMS: usize = 200;

pub(crate) struct ShellShadowRequest {
    pub(crate) tool_name: String,
    pub(crate) call_id: String,
    pub(crate) command: String,
    pub(crate) cwd: PathBuf,
    pub(crate) baseline_model_visible_output: String,
    pub(crate) log_dir: PathBuf,
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
    replacement_gate_passed: bool,
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

pub(crate) async fn maybe_compact_shell_output(
    command: &str,
    cwd: &Path,
    baseline_model_visible_output: &str,
) -> Option<String> {
    let candidate = classify_promoted_replacement(command)?;
    let candidate_output = run_candidate(&candidate, cwd, baseline_model_visible_output)
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
    let candidate_output = run_candidate(
        &candidate,
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
        replacement_gate_passed,
    ) = match candidate_output {
        Ok(output) => {
            let fallback_required = output.contains("fallback_required: true");
            let rendered_output =
                render_replacement_output(&request.command, candidate_name, &output);
            let replacement_gate_passed = !fallback_required
                && should_replace_model_output(
                    &request.baseline_model_visible_output,
                    &rendered_output,
                );
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
                replacement_gate_passed,
            )
        }
        Err(err) => (None, Some(err), None, None, None, false),
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
        replacement_gate_passed,
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

async fn run_candidate(
    candidate: &ReplacementCandidate,
    cwd: &Path,
    baseline_model_visible_output: &str,
) -> Result<String, String> {
    match candidate {
        ReplacementCandidate::SearchText {
            pattern,
            globs,
            paths,
        } => codex_context_ops_impl::search_text(
            cwd,
            pattern,
            globs,
            paths,
            DEFAULT_MAX_FILES,
            DEFAULT_MAX_MATCHES_PER_FILE,
        )
        .await
        .map_err(|err| err.to_string()),
        ReplacementCandidate::FileOutline { path } => {
            let path = resolve_optional_workdir(cwd, Some(path.as_path()));
            codex_context_ops_impl::file_outline(path.as_path(), DEFAULT_MAX_OUTLINE_ITEMS)
                .await
                .map_err(|err| err.to_string())
        }
        _ => candidate
            .render_baseline_digest(baseline_model_visible_output)
            .ok_or_else(|| format!("{} cannot render from baseline", candidate.name())),
    }
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
