use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

const MAX_BASELINE_FILES: usize = 32;
const MAX_BASELINE_FILE_BYTES: u64 = 512 * 1024;
const MAX_PROMPT_LIST_ITEMS: usize = 40;
const MAX_GIT_OUTPUT_CHARS: usize = 12_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineSnapshot {
    pub repo_path: String,
    pub baseline_path: PathBuf,
}

#[derive(Debug)]
pub struct ReviewAnchor {
    anchor: GitReviewAnchor,
}

impl ReviewAnchor {
    pub fn capture(cwd: impl Into<PathBuf>) -> Self {
        Self {
            anchor: GitReviewAnchor::capture(cwd),
        }
    }

    pub fn work_slice(&self) -> ReviewWorkSlice {
        ReviewWorkSlice {
            anchor: self.anchor.clone(),
        }
    }
}

impl Drop for ReviewAnchor {
    fn drop(&mut self) {
        self.anchor.cleanup();
    }
}

#[derive(Debug, Clone)]
pub struct ReviewWorkSlice {
    anchor: GitReviewAnchor,
}

impl ReviewWorkSlice {
    pub fn review_prompt(&self, work_notes: &str) -> String {
        self.anchor.prompt(work_notes)
    }
}

#[derive(Debug, Clone)]
pub struct GitReviewAnchor {
    cwd: PathBuf,
    head: Option<String>,
    dirty_tracked_files: Vec<String>,
    staged_files: Vec<String>,
    untracked_files: Vec<String>,
    baseline_dir: Option<PathBuf>,
    baseline_snapshots: Vec<BaselineSnapshot>,
    capture_error: Option<String>,
}

impl GitReviewAnchor {
    pub fn capture(cwd: impl Into<PathBuf>) -> Self {
        let cwd = cwd.into();
        let git_root = match git_output(&cwd, ["rev-parse", "--show-toplevel"]) {
            Ok(root) => PathBuf::from(root.trim()),
            Err(err) => {
                return Self {
                    cwd,
                    head: None,
                    dirty_tracked_files: Vec::new(),
                    staged_files: Vec::new(),
                    untracked_files: Vec::new(),
                    baseline_dir: None,
                    baseline_snapshots: Vec::new(),
                    capture_error: Some(err),
                };
            }
        };

        let head = git_output(&git_root, ["rev-parse", "HEAD"])
            .ok()
            .map(|head| head.trim().to_string())
            .filter(|head| !head.is_empty());
        let unstaged_files = git_lines(
            &git_root,
            ["diff", "--name-only", "--diff-filter=ACMRTUXB", "--"],
        );
        let staged_files = git_lines(
            &git_root,
            [
                "diff",
                "--cached",
                "--name-only",
                "--diff-filter=ACMRTUXB",
                "--",
            ],
        );
        let untracked_files = git_lines(&git_root, ["ls-files", "--others", "--exclude-standard"]);
        let dirty_tracked_files = unique_paths(
            unstaged_files
                .iter()
                .chain(staged_files.iter())
                .cloned()
                .collect(),
        );
        let (baseline_dir, baseline_snapshots) =
            capture_baselines(&git_root, dirty_tracked_files.iter());

        Self {
            cwd: git_root,
            head,
            dirty_tracked_files,
            staged_files,
            untracked_files,
            baseline_dir,
            baseline_snapshots,
            capture_error: None,
        }
    }

    pub fn cleanup(&self) {
        if let Some(dir) = &self.baseline_dir
            && fs::remove_dir_all(dir).is_err()
        {
            // Best effort: stale temp baselines are bounded and replaced on the next anchor.
        }
    }

