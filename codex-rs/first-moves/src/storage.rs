use crate::types::FirstMove;
use crate::types::FirstMoveKind;
use crate::types::FirstMovesStats;
use crate::types::FirstMovesStorage;
use crate::types::Result;
use crate::types::ToolUseHitRequest;
use sha1::Digest;
use sha1::Sha1;
use sha2::Sha256;
use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqliteJournalMode;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::sqlite::SqliteSynchronous;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

const DB_FILENAME: &str = ".first_moves.db";
const HIT_WINDOW_SECONDS: i64 = 30 * 60;

const SCHEMA_STATEMENTS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS prefetch_log (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        fired_at      INTEGER NOT NULL,
        session_id    TEXT,
        prompt_sha256 TEXT,
        intent        TEXT,
        paths_json    TEXT NOT NULL,
        n_paths       INTEGER NOT NULL,
        confidence    REAL
    )",
    "CREATE INDEX IF NOT EXISTS prefetch_log_time ON prefetch_log(fired_at DESC)",
    "CREATE TABLE IF NOT EXISTS path_freq (
        path        TEXT PRIMARY KEY,
        observed    INTEGER NOT NULL,
        hit_count   INTEGER NOT NULL DEFAULT 0,
        last_seen   INTEGER NOT NULL,
        last_hit_at INTEGER
    )",
    "CREATE TABLE IF NOT EXISTS first_moves_meta (
        key        TEXT PRIMARY KEY,
        value      TEXT NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS prefetch_path_log (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        fired_at      INTEGER NOT NULL,
        session_id    TEXT,
        path          TEXT NOT NULL,
        source_layer  TEXT,
        logic_mode    TEXT,
        hit_count     INTEGER NOT NULL DEFAULT 0,
        last_hit_at   INTEGER
    )",
    "CREATE INDEX IF NOT EXISTS prefetch_path_log_path_time
        ON prefetch_path_log(path, fired_at DESC)",
];

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PathLearning {
    pub(crate) observed: i64,
    pub(crate) hits: i64,
}

pub fn storage_for(project_root: &Path, codex_home: &Path) -> FirstMovesStorage {
    let normalized_root = resolve_repo_root(project_root);
    let repo_name = normalized_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(safe_path_segment)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "repo".to_string());
    let repo_hash = short_hash(normalized_root.to_string_lossy().as_ref());
    let repo_key = format!("{repo_name}-{repo_hash}");
    let system_db = codex_home
        .join("cache")
        .join("first-moves")
        .join(&repo_key)
        .join("first_moves.sqlite");
    let repo_db = normalized_root.join(DB_FILENAME);
    let repo_db_exists = repo_db.is_file();

    FirstMovesStorage {
        repo_key,
        system_db,
        repo_db: Some(repo_db),
        repo_db_exists,
    }
}

pub(crate) async fn load_learning(storage: &FirstMovesStorage) -> HashMap<String, PathLearning> {
    let mut learning = HashMap::new();
    for db_path in readable_db_paths(storage) {
        let Ok(pool) = open_existing_db(&db_path).await else {
            continue;
        };
        let Ok(rows) = sqlx::query("SELECT path, observed, COALESCE(hit_count, 0) FROM path_freq")
            .fetch_all(&pool)
            .await
        else {
            continue;
        };
        for row in rows {
            let Ok(path) = row.try_get::<String, _>(0) else {
                continue;
            };
            let entry = learning.entry(path).or_insert_with(PathLearning::default);
            entry.observed += row.try_get::<i64, _>(1).unwrap_or_default();
            entry.hits += row.try_get::<i64, _>(2).unwrap_or_default();
        }
    }
    learning
}

