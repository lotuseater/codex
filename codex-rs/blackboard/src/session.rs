use crate::BlackboardError;
use crate::BlackboardEvent;
use crate::BlackboardEventKind;
use crate::GlobalIndexRecord;
use crate::active_external_update;
use crate::append_event;
use crate::append_global_index_record;
use crate::clear_if_no_active_external;
use crate::has_active_external_session;
use crate::now_unix_seconds;
use crate::read_events;
use crate::repo_blackboard_path;
use crate::repo_id_for_root;
use crate::truncate_text;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::warn;

#[derive(Debug, Clone)]
struct JoinedRepo {
    repo_id: String,
    repo_root: PathBuf,
    blackboard_path: PathBuf,
    branch: Option<String>,
    intent_recorded: bool,
    last_heartbeat_at: i64,
    last_seen_update_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BlackboardSessionConfig {
    pub path: String,
    pub global_index_path: String,
    pub poll_interval_ms: u64,
    pub heartbeat_interval_seconds: u64,
    pub stale_after_seconds: u64,
    pub recent_window_seconds: u64,
    pub max_injected_bytes: usize,
    pub max_entry_chars: usize,
    pub max_file_bytes: u64,
    pub max_joined_repos: usize,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlackboardSessionMode {
    Root,
    NonRootAgent,
}

impl BlackboardSessionMode {
    fn allows_repo_writes(self) -> bool {
        matches!(self, Self::Root)
    }
}

#[derive(Debug, Clone)]
pub struct BlackboardSessionOptions {
    pub config: BlackboardSessionConfig,
    pub codex_home: PathBuf,
    pub session_id: String,
    pub thread_id: String,
    pub mode: BlackboardSessionMode,
}

#[derive(Debug)]
pub struct BlackboardSession {
    config: BlackboardSessionConfig,
    enabled: bool,
    identity: BlackboardIdentity,
    repo_path: PathBuf,
    global_index_path: PathBuf,
    joined: Mutex<HashMap<String, JoinedRepo>>,
    pending_updates: Mutex<HashMap<String, String>>,
    proposal_ready_repos: Mutex<HashSet<String>>,
    next_sequence: AtomicU64,
    shutdown: CancellationToken,
}

#[derive(Debug, Clone)]
struct BlackboardIdentity {
    session_id: String,
    thread_id: String,
    pid: u32,
    host: String,
}

impl BlackboardSession {
    pub fn new(options: BlackboardSessionOptions) -> Arc<Self> {
        let config = options.config;
        let repo_path = PathBuf::from(&config.path);
        let configured_global_index_path = PathBuf::from(&config.global_index_path);
        let global_index_path = if configured_global_index_path.is_absolute() {
            configured_global_index_path
        } else {
            options.codex_home.join(configured_global_index_path)
        };
        Arc::new(Self {
            enabled: config.enabled && options.mode.allows_repo_writes(),
            config,
            identity: BlackboardIdentity {
                session_id: options.session_id,
                thread_id: options.thread_id,
                pid: std::process::id(),
                host: host_name(),
            },
            repo_path,
            global_index_path,
            joined: Mutex::new(HashMap::new()),
            pending_updates: Mutex::new(HashMap::new()),
            proposal_ready_repos: Mutex::new(HashSet::new()),
            next_sequence: AtomicU64::new(1),
            shutdown: CancellationToken::new(),
        })
    }

    pub fn start(self: &Arc<Self>) {
        if !self.enabled {
            return;
        }
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let poll_interval = Duration::from_millis(this.config.poll_interval_ms);
            loop {
                tokio::select! {
                    _ = this.shutdown.cancelled() => break,
                    _ = tokio::time::sleep(poll_interval) => {
                        this.poll_once().await;
                    }
                }
            }
        });
    }

    pub async fn context_for_turn(&self, cwd: &Path, prompt: &str) -> Option<String> {
        let repo_id = self.join_repo_for_path(cwd).await?;
        self.record_intent_if_needed(&repo_id, cwd, prompt).await;
        self.poll_once().await;
        let update = self.pending_updates.lock().await.remove(&repo_id)?;
        self.proposal_ready_repos
            .lock()
            .await
            .insert(repo_id.clone());
        self.append_kind(
            &repo_id,
            BlackboardEventKind::ExternalUpdateSeen,
            Some("surfaced external blackboard update to model".to_string()),
            Some(cwd),
        )
        .await;
        Some(update)
    }

