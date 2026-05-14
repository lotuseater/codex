use serde::Deserialize;
use serde::Serialize;
use sha1::Digest;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use thiserror::Error;

const MARKER_PREFIX: &str = "<!-- codex:blackboard ";
const MARKER_SUFFIX: &str = " -->";
const HEADER: &str = "# Codex Blackboard\n\nThis ignored runtime file lets Codex sessions coordinate parallel work in this checkout.\n\n";

#[derive(Debug, Error)]
pub enum BlackboardError {
    #[error("blackboard lock is busy: {0}")]
    LockBusy(PathBuf),
    #[error(
        "blackboard file is over the configured size cap: {path} ({bytes} bytes > {max_bytes} bytes)"
    )]
    FileTooLarge {
        path: PathBuf,
        bytes: u64,
        max_bytes: u64,
    },
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, BlackboardError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlackboardEventKind {
    Join,
    Heartbeat,
    Intent,
    ExternalUpdateSeen,
    Proposal,
    Leave,
    StaleClear,
    ClearRequested,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlackboardEvent {
    pub kind: BlackboardEventKind,
    pub session_id: String,
    pub thread_id: String,
    pub pid: u32,
    pub host: String,
    pub repo_id: String,
    pub repo_root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    pub created_at: i64,
    pub lease_until: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalIndexRecord {
    pub session_id: String,
    pub thread_id: String,
    pub repo_id: String,
    pub repo_root: String,
    pub blackboard_path: String,
    pub lease_until: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalUpdate {
    pub repo_id: String,
    pub repo_root: String,
    pub events: Vec<BlackboardEvent>,
}

pub fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

pub fn repo_id_for_root(repo_root: &Path) -> String {
    let canonical = fs::canonicalize(repo_root).unwrap_or_else(|_| repo_root.to_path_buf());
    let normalized = canonical
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    let digest = sha1::Sha1::digest(normalized.as_bytes());
    hex::encode(digest)
}

pub fn repo_blackboard_path(repo_root: &Path, configured_path: &Path) -> PathBuf {
    if configured_path.is_absolute() {
        configured_path.to_path_buf()
    } else {
        repo_root.join(configured_path)
    }
}

pub fn repo_lock_path(blackboard_path: &Path) -> PathBuf {
    blackboard_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("blackboard.lock")
}

pub fn truncate_text(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let mut iter = text.chars();
    let truncated = iter.by_ref().take(max_chars).collect::<String>();
    if iter.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

pub fn parse_events(contents: &str) -> Vec<BlackboardEvent> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let json = trimmed
                .strip_prefix(MARKER_PREFIX)?
                .strip_suffix(MARKER_SUFFIX)?;
            serde_json::from_str::<BlackboardEvent>(json).ok()
        })
        .collect()
}

pub fn read_events(path: &Path) -> io::Result<Vec<BlackboardEvent>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(parse_events(&contents)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

pub fn append_event(
    blackboard_path: &Path,
    stale_lock_after: Duration,
    max_file_bytes: u64,
    event: &BlackboardEvent,
) -> Result<()> {
    if let Some(parent) = blackboard_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let _guard = LockGuard::acquire(&repo_lock_path(blackboard_path), stale_lock_after)?;
    if let Ok(metadata) = fs::metadata(blackboard_path)
        && max_file_bytes > 0
        && metadata.len() > max_file_bytes
    {
        return Err(BlackboardError::FileTooLarge {
            path: blackboard_path.to_path_buf(),
            bytes: metadata.len(),
            max_bytes: max_file_bytes,
        });
    }
    if !blackboard_path.exists() {
        fs::write(blackboard_path, HEADER)?;
    }
    let json = serde_json::to_string(event)?;
    let summary = event
        .text
        .as_deref()
        .map(|text| format!(" - {}", text.replace('\n', " ")))
        .unwrap_or_default();
    let line = format!(
        "- `{}` session `{}` seq `{}`{}\n{MARKER_PREFIX}{json}{MARKER_SUFFIX}\n",
        event.kind.as_str(),
        event.session_id,
        event.sequence,
        summary
    );
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(blackboard_path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

pub fn append_global_index_record(
    global_index_path: &Path,
    stale_lock_after: Duration,
    record: &GlobalIndexRecord,
) -> Result<()> {
    if let Some(parent) = global_index_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let lock_path = global_index_path.with_extension("lock");
    let _guard = LockGuard::acquire(&lock_path, stale_lock_after)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(global_index_path)?;
    let json = serde_json::to_string(record)?;
    writeln!(file, "{json}")?;
    Ok(())
}

pub fn active_external_update(
    events: &[BlackboardEvent],
    own_session_id: &str,
    now: i64,
    recent_window_seconds: i64,
) -> Option<ExternalUpdate> {
    let active_sessions = active_sessions(events, now);
    let recent_cutoff = now.saturating_sub(recent_window_seconds.max(0));
    let external_events = events
        .iter()
        .filter(|event| event.session_id != own_session_id)
        .filter(|event| active_sessions.contains_key(&event.session_id))
        .filter(|event| event.created_at >= recent_cutoff)
        .filter(|event| is_actionable_update_kind(event.kind))
        .cloned()
        .collect::<Vec<_>>();
    let first = external_events.first()?;
    Some(ExternalUpdate {
        repo_id: first.repo_id.clone(),
        repo_root: first.repo_root.clone(),
        events: external_events,
    })
}

pub fn has_active_external_session(
    events: &[BlackboardEvent],
    own_session_id: &str,
    now: i64,
) -> bool {
    active_sessions(events, now)
        .keys()
        .any(|session_id| session_id != own_session_id)
}

pub fn clear_if_no_active_external(
    blackboard_path: &Path,
    own_session_id: &str,
    now: i64,
    stale_lock_after: Duration,
) -> Result<bool> {
    let _guard = LockGuard::acquire(&repo_lock_path(blackboard_path), stale_lock_after)?;
    let events = read_events(blackboard_path)?;
    if has_active_external_session(&events, own_session_id, now) {
        return Ok(false);
    }
    match fs::remove_file(blackboard_path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(BlackboardError::Io(err)),
    }
}

fn is_actionable_update_kind(kind: BlackboardEventKind) -> bool {
    matches!(
        kind,
        BlackboardEventKind::Intent | BlackboardEventKind::Proposal
    )
}

fn active_sessions(events: &[BlackboardEvent], now: i64) -> HashMap<String, &BlackboardEvent> {
    let mut latest = HashMap::new();
    for event in events {
        latest.insert(event.session_id.clone(), event);
    }
    latest
        .into_iter()
        .filter(|(_, event)| event.lease_until >= now)
        .filter(|(_, event)| {
            !matches!(
                event.kind,
                BlackboardEventKind::Leave
                    | BlackboardEventKind::StaleClear
                    | BlackboardEventKind::ClearRequested
            )
        })
        .collect()
}

impl BlackboardEventKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Join => "join",
            Self::Heartbeat => "heartbeat",
            Self::Intent => "intent",
            Self::ExternalUpdateSeen => "external_update_seen",
            Self::Proposal => "proposal",
            Self::Leave => "leave",
            Self::StaleClear => "stale_clear",
            Self::ClearRequested => "clear_requested",
        }
    }
}

struct LockGuard {
    path: PathBuf,
}

impl LockGuard {
    fn acquire(path: &Path, stale_lock_after: Duration) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(mut file) => {
                writeln!(file, "{}", now_unix_seconds())?;
                Ok(Self {
                    path: path.to_path_buf(),
                })
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                if lock_is_stale(path, stale_lock_after) {
                    let _ = fs::remove_file(path);
                    match OpenOptions::new().write(true).create_new(true).open(path) {
                        Ok(mut file) => {
                            writeln!(file, "{}", now_unix_seconds())?;
                            Ok(Self {
                                path: path.to_path_buf(),
                            })
                        }
                        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                            Err(BlackboardError::LockBusy(path.to_path_buf()))
                        }
                        Err(err) => Err(err.into()),
                    }
                } else {
                    Err(BlackboardError::LockBusy(path.to_path_buf()))
                }
            }
            Err(err) => Err(err.into()),
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn lock_is_stale(path: &Path, stale_lock_after: Duration) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return true;
    };
    let Ok(modified) = metadata.modified() else {
        return true;
    };
    modified.elapsed().unwrap_or_default() >= stale_lock_after
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::time::Duration;

    fn event(kind: BlackboardEventKind, session_id: &str, created_at: i64) -> BlackboardEvent {
        BlackboardEvent {
            kind,
            session_id: session_id.to_string(),
            thread_id: format!("thread-{session_id}"),
            pid: 42,
            host: "host".to_string(),
            repo_id: "repo".to_string(),
            repo_root: "/repo".to_string(),
            branch: Some("main".to_string()),
            cwd: Some("/repo".to_string()),
            created_at,
            lease_until: created_at + 120,
            text: Some("hello".to_string()),
            sequence: 1,
        }
    }

    #[test]
    fn parse_events_ignores_corrupt_lines() {
        let valid = event(BlackboardEventKind::Join, "a", 10);
        let json = serde_json::to_string(&valid).unwrap();
        let contents = format!(
            "not json\n{MARKER_PREFIX}{{broken{MARKER_SUFFIX}\n{MARKER_PREFIX}{json}{MARKER_SUFFIX}\n"
        );

        assert_eq!(parse_events(&contents), vec![valid]);
    }

    #[test]
    fn append_event_recovers_stale_lock() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(".codex/blackboard.md");
        let lock = repo_lock_path(&path);
        fs::create_dir_all(lock.parent().unwrap()).unwrap();
        fs::write(&lock, "stale").unwrap();

        append_event(
            &path,
            Duration::from_secs(0),
            1024 * 1024,
            &event(BlackboardEventKind::Join, "a", 10),
        )
        .unwrap();

        assert_eq!(read_events(&path).unwrap().len(), 1);
        assert!(!lock.exists());
    }

    #[test]
    fn append_event_reports_oversized_files() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(".codex/blackboard.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "too large").unwrap();

        let err = append_event(
            &path,
            Duration::from_secs(0),
            4,
            &event(BlackboardEventKind::Join, "a", 10),
        )
        .unwrap_err();

        assert!(matches!(err, BlackboardError::FileTooLarge { .. }));
    }

    #[test]
    fn active_external_update_filters_own_stale_and_bookkeeping_events() {
        let events = vec![
            event(BlackboardEventKind::Intent, "own", 100),
            event(BlackboardEventKind::Heartbeat, "external", 105),
            event(BlackboardEventKind::Join, "external-join", 106),
            event(BlackboardEventKind::ExternalUpdateSeen, "external-ack", 107),
            event(BlackboardEventKind::Proposal, "external", 110),
            BlackboardEvent {
                lease_until: 90,
                ..event(BlackboardEventKind::Intent, "stale", 80)
            },
        ];

        let update = active_external_update(&events, "own", 120, 60).unwrap();

        assert_eq!(
            update
                .events
                .iter()
                .map(|event| (&event.session_id, event.kind))
                .collect::<Vec<_>>(),
            vec![(&"external".to_string(), BlackboardEventKind::Proposal)]
        );
    }

    #[test]
    fn inactive_terminal_events_clear_external_sessions() {
        for kind in [
            BlackboardEventKind::Leave,
            BlackboardEventKind::StaleClear,
            BlackboardEventKind::ClearRequested,
        ] {
            let events = vec![
                event(BlackboardEventKind::Heartbeat, "external", 100),
                event(kind, "external", 110),
            ];

            assert!(!has_active_external_session(&events, "own", 120));
        }
    }

    #[test]
    fn clear_if_no_active_external_keeps_active_external_under_lock() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(".codex/blackboard.md");
        append_event(
            &path,
            Duration::from_secs(0),
            1024 * 1024,
            &event(BlackboardEventKind::Join, "own", 100),
        )
        .unwrap();
        append_event(
            &path,
            Duration::from_secs(0),
            1024 * 1024,
            &event(BlackboardEventKind::Intent, "external", 110),
        )
        .unwrap();

        let cleared =
            clear_if_no_active_external(&path, "own", 120, Duration::from_secs(0)).unwrap();

        assert!(!cleared);
        assert_eq!(read_events(&path).unwrap().len(), 2);
    }

    #[test]
    fn repo_id_is_stable_for_same_root() {
        let temp = tempfile::tempdir().unwrap();

        assert_eq!(
            repo_id_for_root(temp.path()),
            repo_id_for_root(&temp.path().join("."))
        );
    }

    #[test]
    fn global_index_records_are_json_lines() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("blackboard/index.jsonl");
        let record = GlobalIndexRecord {
            session_id: "s".to_string(),
            thread_id: "t".to_string(),
            repo_id: "r".to_string(),
            repo_root: "/repo".to_string(),
            blackboard_path: "/repo/.codex/blackboard.md".to_string(),
            lease_until: 10,
            updated_at: 1,
        };

        append_global_index_record(&path, Duration::from_secs(0), &record).unwrap();

        let line = fs::read_to_string(path).unwrap();
        assert_eq!(
            serde_json::from_str::<GlobalIndexRecord>(line.trim()).unwrap(),
            record
        );
    }
}