pub(crate) async fn record_prediction(
    storage: &FirstMovesStorage,
    session_id: Option<&str>,
    prompt: &str,
    intent: &str,
    confidence: f64,
    moves: &[FirstMove],
) -> Result<()> {
    let pool = open_or_create_db(&storage.system_db).await?;
    let now = now_seconds();
    let read_paths = moves
        .iter()
        .filter(|entry| matches!(entry.kind, FirstMoveKind::Read))
        .filter_map(|entry| entry.path.as_ref())
        .map(|path| normalize_path_text(path.to_string_lossy().as_ref()))
        .collect::<Vec<_>>();
    let paths_json = serde_json::to_string(&read_paths)?;
    sqlx::query(
        "INSERT INTO prefetch_log
        (fired_at, session_id, prompt_sha256, intent, paths_json, n_paths, confidence)
        VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(now)
    .bind(session_id)
    .bind(sha256_hex(prompt))
    .bind(intent)
    .bind(paths_json)
    .bind(i64::try_from(read_paths.len()).unwrap_or(i64::MAX))
    .bind(confidence)
    .execute(&pool)
    .await?;

    for entry in moves
        .iter()
        .filter(|entry| matches!(entry.kind, FirstMoveKind::Read))
    {
        let Some(path) = entry.path.as_ref() else {
            continue;
        };
        let path = normalize_path_text(path.to_string_lossy().as_ref());
        sqlx::query(
            "INSERT INTO path_freq(path, observed, last_seen)
            VALUES (?, 1, ?)
            ON CONFLICT(path) DO UPDATE SET
                observed = observed + 1,
                last_seen = excluded.last_seen",
        )
        .bind(&path)
        .bind(now)
        .execute(&pool)
        .await?;
        sqlx::query(
            "INSERT INTO prefetch_path_log
            (fired_at, session_id, path, source_layer, logic_mode, hit_count, last_hit_at)
            VALUES (?, ?, ?, ?, NULL, 0, NULL)",
        )
        .bind(now)
        .bind(session_id)
        .bind(&path)
        .bind(&entry.source_layer)
        .execute(&pool)
        .await?;
    }

    Ok(())
}

pub async fn record_tool_use_hit(request: ToolUseHitRequest<'_>) -> Result<()> {
    let storage = storage_for(request.project_root, request.codex_home);
    let haystack = normalize_path_text(&format!("{} {}", request.tool_name, request.tool_input));
    let now = now_seconds();
    let since = now.saturating_sub(HIT_WINDOW_SECONDS);

    for db_path in writable_db_paths(&storage) {
        if !db_path.is_file() {
            continue;
        }
        let pool = open_or_create_db(&db_path).await?;
        let rows = sqlx::query(
            "SELECT id, path FROM prefetch_path_log
            WHERE fired_at >= ? AND COALESCE(hit_count, 0) = 0",
        )
        .bind(since)
        .fetch_all(&pool)
        .await?;

        for row in rows {
            let id = row.try_get::<i64, _>(0).unwrap_or_default();
            let path = row.try_get::<String, _>(1).unwrap_or_default();
            if !tool_input_mentions_path(&haystack, &path) {
                continue;
            }
            sqlx::query(
                "UPDATE prefetch_path_log
                SET hit_count = hit_count + 1, last_hit_at = ?
                WHERE id = ?",
            )
            .bind(now)
            .bind(id)
            .execute(&pool)
            .await?;
            sqlx::query(
                "INSERT INTO path_freq(path, observed, hit_count, last_seen, last_hit_at)
                VALUES (?, 0, 1, ?, ?)
                ON CONFLICT(path) DO UPDATE SET
                    hit_count = hit_count + 1,
                    last_hit_at = excluded.last_hit_at",
            )
            .bind(&path)
            .bind(now)
            .bind(now)
            .execute(&pool)
            .await?;
        }
    }

    Ok(())
}

pub async fn stats(project_root: &Path, codex_home: &Path) -> Result<FirstMovesStats> {
    let storage = storage_for(project_root, codex_home);
    let mut prediction_rows = 0;
    let mut predicted_path_rows = 0;
    let mut hit_count = 0;
    let mut hit_path_rows = 0;
    let mut learned_path_rows = 0;

    for db_path in readable_db_paths(&storage) {
        let Ok(pool) = open_existing_db(&db_path).await else {
            continue;
        };
        prediction_rows += scalar_i64(&pool, "SELECT COUNT(*) FROM prefetch_log").await;
        predicted_path_rows += scalar_i64(&pool, "SELECT COUNT(*) FROM prefetch_path_log").await;
        hit_count += scalar_i64(
            &pool,
            "SELECT COALESCE(SUM(hit_count), 0) FROM prefetch_path_log",
        )
        .await;
        hit_path_rows += scalar_i64(
            &pool,
            "SELECT COUNT(*) FROM prefetch_path_log WHERE COALESCE(hit_count, 0) > 0",
        )
        .await;
        learned_path_rows += scalar_i64(&pool, "SELECT COUNT(*) FROM path_freq").await;
    }

    Ok(FirstMovesStats {
        repo_key: storage.repo_key.clone(),
        storage,
        prediction_rows,
        predicted_path_rows,
        hit_count,
        hit_path_rows,
        learned_path_rows,
    })
}

async fn scalar_i64(pool: &SqlitePool, query: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(query)
        .fetch_one(pool)
        .await
        .unwrap_or_default()
}

async fn open_existing_db(path: &Path) -> Result<SqlitePool> {
    let options = sqlite_options(path, /*create_if_missing*/ false);
    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?)
}

