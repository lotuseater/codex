mod format;
mod git;
mod index;
mod rank;
mod shadow;
mod types;

pub use types::Anchor;
pub use types::ChangedAreas;
pub use types::ChangedPath;
pub use types::FileRecord;
pub use types::RepoContextScoutConfig;
pub use types::RepoContextScoutMode;
pub use types::RepoIndex;
pub use types::Result;
pub use types::ScoutBundle;
pub use types::ScoutCandidate;
pub use types::ScoutCommandMode;
pub use types::ScoutError;
pub use types::ScoutRequest;
pub use types::ScoutStatus;
pub use types::ScoutTrigger;
pub use types::ShadowRecord;
pub use types::SupportRoute;

use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use sha1::Digest;
use sha1::Sha1;

use crate::format::format_packet;
use crate::git::current_git_head;
use crate::git::git_root;
use crate::git::read_changed_areas;
use crate::index::build_index;
use crate::index::load_index;
use crate::index::save_index;
use crate::index::with_changed_overlay;
use crate::rank::rank_files;
use crate::shadow::append_shadow_record;
use crate::types::IndexState;

pub fn run_scout(request: ScoutRequest<'_>) -> Result<ScoutBundle> {
    let project_root =
        git_root(request.project_root).unwrap_or_else(|_| request.project_root.to_path_buf());
    let cache = CachePaths::new(request.codex_home, project_root.as_path())?;
    let mut loaded_index = load_index(cache.index.as_path())?;
    let had_index = loaded_index.is_some();
    if matches!(request.mode, ScoutCommandMode::Refresh) || loaded_index.is_none() {
        let index = build_index(project_root.as_path(), &request.config)?;
        save_index(cache.index.as_path(), &index)?;
        loaded_index = Some(index);
    }

    let Some(index) = loaded_index else {
        return Err(ScoutError::InvalidProjectRoot(project_root));
    };

    let changed = read_changed_areas(project_root.as_path());
    let current_head = current_git_head(project_root.as_path());
    let index_state = if !had_index {
        IndexState::Cold
    } else if index.git_head != current_head {
        IndexState::Stale
    } else {
        IndexState::Warm
    };
    let overlay_index = with_changed_overlay(index, &changed, &request.config)?;
    let candidates = if matches!(request.mode, ScoutCommandMode::Status) {
        Vec::new()
    } else {
        rank_files(&overlay_index, &changed, request.prompt, &request.config)
    };
    let support_routes = rank::support_routes_for_prompt(request.prompt, &candidates);
    let packet = format_packet(
        &overlay_index,
        &changed,
        &candidates,
        &support_routes,
        index_state,
        request.prompt,
        &request.config,
    );
    let status = ScoutStatus {
        repo_key: cache.repo_key.clone(),
        cache_dir: cache.dir,
        index_path: cache.index,
        generated_at_unix: overlay_index.generated_at_unix,
        index_state,
        indexed_files: overlay_index.files.len(),
        git_head: overlay_index.git_head.clone(),
        changed_paths: changed.paths.clone(),
    };

    Ok(ScoutBundle {
        repo_key: cache.repo_key,
        project_root,
        status,
        candidates,
        support_routes,
        packet_tokens: approx_tokens(&packet),
        packet,
    })
}

pub fn run_shadow(request: ScoutRequest<'_>) -> Result<()> {
    let prompt_hash = short_hash(request.prompt);
    let started_at_unix = unix_now();
    let result = run_scout(request);
    let project_root =
        git_root(request.project_root).unwrap_or_else(|_| request.project_root.to_path_buf());
    let cache = CachePaths::new(request.codex_home, project_root.as_path())?;
    let record = match result {
        Ok(bundle) => ShadowRecord {
            timestamp_unix: started_at_unix,
            trigger: request.trigger,
            prompt_hash,
            repo_key: bundle.repo_key,
            index_state: bundle.status.index_state,
            selected_paths: bundle
                .candidates
                .iter()
                .map(|candidate| candidate.path.clone())
                .collect(),
            support_routes: bundle.support_routes,
            packet_tokens: bundle.packet_tokens,
            changed_path_count: bundle.status.changed_paths.len(),
            fallback_reason: None,
            verdict: "recorded".to_string(),
        },
        Err(err) => ShadowRecord {
            timestamp_unix: started_at_unix,
            trigger: request.trigger,
            prompt_hash,
            repo_key: cache.repo_key.clone(),
            index_state: IndexState::Cold,
            selected_paths: Vec::new(),
            support_routes: Vec::new(),
            packet_tokens: 0,
            changed_path_count: 0,
            fallback_reason: Some(err.to_string()),
            verdict: "fallback".to_string(),
        },
    };
    append_shadow_record(cache.shadow.as_path(), &record)
}

