use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use tokio::process::Command;

use crate::function_tool::FunctionCallError;
use crate::session::turn_context::TurnEnvironment;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::handlers::parse_arguments;

use super::execution;

const DEFAULT_GIT_LIMIT: usize = 80;
const MAX_GIT_LIMIT: usize = 500;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GitWorktreeSummaryArgs {
    #[serde(default)]
    workdir: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitStatusEntry {
    status: String,
    path: String,
}

pub(super) async fn handle(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: GitWorktreeSummaryArgs = parse_arguments(arguments)?;
    let turn_environment = execution::primary_environment(&invocation)?;
    let workdir = execution::resolve_workdir(turn_environment, args.workdir.as_deref());
    let limit = args
        .limit
        .unwrap_or(DEFAULT_GIT_LIMIT)
        .clamp(1, MAX_GIT_LIMIT);
    let output =
        git_worktree_summary_in_environment(&invocation, turn_environment, &workdir, limit).await?;
    Ok(FunctionToolOutput::from_text(output, Some(true)))
}

pub(crate) async fn git_worktree_summary(
    workdir: &Path,
    limit: usize,
) -> Result<String, FunctionCallError> {
    let limit = limit.clamp(1, MAX_GIT_LIMIT);
    let repo_root = run_git(workdir, &["rev-parse", "--show-toplevel"]).await?;
    let branch = run_git(workdir, &["branch", "--show-current"])
        .await
        .unwrap_or_default();
    let status_raw = run_git_bytes(workdir, &["status", "--porcelain=v1", "-z"]).await?;
    let status_entries = parse_git_status_z(&status_raw);
    let shortstat = run_git(workdir, &["diff", "--shortstat"])
        .await
        .unwrap_or_default();
    let staged_shortstat = run_git(workdir, &["diff", "--cached", "--shortstat"])
        .await
        .unwrap_or_default();

    Ok(render_git_worktree_summary(
        workdir,
        repo_root.trim(),
        branch.trim(),
        shortstat.trim(),
        staged_shortstat.trim(),
        &status_entries,
        limit,
    ))
}

async fn git_worktree_summary_in_environment(
    invocation: &ToolInvocation,
    turn_environment: &TurnEnvironment,
    workdir: &codex_utils_absolute_path::AbsolutePathBuf,
    limit: usize,
) -> Result<String, FunctionCallError> {
    let limit = limit.clamp(1, MAX_GIT_LIMIT);
    let repo_root = run_git_in_environment(
        invocation,
        turn_environment,
        workdir,
        &["rev-parse", "--show-toplevel"],
    )
    .await?;
    let branch = run_git_in_environment(
        invocation,
        turn_environment,
        workdir,
        &["branch", "--show-current"],
    )
    .await
    .unwrap_or_default();
    let status_raw = run_git_bytes_in_environment(
        invocation,
        turn_environment,
        workdir,
        &["status", "--porcelain=v1", "-z"],
    )
    .await?;
    let status_entries = parse_git_status_z(&status_raw);
    let shortstat = run_git_in_environment(
        invocation,
        turn_environment,
        workdir,
        &["diff", "--shortstat"],
    )
    .await
    .unwrap_or_default();
    let staged_shortstat = run_git_in_environment(
        invocation,
        turn_environment,
        workdir,
        &["diff", "--cached", "--shortstat"],
    )
    .await
    .unwrap_or_default();

    Ok(render_git_worktree_summary(
        workdir.as_path(),
        repo_root.trim(),
        branch.trim(),
        shortstat.trim(),
        staged_shortstat.trim(),
        &status_entries,
        limit,
    ))
}

async fn run_git_in_environment(
    invocation: &ToolInvocation,
    turn_environment: &TurnEnvironment,
    workdir: &codex_utils_absolute_path::AbsolutePathBuf,
    args: &[&str],
) -> Result<String, FunctionCallError> {
    let output = run_git_bytes_in_environment(invocation, turn_environment, workdir, args).await?;
    Ok(String::from_utf8_lossy(&output).to_string())
}

async fn run_git_bytes_in_environment(
    invocation: &ToolInvocation,
    turn_environment: &TurnEnvironment,
    workdir: &codex_utils_absolute_path::AbsolutePathBuf,
    args: &[&str],
) -> Result<Vec<u8>, FunctionCallError> {
    let mut command = vec!["git".to_string()];
    command.extend(args.iter().map(|arg| arg.to_string()));
    let output = execution::run_command(invocation, turn_environment, workdir, command).await?;
    if output.timed_out {
        return Err(FunctionCallError::RespondToModel(
            "git timed out while inspecting the worktree".to_string(),
        ));
    }
    if output.exit_code != 0 {
        let stderr = output.stderr_text();
        let message = if stderr.is_empty() {
            format!("git exited with status {}", output.exit_code)
        } else {
            stderr
        };
        return Err(FunctionCallError::RespondToModel(message));
    }
    Ok(output.stdout)
}

async fn run_git(workdir: &Path, args: &[&str]) -> Result<String, FunctionCallError> {
    let output = run_git_bytes(workdir, args).await?;
    Ok(String::from_utf8_lossy(&output).to_string())
}

async fn run_git_bytes(workdir: &Path, args: &[&str]) -> Result<Vec<u8>, FunctionCallError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(args)
        .output()
        .await
        .map_err(|err| FunctionCallError::RespondToModel(format!("failed to run git: {err}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("git exited with status {}", output.status)
        } else {
            stderr
        };
        return Err(FunctionCallError::RespondToModel(message));
    }
    Ok(output.stdout)
}

fn parse_git_status_z(bytes: &[u8]) -> Vec<GitStatusEntry> {
    let mut entries = Vec::new();
    let mut fields = bytes
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    while let Some(field) = fields.next() {
        if field.len() < 4 {
            continue;
        }
        let status = String::from_utf8_lossy(&field[..2]).to_string();
        let path = String::from_utf8_lossy(&field[3..]).to_string();
        if status.contains('R') || status.contains('C') {
            let previous = fields
                .next()
                .map(|field| String::from_utf8_lossy(field).to_string());
            let path = if let Some(previous) = previous {
                format!("{previous} -> {path}")
            } else {
                path
            };
            entries.push(GitStatusEntry { status, path });
        } else {
            entries.push(GitStatusEntry { status, path });
        }
    }
    entries
}

fn render_git_worktree_summary(
    workdir: &Path,
    repo_root: &str,
    branch: &str,
    shortstat: &str,
    staged_shortstat: &str,
    entries: &[GitStatusEntry],
    limit: usize,
) -> String {
    let status_counts = count_statuses(entries);
    let mut lines = vec![
        "git_worktree_summary".to_string(),
        format!("repo_root: {repo_root}"),
    ];
    let workdir_text = workdir.display().to_string();
    if normalize_path_for_compare(&workdir_text) != normalize_path_for_compare(repo_root) {
        lines.push(format!("workdir: {}", workdir.display()));
    }
    if !branch.is_empty() {
        lines.push(format!("branch: {branch}"));
    }
    lines.push(format!("changed_paths: {}", entries.len()));
    if !status_counts.is_empty() {
        lines.push(format!("status_counts: {}", render_counts(&status_counts)));
    }
    if !staged_shortstat.is_empty() {
        lines.push(format!("staged_diff: {staged_shortstat}"));
    }
    if !shortstat.is_empty() {
        lines.push(format!("unstaged_diff: {shortstat}"));
    }

    if entries.is_empty() {
        lines.push("status: clean".to_string());
        return lines.join("\n");
    }

    let omitted = entries.len().saturating_sub(limit);
    lines.push(format!(
        "paths: {} shown, {} omitted",
        entries.len().min(limit),
        omitted
    ));
    lines.extend(
        entries
            .iter()
            .take(limit)
            .map(|entry| format!("{} {}", entry.status, entry.path)),
    );
    if omitted > 0 {
        lines.push("fallback_required: true".to_string());
        lines.push("fallback_reason: max_paths".to_string());
    }
    lines.join("\n")
}

fn count_statuses(entries: &[GitStatusEntry]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for entry in entries {
        *counts.entry(normalized_status(&entry.status)).or_default() += 1;
    }
    counts
}

fn normalized_status(status: &str) -> String {
    let trimmed = status.trim();
    if trimmed.is_empty() {
        status.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_path_for_compare(path: &str) -> String {
    path.replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn render_counts(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_git_status_z_handles_renames_and_copies() {
        let entries = parse_git_status_z(b" M src/lib.rs\0R  new.rs\0old.rs\0 C copy.rs\0src.rs\0");

        assert_eq!(
            entries,
            vec![
                GitStatusEntry {
                    status: " M".to_string(),
                    path: "src/lib.rs".to_string(),
                },
                GitStatusEntry {
                    status: "R ".to_string(),
                    path: "old.rs -> new.rs".to_string(),
                },
                GitStatusEntry {
                    status: " C".to_string(),
                    path: "src.rs -> copy.rs".to_string(),
                },
            ]
        );
    }

    #[test]
    fn render_git_summary_marks_omitted_paths() {
        let entries = vec![
            GitStatusEntry {
                status: " M".to_string(),
                path: "a.rs".to_string(),
            },
            GitStatusEntry {
                status: "??".to_string(),
                path: "b.rs".to_string(),
            },
        ];

        let summary = render_git_worktree_summary(
            Path::new("repo"),
            "repo",
            "main",
            "1 file changed",
            "",
            &entries,
            1,
        );

        assert!(summary.contains("changed_paths: 2"));
        assert!(summary.contains("status_counts: M=1,??=1"));
        assert!(summary.contains("paths: 1 shown, 1 omitted"));
        assert!(summary.contains("fallback_required: true"));
        assert!(summary.contains("fallback_reason: max_paths"));
    }
}