async fn open_or_create_db(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let options = sqlite_options(path, /*create_if_missing*/ true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    for statement in SCHEMA_STATEMENTS {
        sqlx::query(statement).execute(&pool).await?;
    }
    Ok(pool)
}

fn sqlite_options(path: &Path, create_if_missing: bool) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(create_if_missing)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_millis(100))
}

fn readable_db_paths(storage: &FirstMovesStorage) -> Vec<PathBuf> {
    let mut paths = vec![storage.system_db.clone()];
    if storage.repo_db_exists
        && let Some(repo_db) = storage.repo_db.as_ref()
    {
        paths.push(repo_db.clone());
    }
    paths
}

fn writable_db_paths(storage: &FirstMovesStorage) -> Vec<PathBuf> {
    readable_db_paths(storage)
}

fn tool_input_mentions_path(haystack: &str, path: &str) -> bool {
    if path.len() < 4 {
        return false;
    }
    if haystack.contains(path) {
        return true;
    }
    let backslash_path = path.replace('/', "\\");
    if haystack.contains(&backslash_path) {
        return true;
    }
    path.rsplit('/')
        .next()
        .is_some_and(|name| name.len() >= 6 && haystack.contains(name))
}

pub(crate) fn normalize_path_text(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

pub(crate) fn short_hash(value: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(value.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    digest.chars().take(12).collect()
}

fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn normalize_root(project_root: &Path) -> PathBuf {
    project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf())
}

pub(crate) fn resolve_repo_root(project_root: &Path) -> PathBuf {
    let normalized_root = normalize_root(project_root);
    normalized_root
        .ancestors()
        .find(|path| path.join(".git").exists())
        .map(Path::to_path_buf)
        .unwrap_or(normalized_root)
}

fn safe_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolUseHitRequest;
    use pretty_assertions::assert_eq;

    #[test]
    fn system_storage_namespace_includes_repo_name_and_hash() {
        let temp = tempfile::tempdir().expect("temp dir");
        let codex_home = temp.path().join("codex-home");
        let project = temp.path().join("src");
        std::fs::create_dir_all(&project).expect("project dir");

        let storage = storage_for(&project, &codex_home);

        assert!(storage.repo_key.starts_with("src-"));
        assert!(storage.system_db.ends_with("first_moves.sqlite"));
    }

    #[test]
    fn system_storage_namespace_uses_enclosing_repo_name_for_subdirectories() {
        let temp = tempfile::tempdir().expect("temp dir");
        let codex_home = temp.path().join("codex-home");
        let repo = temp.path().join("actual-repo");
        let project = repo.join("src");
        std::fs::create_dir_all(&project).expect("project dir");
        std::fs::create_dir(repo.join(".git")).expect("git marker");

        let storage = storage_for(&project, &codex_home);

        assert!(storage.repo_key.starts_with("actual-repo-"));
        assert_eq!(storage.repo_db, Some(repo.join(DB_FILENAME)));
    }

    #[tokio::test]
    async fn records_tool_hits_for_recent_predictions() {
        let temp = tempfile::tempdir().expect("temp dir");
        let codex_home = temp.path().join("codex-home");
        let project = temp.path().join("repo");
        std::fs::create_dir_all(&project).expect("project dir");
        let storage = storage_for(&project, &codex_home);
        let moves = vec![FirstMove {
            kind: FirstMoveKind::Read,
            confidence: 0.9,
            reason: "explicit path mention".to_string(),
            source_layer: "explicit_path".to_string(),
            path: Some(PathBuf::from("src/lib.rs")),
            query: None,
            excerpt: None,
        }];
        record_prediction(
            &storage,
            Some("session"),
            "read src/lib.rs",
            "implement",
            0.9,
            &moves,
        )
        .await
        .expect("record prediction");

        record_tool_use_hit(ToolUseHitRequest {
            project_root: &project,
            codex_home: &codex_home,
            tool_name: "shell_command",
            tool_input: "Get-Content src/lib.rs",
        })
        .await
        .expect("record hit");

        let stats = stats(&project, &codex_home).await.expect("stats");
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.hit_path_rows, 1);
    }

    #[tokio::test]
    async fn hit_tracking_does_not_create_empty_system_db() {
        let temp = tempfile::tempdir().expect("temp dir");
        let codex_home = temp.path().join("codex-home");
        let project = temp.path().join("repo");
        std::fs::create_dir_all(&project).expect("project dir");
        let storage = storage_for(&project, &codex_home);

        record_tool_use_hit(ToolUseHitRequest {
            project_root: &project,
            codex_home: &codex_home,
            tool_name: "shell_command",
            tool_input: "Get-Content src/lib.rs",
        })
        .await
        .expect("record hit");

        assert!(!storage.system_db.exists());
    }
}