pub fn approx_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

pub(crate) fn short_hash(value: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(value.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    digest.chars().take(12).collect()
}

pub(crate) fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[derive(Debug)]
struct CachePaths {
    repo_key: String,
    dir: PathBuf,
    index: PathBuf,
    shadow: PathBuf,
}

impl CachePaths {
    fn new(codex_home: &Path, project_root: &Path) -> Result<Self> {
        let root = project_root
            .canonicalize()
            .unwrap_or_else(|_| project_root.to_path_buf());
        let leaf = root
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("repo");
        let repo_key = format!("{leaf}-{}", short_hash(&slash_path(root.as_path())));
        let dir = codex_home.join("context-scout").join(&repo_key);
        Ok(Self {
            index: dir.join("index.json"),
            shadow: dir.join("shadow.jsonl"),
            repo_key,
            dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use super::*;

    fn git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .status()
            .expect("git command should start");
        assert!(status.success(), "git {args:?} failed");
    }

    fn init_repo() -> TempDir {
        let temp = TempDir::new().expect("tempdir");
        git(temp.path(), &["init"]);
        git(temp.path(), &["config", "user.email", "test@example.com"]);
        git(temp.path(), &["config", "user.name", "Test"]);
        fs::create_dir_all(temp.path().join(".github/workflows")).expect("mkdir");
        fs::create_dir_all(temp.path().join("target")).expect("mkdir");
        fs::write(temp.path().join("src.rs"), "pub fn alpha() {}\n").expect("write src");
        fs::write(temp.path().join(".github/workflows/ci.yml"), "name: ci\n").expect("write ci");
        fs::write(
            temp.path().join("target/generated.rs"),
            "pub fn generated() {}\n",
        )
        .expect("write target");
        git(temp.path(), &["add", "."]);
        git(temp.path(), &["commit", "-m", "init"]);
        temp
    }

    #[test]
    fn index_includes_hidden_config_and_skips_generated_dirs() {
        let repo = init_repo();
        let cache = TempDir::new().expect("cache");
        let bundle = run_scout(ScoutRequest {
            project_root: repo.path(),
            codex_home: cache.path(),
            prompt: "review ci workflow",
            config: RepoContextScoutConfig::default(),
            mode: ScoutCommandMode::Refresh,
            trigger: ScoutTrigger::Manual,
        })
        .expect("scout should run");

        let indexed = bundle.status.index_path.as_path().to_path_buf();
        let saved = load_index(indexed.as_path())
            .expect("index load should work")
            .expect("index should exist");
        let paths = saved
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>();
        assert!(paths.contains(&".github/workflows/ci.yml"));
        assert!(!paths.contains(&"target/generated.rs"));
    }

    #[test]
    fn changed_overlay_expands_untracked_directories() {
        let repo = init_repo();
        fs::create_dir_all(repo.path().join("newdir")).expect("mkdir");
        fs::write(
            repo.path().join("newdir/new_file.rs"),
            "pub fn changed() {}\n",
        )
        .expect("write new file");
        let cache = TempDir::new().expect("cache");
        let bundle = run_scout(ScoutRequest {
            project_root: repo.path(),
            codex_home: cache.path(),
            prompt: "fix changed code",
            config: RepoContextScoutConfig::default(),
            mode: ScoutCommandMode::Scout,
            trigger: ScoutTrigger::Manual,
        })
        .expect("scout should run");

        assert!(
            bundle
                .status
                .changed_paths
                .iter()
                .any(|path| path.path == "newdir/new_file.rs")
        );
        assert_eq!(bundle.candidates[0].path, "newdir/new_file.rs");
    }

    #[test]
    fn packet_respects_token_budget() {
        let repo = init_repo();
        fs::write(
            repo.path().join("src.rs"),
            "pub fn alpha() {}\npub fn beta() {}\n",
        )
        .expect("write src");
        let cache = TempDir::new().expect("cache");
        let config = RepoContextScoutConfig {
            max_output_tokens: 60,
            ..RepoContextScoutConfig::default()
        };
        let bundle = run_scout(ScoutRequest {
            project_root: repo.path(),
            codex_home: cache.path(),
            prompt: "alpha beta review",
            config,
            mode: ScoutCommandMode::Scout,
            trigger: ScoutTrigger::Manual,
        })
        .expect("scout should run");

        assert!(bundle.packet_tokens <= 80);
    }
}