    pub async fn observe_path(&self, path: &Path) {
        let _ = self.join_repo_for_path(path).await;
    }

    pub async fn record_assistant_proposal(&self, cwd: &Path, message: &str) {
        if !self.enabled || message.trim().is_empty() {
            return;
        }
        let Some(repo_id) = self.join_repo_for_path(cwd).await else {
            return;
        };
        if !self.proposal_ready_repos.lock().await.remove(&repo_id) {
            return;
        }
        self.append_kind(
            &repo_id,
            BlackboardEventKind::Proposal,
            Some(truncate_text(message.trim(), self.config.max_entry_chars)),
            Some(cwd),
        )
        .await;
    }

    pub async fn shutdown(&self) {
        if !self.enabled {
            return;
        }
        self.shutdown.cancel();
        let repo_ids = self.joined.lock().await.keys().cloned().collect::<Vec<_>>();
        for repo_id in repo_ids {
            self.append_kind(&repo_id, BlackboardEventKind::Leave, None, None)
                .await;
            self.delete_repo_file_if_alone(&repo_id).await;
        }
    }

    async fn join_repo_for_path(&self, path: &Path) -> Option<String> {
        if !self.enabled {
            return None;
        }
        let repo_root = find_git_repo_root(path)?;
        let repo_id = repo_id_for_root(&repo_root);
        if self.joined.lock().await.contains_key(&repo_id) {
            return Some(repo_id);
        }
        let joined_repo_count = self.joined.lock().await.len();
        if joined_repo_count >= self.config.max_joined_repos {
            debug!(
                "blackboard max joined repos reached; skipping {}",
                repo_root.display()
            );
            return None;
        }
        let blackboard_path = repo_blackboard_path(&repo_root, &self.repo_path);
        let joined = JoinedRepo {
            repo_id: repo_id.clone(),
            repo_root,
            blackboard_path,
            branch: current_branch(path),
            intent_recorded: false,
            last_heartbeat_at: 0,
            last_seen_update_key: None,
        };
        self.joined
            .lock()
            .await
            .insert(repo_id.clone(), joined.clone());
        self.append_to_repo(&joined, BlackboardEventKind::Join, None, Some(path))
            .await;
        Some(repo_id)
    }

