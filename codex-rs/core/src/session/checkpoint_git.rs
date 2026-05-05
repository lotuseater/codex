use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum GitCheckpointOutcome {
    Disabled,
    NotRepository,
    NoChanges,
    Blocked(String),
    CommittedAndPushed { commit: String },
    CommittedPushFailed { commit: String, error: String },
    Failed(String),
}

impl GitCheckpointOutcome {
    pub(crate) fn summary(&self) -> String {
        match self {
            Self::Disabled => "git checkpoint sync disabled".to_string(),
            Self::NotRepository => {
                "git checkpoint sync skipped: cwd is not a git repository".to_string()
            }
            Self::NoChanges => {
                "git checkpoint sync skipped: no checkpoint-owned changes".to_string()
            }
            Self::Blocked(reason) => format!("git checkpoint sync blocked: {reason}"),
            Self::CommittedAndPushed { commit } => {
                format!("git checkpoint sync committed and pushed {commit}")
            }
            Self::CommittedPushFailed { commit, error } => {
                format!("git checkpoint sync committed {commit}, but push failed: {error}")
            }
            Self::Failed(error) => format!("git checkpoint sync failed: {error}"),
        }
    }

    pub(crate) fn should_warn(&self) -> bool {
        matches!(
            self,
            Self::Blocked(_) | Self::CommittedPushFailed { .. } | Self::Failed(_)
        )
    }
}

pub(crate) fn dirty_paths(cwd: &Path) -> Result<HashSet<String>, String> {
    if !is_git_repository(cwd) {
        return Ok(HashSet::new());
    }
    let output = git_output(cwd, &["status", "--porcelain=v1", "-z"])?;
    Ok(parse_porcelain_z_paths(output.as_bytes()))
}

pub(crate) fn worktree_key(cwd: &Path) -> Result<Option<String>, String> {
    if !is_git_repository(cwd) {
        return Ok(None);
    }
    let root = git_output(cwd, &["rev-parse", "--show-toplevel"])?;
    let root = root.trim();
    if root.is_empty() {
        return Ok(None);
    }
    Ok(Some(root.replace('\\', "/")))
}

pub(crate) fn commit_and_push_checkpoint(
    cwd: &Path,
    baseline_dirty_paths: &HashSet<String>,
    title: &str,
    body: &str,
) -> (GitCheckpointOutcome, HashSet<String>) {
    if !is_git_repository(cwd) {
        return (GitCheckpointOutcome::NotRepository, HashSet::new());
    }

    if let Some(blocker) = repository_blocker(cwd) {
        return (
            GitCheckpointOutcome::Blocked(blocker),
            baseline_dirty_paths.clone(),
        );
    }

    let current_dirty_paths = match dirty_paths(cwd) {
        Ok(paths) => paths,
        Err(error) => {
            return (
                GitCheckpointOutcome::Failed(error),
                baseline_dirty_paths.clone(),
            );
        }
    };
    let mut checkpoint_paths: Vec<String> = current_dirty_paths
        .difference(baseline_dirty_paths)
        .cloned()
        .collect();
    checkpoint_paths.sort();
    if checkpoint_paths.is_empty() {
        return (GitCheckpointOutcome::NoChanges, current_dirty_paths);
    }

    let staged_paths = match staged_paths(cwd) {
        Ok(paths) => paths,
        Err(error) => {
            return (
                GitCheckpointOutcome::Failed(error),
                baseline_dirty_paths.clone(),
            );
        }
    };
    if !staged_paths.is_empty() {
        return (
            GitCheckpointOutcome::Blocked(
                "repository has pre-staged changes; refusing checkpoint commit".to_string(),
            ),
            baseline_dirty_paths.clone(),
        );
    }

    let mut add_args = vec!["add", "--"];
    let checkpoint_path_refs: Vec<&str> = checkpoint_paths.iter().map(String::as_str).collect();
    add_args.extend(checkpoint_path_refs);
    if let Err(error) = git_output(cwd, &add_args) {
        return (
            GitCheckpointOutcome::Failed(format!("git add failed: {error}")),
            baseline_dirty_paths.clone(),
        );
    }

    if let Err(error) = git_output(cwd, &["commit", "-m", title, "-m", body]) {
        let mut reset_args = vec!["reset", "--"];
        reset_args.extend(checkpoint_paths.iter().map(String::as_str));
        let reset_error = git_output(cwd, &reset_args).err();
        let error = match reset_error {
            Some(reset_error) => {
                format!(
                    "git commit failed: {error}; additionally failed to unstage checkpoint paths: {reset_error}"
                )
            }
            None => format!("git commit failed: {error}"),
        };
        return (
            GitCheckpointOutcome::Failed(error),
            baseline_dirty_paths.clone(),
        );
    }

    let commit = match git_output(cwd, &["rev-parse", "--short=12", "HEAD"]) {
        Ok(commit) => commit.trim().to_string(),
        Err(error) => {
            return (
                GitCheckpointOutcome::Failed(format!("git rev-parse after commit failed: {error}")),
                baseline_dirty_paths.clone(),
            );
        }
    };
    let post_commit_dirty_paths = dirty_paths(cwd).unwrap_or_default();

    if let Err(error) = git_output(cwd, &["push"]) {
        return (
            GitCheckpointOutcome::CommittedPushFailed { commit, error },
            post_commit_dirty_paths,
        );
    }

    (
        GitCheckpointOutcome::CommittedAndPushed { commit },
        post_commit_dirty_paths,
    )
}