    pub fn prompt(&self, work_notes: &str) -> String {
        if let Some(error) = &self.capture_error {
            return limited_git_prompt(&self.cwd, error, work_notes);
        }

        let current_head = git_output(&self.cwd, ["rev-parse", "HEAD"])
            .ok()
            .map(|head| head.trim().to_string())
            .filter(|head| !head.is_empty());
        let commits_since_anchor = self
            .head
            .as_deref()
            .and_then(|head| {
                git_output(&self.cwd, ["log", "--oneline", &format!("{head}..HEAD")]).ok()
            })
            .map(|output| truncate_output(output.trim()))
            .filter(|output| !output.is_empty())
            .unwrap_or_else(|| {
                "(no commits since anchor or anchor commit unavailable)".to_string()
            });

        let committed_files = self
            .head
            .as_deref()
            .map(|head| {
                git_lines(
                    &self.cwd,
                    ["diff", "--name-only", &format!("{head}..HEAD"), "--"],
                )
            })
            .unwrap_or_default();
        let unstaged_files = git_lines(&self.cwd, ["diff", "--name-only", "--"]);
        let staged_files = git_lines(&self.cwd, ["diff", "--cached", "--name-only", "--"]);
        let untracked_files = git_lines(&self.cwd, ["ls-files", "--others", "--exclude-standard"]);
        let changed_files = unique_paths(
            committed_files
                .iter()
                .chain(staged_files.iter())
                .chain(unstaged_files.iter())
                .chain(untracked_files.iter())
                .cloned()
                .collect(),
        );

        let anchor_head = self
            .head
            .as_deref()
            .map(str::to_string)
            .unwrap_or_else(|| "(anchor commit unavailable)".to_string());
        let current_head =
            current_head.unwrap_or_else(|| "(current commit unavailable)".to_string());
        let files_arg = shell_join(changed_files.iter().take(MAX_PROMPT_LIST_ITEMS));
        let files_arg = if files_arg.is_empty() {
            "<files>".to_string()
        } else {
            files_arg
        };
        let committed_diff_command = self.head.as_deref().map_or_else(
            || "git diff <anchor>..HEAD -- <files>".to_string(),
            |head| format!("git diff {head}..HEAD -- {files_arg}"),
        );
        let baseline_commands = self
            .baseline_snapshots
            .iter()
            .filter(|snapshot| changed_files.iter().any(|path| path == &snapshot.repo_path))
            .take(MAX_PROMPT_LIST_ITEMS)
            .map(|snapshot| {
                format!(
                    "git diff --no-index -- {} {}",
                    quote_path(&snapshot.baseline_path),
                    quote_path(self.cwd.join(&snapshot.repo_path))
                )
            })
            .collect::<Vec<_>>();
        let baseline_command_section = if baseline_commands.is_empty() {
            "- (no dirty-at-anchor baseline snapshots apply to the current changed files)"
                .to_string()
        } else {
            baseline_commands
                .iter()
                .map(|command| format!("- `{command}`"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "\
Automatic self-review of the just-completed work slice.

Please review the changes done since the last self-review checkpoint. Use the git evidence below before relying on conversation memory. Fix actionable findings, then let Codex resume queued work.

Review anchor:
- repository: {}
- anchor commit: {anchor_head}
- current commit: {current_head}
- dirty tracked files at anchor: {}
- staged files at anchor: {}
- untracked files at anchor: {}
- dirty-at-anchor baseline snapshots: {}

Work since anchor:
- commits since anchor:
{commits_since_anchor}
- changed files since anchor: {}
- currently staged files: {}
- currently unstaged tracked files: {}
- currently untracked files: {}

Exact diff commands to inspect:
- `git status --short`
- `git log --oneline {anchor_head}..HEAD`
- `{committed_diff_command}`
- `git diff --cached -- {files_arg}`
- `git diff -- {files_arg}`
{baseline_command_section}

Review instructions:
- Check for bugs, regressions, maintainability issues, missed tests, and long-term design problems.
- Prefer targeted file reads and the diff commands above; do not broaden into unrelated repository areas.
- If you find a concrete repo-controlled issue, apply one coherent repair pass and rerun the most relevant targeted verification before finalizing.
- If there are no findings, say that explicitly and continue with queued work.

Compact work notes:
{work_notes}
",
            self.cwd.display(),
            format_list(&self.dirty_tracked_files),
            format_list(&self.staged_files),
            format_list(&self.untracked_files),
            format_baselines(&self.baseline_snapshots),
            format_list(&changed_files),
            format_list(&staged_files),
            format_list(&unstaged_files),
            format_list(&untracked_files),
        )
    }
}

fn limited_git_prompt(cwd: &Path, error: &str, work_notes: &str) -> String {
    format!(
        "\
Automatic self-review of the just-completed work slice.

Git evidence is limited: Codex could not capture a git review anchor for `{}` ({error}).

Review instructions:
- First check whether this is expected for the current working directory.
- If a git repository is available, start with `git status --short`, then inspect targeted diffs for the changed files.
- Otherwise, use the compact work notes below and the most relevant targeted file reads to review for bugs, regressions, maintainability issues, missed tests, and long-term design problems.
- If you find a concrete repo-controlled issue, apply one coherent repair pass and rerun the most relevant targeted verification before finalizing.
- If there are no findings, say that explicitly and continue with queued work.

Compact work notes:
{work_notes}
",
        cwd.display()
    )
}

fn capture_baselines<'a>(
    cwd: &Path,
    paths: impl Iterator<Item = &'a String>,
) -> (Option<PathBuf>, Vec<BaselineSnapshot>) {
    let dir = std::env::temp_dir().join(format!(
        "codex-self-review-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));
    let mut snapshots = Vec::new();

    for repo_path in paths.take(MAX_BASELINE_FILES) {
        let source = cwd.join(repo_path);
        let Ok(metadata) = fs::metadata(&source) else {
            continue;
        };
        if !metadata.is_file() || metadata.len() > MAX_BASELINE_FILE_BYTES {
            continue;
        }
        if snapshots.is_empty() && fs::create_dir_all(&dir).is_err() {
            return (None, Vec::new());
        }
        let baseline_path = dir.join(safe_baseline_name(repo_path));
        if fs::copy(&source, &baseline_path).is_ok() {
            snapshots.push(BaselineSnapshot {
                repo_path: repo_path.clone(),
                baseline_path,
            });
        }
    }

    let baseline_dir = (!snapshots.is_empty()).then_some(dir);
    (baseline_dir, snapshots)
}

fn git_lines<const N: usize>(cwd: &Path, args: [&str; N]) -> Vec<String> {
    git_output(cwd, args)
        .map(|output| {
            output
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn git_output<const N: usize>(cwd: &Path, args: [&str; N]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.trim().to_string())
    }
}

fn unique_paths(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn truncate_output(output: &str) -> String {
    let mut chars = output.chars();
    let truncated = chars
        .by_ref()
        .take(MAX_GIT_OUTPUT_CHARS)
        .collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}\n... truncated ...")
    } else {
        truncated
    }
}

fn format_list(items: &[String]) -> String {
    if items.is_empty() {
        return "(none)".to_string();
    }
    let mut formatted = items
        .iter()
        .take(MAX_PROMPT_LIST_ITEMS)
        .cloned()
        .collect::<Vec<_>>();
    if items.len() > MAX_PROMPT_LIST_ITEMS {
        formatted.push(format!(
            "... {} more",
            items.len().saturating_sub(MAX_PROMPT_LIST_ITEMS)
        ));
    }
    formatted.join(", ")
}

fn format_baselines(snapshots: &[BaselineSnapshot]) -> String {
    if snapshots.is_empty() {
        return "(none)".to_string();
    }
    snapshots
        .iter()
        .take(MAX_PROMPT_LIST_ITEMS)
        .map(|snapshot| {
            format!(
                "{} -> {}",
                snapshot.repo_path,
                snapshot.baseline_path.display()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn shell_join<'a>(paths: impl Iterator<Item = &'a String>) -> String {
    paths
        .map(|path| quote_str(path))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_path(path: impl AsRef<Path>) -> String {
    quote_str(&path.as_ref().display().to_string())
}

fn quote_str(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }
    if arg
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '\\' | '.' | '_' | '-' | ':'))
    {
        return arg.to_string();
    }
    format!("'{}'", arg.replace('\'', "'\\''"))
}

fn safe_baseline_name(path: &str) -> String {
    path.chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn prompt_includes_commits_files_and_diff_commands() {
        let repo = test_repo();
        write_file(&repo, "src/lib.rs", "before\n");
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "initial"]);

        let anchor = ReviewAnchor::capture(&repo);
        write_file(&repo, "src/lib.rs", "after\n");
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "change lib"]);
        write_file(&repo, "src/main.rs", "dirty\n");

        let prompt = anchor.work_slice().review_prompt("- commands completed: 1");

        assert!(prompt.contains("change lib"));
        assert!(prompt.contains("src/lib.rs"));
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("git log --oneline"));
        assert!(prompt.contains("git diff --"));
        assert!(prompt.contains("commands completed: 1"));
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn dirty_anchor_uses_no_index_baseline_command() {
        let repo = test_repo();
        write_file(&repo, "src/lib.rs", "clean\n");
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "initial"]);
        write_file(&repo, "src/lib.rs", "dirty at anchor\n");

        let anchor = ReviewAnchor::capture(&repo);
        write_file(&repo, "src/lib.rs", "changed after anchor\n");

        let prompt = anchor.work_slice().review_prompt("- file-change steps: 1");

        assert!(prompt.contains("git diff --no-index --"));
        assert!(prompt.contains("src/lib.rs"));
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn non_git_prompt_has_fallback_commands() {
        let dir = unique_temp_dir("non-git");
        fs::create_dir_all(&dir).unwrap();

        let anchor = ReviewAnchor::capture(&dir);
        let prompt = anchor.work_slice().review_prompt("- commands completed: 0");

        assert!(prompt.contains("Git evidence is limited"));
        assert!(prompt.contains("git status --short"));
        let _ = fs::remove_dir_all(dir);
    }

    fn test_repo() -> PathBuf {
        let repo = unique_temp_dir("repo");
        fs::create_dir_all(repo.join("src")).unwrap();
        git(&repo, &["init"]);
        git(&repo, &["config", "user.email", "codex@example.test"]);
        git(&repo, &["config", "user.name", "Codex Test"]);
        repo
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!(
            "codex-self-review-{label}-{}-{stamp}",
            std::process::id()
        ))
    }

    fn write_file(repo: &Path, path: &str, contents: &str) {
        let path = repo.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
    }

    fn git(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