    async fn record_intent_if_needed(&self, repo_id: &str, cwd: &Path, prompt: &str) {
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return;
        }
        let joined = {
            let mut joined = self.joined.lock().await;
            let Some(repo) = joined.get_mut(repo_id) else {
                return;
            };
            if repo.intent_recorded {
                return;
            }
            repo.intent_recorded = true;
            repo.clone()
        };
        self.append_to_repo(
            &joined,
            BlackboardEventKind::Intent,
            Some(truncate_text(prompt, self.config.max_entry_chars)),
            Some(cwd),
        )
        .await;
    }

    async fn poll_once(&self) {
        if !self.enabled {
            return;
        }
        let repo_ids = self.joined.lock().await.keys().cloned().collect::<Vec<_>>();
        for repo_id in repo_ids {
            self.heartbeat_if_due(&repo_id).await;
            self.poll_repo(&repo_id).await;
        }
    }

    async fn heartbeat_if_due(&self, repo_id: &str) {
        let now = now_unix_seconds();
        let joined = {
            let mut joined = self.joined.lock().await;
            let Some(repo) = joined.get_mut(repo_id) else {
                return;
            };
            let heartbeat_due = now.saturating_sub(repo.last_heartbeat_at)
                >= i64::try_from(self.config.heartbeat_interval_seconds).unwrap_or(i64::MAX);
            if !heartbeat_due {
                return;
            }
            repo.last_heartbeat_at = now;
            repo.clone()
        };
        self.append_to_repo(&joined, BlackboardEventKind::Heartbeat, None, None)
            .await;
    }

    async fn poll_repo(&self, repo_id: &str) {
        let joined = {
            let joined = self.joined.lock().await;
            let Some(repo) = joined.get(repo_id) else {
                return;
            };
            repo.clone()
        };
        let events = match read_events(&joined.blackboard_path) {
            Ok(events) => events,
            Err(err) => {
                warn!(
                    "failed to read blackboard {}: {err}",
                    joined.blackboard_path.display()
                );
                return;
            }
        };
        let now = now_unix_seconds();
        let Some(update) = active_external_update(
            &events,
            &self.identity.session_id,
            now,
            i64::try_from(self.config.recent_window_seconds).unwrap_or(i64::MAX),
        ) else {
            return;
        };
        let Some(key) = update_key(&update.events) else {
            return;
        };
        let already_seen = {
            let mut joined = self.joined.lock().await;
            let Some(repo) = joined.get_mut(repo_id) else {
                return;
            };
            if repo.last_seen_update_key.as_deref() == Some(key.as_str()) {
                true
            } else {
                repo.last_seen_update_key = Some(key);
                false
            }
        };
        if already_seen {
            return;
        }
        let rendered = render_update(&update, self.config.max_injected_bytes);
        self.pending_updates
            .lock()
            .await
            .insert(repo_id.to_string(), rendered);
    }

    async fn append_kind(
        &self,
        repo_id: &str,
        kind: BlackboardEventKind,
        text: Option<String>,
        cwd: Option<&Path>,
    ) {
        let joined = {
            let joined = self.joined.lock().await;
            let Some(repo) = joined.get(repo_id) else {
                return;
            };
            repo.clone()
        };
        self.append_to_repo(&joined, kind, text, cwd).await;
    }

    async fn append_to_repo(
        &self,
        joined: &JoinedRepo,
        kind: BlackboardEventKind,
        text: Option<String>,
        cwd: Option<&Path>,
    ) {
        let now = now_unix_seconds();
        let lease_until =
            now.saturating_add(i64::try_from(self.config.stale_after_seconds).unwrap_or(i64::MAX));
        let event = BlackboardEvent {
            kind,
            session_id: self.identity.session_id.clone(),
            thread_id: self.identity.thread_id.clone(),
            pid: self.identity.pid,
            host: self.identity.host.clone(),
            repo_id: joined.repo_id.clone(),
            repo_root: joined.repo_root.to_string_lossy().to_string(),
            branch: joined.branch.clone(),
            cwd: cwd.map(|cwd| cwd.to_string_lossy().to_string()),
            created_at: now,
            lease_until,
            text,
            sequence: self.next_sequence.fetch_add(1, Ordering::Relaxed),
        };
        let stale_lock_after = Duration::from_secs(self.config.stale_after_seconds);
        match append_event(
            &joined.blackboard_path,
            stale_lock_after,
            self.config.max_file_bytes,
            &event,
        ) {
            Ok(()) => {
                self.append_global_index(joined, lease_until, now, stale_lock_after);
            }
            Err(BlackboardError::FileTooLarge { .. }) => {
                self.clear_oversized_file_if_safe(joined, now);
            }
            Err(BlackboardError::LockBusy(path)) => {
                debug!("blackboard lock busy at {}", path.display());
            }
            Err(err) => {
                warn!("failed to append blackboard event: {err}");
            }
        }
    }

    fn append_global_index(
        &self,
        joined: &JoinedRepo,
        lease_until: i64,
        updated_at: i64,
        stale_lock_after: Duration,
    ) {
        let record = GlobalIndexRecord {
            session_id: self.identity.session_id.clone(),
            thread_id: self.identity.thread_id.clone(),
            repo_id: joined.repo_id.clone(),
            repo_root: joined.repo_root.to_string_lossy().to_string(),
            blackboard_path: joined.blackboard_path.to_string_lossy().to_string(),
            lease_until,
            updated_at,
        };
        if let Err(err) =
            append_global_index_record(&self.global_index_path, stale_lock_after, &record)
        {
            debug!("failed to update blackboard global index: {err}");
        }
    }

    fn clear_oversized_file_if_safe(&self, joined: &JoinedRepo, now: i64) {
        match read_events(&joined.blackboard_path) {
            Ok(events) if has_active_external_session(&events, &self.identity.session_id, now) => {
                warn!(
                    "blackboard {} is oversized and has active external sessions; leaving it in place",
                    joined.blackboard_path.display()
                );
            }
            Ok(_) => {
                if let Err(err) = clear_if_no_active_external(
                    &joined.blackboard_path,
                    &self.identity.session_id,
                    now,
                    Duration::from_secs(self.config.stale_after_seconds),
                ) {
                    warn!(
                        "failed to clear oversized blackboard {}: {err}",
                        joined.blackboard_path.display()
                    );
                }
            }
            Err(err) => {
                warn!(
                    "failed to inspect oversized blackboard {}: {err}",
                    joined.blackboard_path.display()
                );
            }
        }
    }

    async fn delete_repo_file_if_alone(&self, repo_id: &str) {
        let joined = {
            let joined = self.joined.lock().await;
            let Some(repo) = joined.get(repo_id) else {
                return;
            };
            repo.clone()
        };
        let now = now_unix_seconds();
        if let Err(err) = clear_if_no_active_external(
            &joined.blackboard_path,
            &self.identity.session_id,
            now,
            Duration::from_secs(self.config.stale_after_seconds),
        ) {
            debug!("failed to delete idle blackboard: {err}");
        }
    }
}