fn is_git_repository(cwd: &Path) -> bool {
    git_output(cwd, &["rev-parse", "--is-inside-work-tree"])
        .map(|output| output.trim() == "true")
        .unwrap_or(false)
}

fn repository_blocker(cwd: &Path) -> Option<String> {
    let git_dir = git_output(cwd, &["rev-parse", "--git-dir"]).ok()?;
    let git_dir = resolve_git_dir(cwd, git_dir.trim());
    for marker in ["MERGE_HEAD", "CHERRY_PICK_HEAD", "REVERT_HEAD"] {
        if git_dir.join(marker).exists() {
            return Some(format!("{marker} exists"));
        }
    }
    for marker in ["rebase-merge", "rebase-apply"] {
        if git_dir.join(marker).exists() {
            return Some(format!("{marker} exists"));
        }
    }
    if git_output(
        cwd,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .is_err()
    {
        return Some("current branch has no upstream".to_string());
    }
    None
}

fn resolve_git_dir(cwd: &Path, git_dir: &str) -> PathBuf {
    let path = PathBuf::from(git_dir);
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

fn git_output(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|err| format!("failed to launch git: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(if detail.is_empty() {
            format!("git {:?} exited with {}", args, output.status)
        } else {
            detail
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn staged_paths(cwd: &Path) -> Result<HashSet<String>, String> {
    let output = git_output(cwd, &["diff", "--cached", "--name-only", "-z"])?;
    Ok(parse_z_paths(output.as_bytes()))
}

fn parse_z_paths(output: &[u8]) -> HashSet<String> {
    output
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| String::from_utf8_lossy(entry).replace('\\', "/"))
        .collect()
}

fn parse_porcelain_z_paths(output: &[u8]) -> HashSet<String> {
    let mut paths = HashSet::new();
    let entries: Vec<&[u8]> = output.split(|byte| *byte == 0).collect();
    let mut index = 0;
    while index < entries.len() {
        let entry = entries[index];
        index += 1;
        if entry.is_empty() {
            continue;
        }
        let status = if entry.len() > 3 && entry[2] == b' ' {
            &entry[..2]
        } else {
            b""
        };
        let path_bytes = if entry.len() > 3 && entry[2] == b' ' {
            &entry[3..]
        } else {
            entry
        };
        let path = String::from_utf8_lossy(path_bytes).replace('\\', "/");
        if !path.is_empty() {
            paths.insert(path);
        }
        if status.contains(&b'R') || status.contains(&b'C') {
            index += 1;
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parse_porcelain_z_paths_handles_tracked_and_untracked_paths() {
        let paths = parse_porcelain_z_paths(b" M src/lib.rs\0?? new file.txt\0");

        assert!(paths.contains("src/lib.rs"));
        assert!(paths.contains("new file.txt"));
    }

    #[test]
    fn parse_porcelain_z_paths_handles_rename_records() {
        let paths = parse_porcelain_z_paths(b"R  new-name.txt\0old-name.txt\0 M src/lib.rs\0");

        assert!(paths.contains("new-name.txt"));
        assert!(paths.contains("src/lib.rs"));
        assert!(!paths.contains("old-name.txt"));
    }

    #[test]
    fn parse_z_paths_handles_staged_paths() {
        let paths = parse_z_paths(b"src/lib.rs\0new file.txt\0");

        assert!(paths.contains("src/lib.rs"));
        assert!(paths.contains("new file.txt"));
    }

    #[test]
    fn checkpoint_outcome_warns_only_for_actionable_failures() {
        assert!(!GitCheckpointOutcome::Disabled.should_warn());
        assert!(!GitCheckpointOutcome::NoChanges.should_warn());
        assert!(GitCheckpointOutcome::Blocked("merge in progress".to_string()).should_warn());
        assert!(
            GitCheckpointOutcome::CommittedPushFailed {
                commit: "abc123".to_string(),
                error: "no upstream".to_string(),
            }
            .should_warn()
        );
    }

    #[test]
    fn checkpoint_commit_refuses_pre_staged_changes() {
        let remote = TempDir::new().expect("create remote tempdir");
        run_git(remote.path(), &["init", "--bare"]);

        let repo = TempDir::new().expect("create repo tempdir");
        run_git(repo.path(), &["init"]);
        run_git(repo.path(), &["config", "user.email", "codex@example.com"]);
        run_git(repo.path(), &["config", "user.name", "Codex Test"]);
        fs::write(repo.path().join("README.md"), "initial\n").expect("write initial file");
        run_git(repo.path(), &["add", "README.md"]);
        run_git(repo.path(), &["commit", "-m", "initial"]);
        let remote_path = remote.path().to_string_lossy().into_owned();
        run_git(repo.path(), &["remote", "add", "origin", &remote_path]);
        run_git(repo.path(), &["push", "-u", "origin", "HEAD"]);

        let baseline = dirty_paths(repo.path()).expect("read clean baseline");
        let head_before = git_output(repo.path(), &["rev-parse", "HEAD"]).expect("read head");

        fs::write(repo.path().join("staged.txt"), "staged\n").expect("write staged file");
        run_git(repo.path(), &["add", "staged.txt"]);
        fs::write(repo.path().join("checkpoint.txt"), "checkpoint\n")
            .expect("write checkpoint file");

        let (outcome, _) = commit_and_push_checkpoint(repo.path(), &baseline, "checkpoint", "body");

        assert_eq!(
            outcome,
            GitCheckpointOutcome::Blocked(
                "repository has pre-staged changes; refusing checkpoint commit".to_string(),
            )
        );
        assert_eq!(
            git_output(repo.path(), &["rev-parse", "HEAD"]).expect("read head after block"),
            head_before
        );
        assert!(
            staged_paths(repo.path())
                .expect("read staged paths")
                .contains("staged.txt")
        );
    }

    #[test]
    fn worktree_key_uses_repository_root_for_subdirectories() {
        let repo = TempDir::new().expect("create repo tempdir");
        run_git(repo.path(), &["init"]);
        let nested = repo.path().join("nested");
        fs::create_dir(&nested).expect("create nested dir");

        assert_eq!(
            worktree_key(&nested).expect("resolve nested worktree key"),
            worktree_key(repo.path()).expect("resolve root worktree key"),
        );
    }

    fn run_git(cwd: &Path, args: &[&str]) {
        git_output(cwd, args).unwrap_or_else(|error| panic!("git {args:?} failed: {error}"));
    }
}
