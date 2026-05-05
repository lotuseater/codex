use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use codex_analytics::CompactionReason;
use codex_protocol::ThreadId;

pub(crate) fn write_scratchpad(
    codex_home: &Path,
    thread_id: ThreadId,
    turn_id: &str,
    reason: CompactionReason,
    git_summary: &str,
) -> io::Result<PathBuf> {
    let dir = codex_home.join("tmp").join("semantic-compaction");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{thread_id}-{turn_id}.md"));
    let body = format!(
        "# Semantic Compaction Scratchpad\n\n- thread: {thread_id}\n- turn: {turn_id}\n- reason: {reason:?}\n- git: {git_summary}\n"
    );
    fs::write(&path, body)?;
    Ok(path)
}

pub(crate) fn cleanup_scratchpad(path: Option<PathBuf>) {
    if let Some(path) = path {
        let _ = fs::remove_file(path);
    }
}