fn update_key(events: &[BlackboardEvent]) -> Option<String> {
    events.last().map(|event| {
        format!(
            "{}:{}:{}:{}",
            event.session_id,
            event.kind.as_str(),
            event.sequence,
            event.created_at
        )
    })
}

fn render_update(update: &crate::ExternalUpdate, max_bytes: usize) -> String {
    let mut text = format!(
        "<blackboard_update repo_root=\"{}\">\n",
        xml_escape(&update.repo_root)
    );
    for event in &update.events {
        let body = event.text.as_deref().unwrap_or("");
        text.push_str(&format!(
            "- {} from session {}: {}\n",
            event.kind.as_str(),
            event.session_id,
            body.replace('\n', " ")
        ));
    }
    text.push_str("</blackboard_update>");
    truncate_utf8_bytes(&text, max_bytes)
}

fn truncate_utf8_bytes(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = max_bytes.saturating_sub(3);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    format!("{}...", &text[..end])
}

fn host_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn current_branch(path: &Path) -> Option<String> {
    let repo_root = find_git_repo_root(path)?;
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!branch.is_empty()).then_some(branch)
}

fn find_git_repo_root(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };
    start
        .ancestors()
        .find(|candidate| candidate.join(".git").exists())
        .map(Path::to_path_buf)
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;

    fn test_config() -> BlackboardSessionConfig {
        BlackboardSessionConfig {
            path: ".codex/blackboard.md".to_string(),
            global_index_path: "blackboard/index.jsonl".to_string(),
            poll_interval_ms: 250,
            heartbeat_interval_seconds: 5,
            stale_after_seconds: 30,
            recent_window_seconds: 600,
            max_injected_bytes: 2048,
            max_entry_chars: 256,
            max_file_bytes: 262_144,
            max_joined_repos: 16,
            enabled: true,
        }
    }

    fn make_repo() -> tempfile::TempDir {
        let repo = tempfile::tempdir().unwrap();
        fs::create_dir(repo.path().join(".git")).unwrap();
        repo
    }

    fn make_manager(
        repo_parent: &Path,
        session_id: &str,
        thread_id: &str,
        mode: BlackboardSessionMode,
    ) -> Arc<BlackboardSession> {
        make_manager_with_config(repo_parent, test_config(), session_id, thread_id, mode)
    }

    fn make_manager_with_config(
        repo_parent: &Path,
        config: BlackboardSessionConfig,
        session_id: &str,
        thread_id: &str,
        mode: BlackboardSessionMode,
    ) -> Arc<BlackboardSession> {
        BlackboardSession::new(BlackboardSessionOptions {
            config,
            codex_home: repo_parent.join("codex-home"),
            session_id: session_id.to_string(),
            thread_id: thread_id.to_string(),
            mode,
        })
    }

    #[tokio::test]
    async fn default_disabled_session_does_not_create_blackboard() {
        let repo = make_repo();
        let manager = make_manager_with_config(
            repo.path(),
            BlackboardSessionConfig {
                enabled: false,
                ..test_config()
            },
            "own",
            "thread-own",
            BlackboardSessionMode::Root,
        );

        let context = manager
            .context_for_turn(repo.path(), "work on feature")
            .await;
        manager.observe_path(repo.path()).await;

        assert_eq!(context, None);
        assert!(!repo.path().join(".codex/blackboard.md").exists());
    }

    #[tokio::test]
    async fn solo_session_records_intent_without_context_injection() {
        let repo = make_repo();
        let manager = make_manager(
            repo.path(),
            "own",
            "thread-own",
            BlackboardSessionMode::Root,
        );

        let context = manager
            .context_for_turn(repo.path(), "work on feature")
            .await;

        assert_eq!(context, None);
        let events = read_events(&repo.path().join(".codex/blackboard.md")).unwrap();
        assert_eq!(
            events.iter().map(|event| event.kind).collect::<Vec<_>>(),
            vec![
                BlackboardEventKind::Join,
                BlackboardEventKind::Intent,
                BlackboardEventKind::Heartbeat
            ]
        );
    }

    #[tokio::test]
    async fn external_active_update_queues_one_context_and_then_proposal() {
        let repo = make_repo();
        let manager = make_manager(
            repo.path(),
            "own",
            "thread-own",
            BlackboardSessionMode::Root,
        );
        let external = make_manager(
            repo.path(),
            "external",
            "thread-external",
            BlackboardSessionMode::Root,
        );
        external
            .context_for_turn(repo.path(), "I will edit parser")
            .await;

        let context = manager
            .context_for_turn(repo.path(), "my plan")
            .await
            .unwrap();
        manager
            .record_assistant_proposal(repo.path(), "I will stay out of parser")
            .await;
        let second_context = manager.context_for_turn(repo.path(), "next turn").await;

        assert!(context.contains("<blackboard_update"));
        assert!(context.contains("I will edit parser"));
        assert_eq!(second_context, None);
        let events = read_events(&repo.path().join(".codex/blackboard.md")).unwrap();
        assert!(events.iter().any(|event| {
            event.kind == BlackboardEventKind::Proposal
                && event.text.as_deref() == Some("I will stay out of parser")
        }));
    }

    #[tokio::test]
    async fn stale_external_update_is_not_injected() {
        let repo = make_repo();
        let manager = make_manager(
            repo.path(),
            "own",
            "thread-own",
            BlackboardSessionMode::Root,
        );
        let external = BlackboardEvent {
            kind: BlackboardEventKind::Intent,
            session_id: "external".to_string(),
            thread_id: "thread-external".to_string(),
            pid: 1,
            host: "host".to_string(),
            repo_id: repo_id_for_root(repo.path()),
            repo_root: repo.path().to_string_lossy().to_string(),
            branch: None,
            cwd: Some(repo.path().to_string_lossy().to_string()),
            created_at: 1,
            lease_until: 2,
            text: Some("stale".to_string()),
            sequence: 1,
        };
        append_event(
            &repo.path().join(".codex/blackboard.md"),
            Duration::from_secs(0),
            262_144,
            &external,
        )
        .unwrap();

        let context = manager.context_for_turn(repo.path(), "my plan").await;

        assert_eq!(context, None);
    }

    #[tokio::test]
    async fn subagent_sessions_do_not_create_blackboards() {
        let repo = make_repo();
        let manager = make_manager(
            repo.path(),
            "sub",
            "thread-sub",
            BlackboardSessionMode::NonRootAgent,
        );

        let context = manager.context_for_turn(repo.path(), "subagent work").await;

        assert_eq!(context, None);
        assert!(!repo.path().join(".codex/blackboard.md").exists());
    }

    #[tokio::test]
    async fn multi_repo_session_records_both_repos_in_global_index() {
        let repo_a = make_repo();
        let repo_b = make_repo();
        let manager = make_manager(
            repo_a.path(),
            "own",
            "thread-own",
            BlackboardSessionMode::Root,
        );

        manager.context_for_turn(repo_a.path(), "work a").await;
        manager.observe_path(repo_b.path()).await;

        let index =
            fs::read_to_string(repo_a.path().join("codex-home/blackboard/index.jsonl")).unwrap();
        assert!(index.contains(&repo_id_for_root(repo_a.path())));
        assert!(index.contains(&repo_id_for_root(repo_b.path())));
    }
}
