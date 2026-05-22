use anyhow::Result;
use anyhow::anyhow;
use notify::RecursiveMode;
use notify::Watcher;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
use std::time::Instant;
use tokio::task;
use walkdir::WalkDir;

pub async fn wait_for_path_exists(path: impl Into<PathBuf>, timeout: Duration) -> Result<PathBuf> {
    let path = path.into();
    task::spawn_blocking(move || wait_for_path_exists_blocking(path, timeout)).await?
}

pub async fn wait_for_matching_file(
    root: impl Into<PathBuf>,
    timeout: Duration,
    predicate: impl FnMut(&Path) -> bool + Send + 'static,
) -> Result<PathBuf> {
    let root = root.into();
    task::spawn_blocking(move || {
        let mut predicate = predicate;
        blocking_find_matching_file(root, timeout, &mut predicate)
    })
    .await?
}

fn wait_for_path_exists_blocking(path: PathBuf, timeout: Duration) -> Result<PathBuf> {
    if path.exists() {
        return Ok(path);
    }

    let watch_root = nearest_existing_ancestor(&path);
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(&watch_root, RecursiveMode::Recursive)?;

    let deadline = Instant::now() + timeout;
    loop {
        if path.exists() {
            return Ok(path);
        }
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        let remaining = deadline.saturating_duration_since(now);
        match rx.recv_timeout(remaining) {
            Ok(Ok(_event)) => {
                if path.exists() {
                    return Ok(path);
                }
            }
            Ok(Err(err)) => return Err(err.into()),
            Err(RecvTimeoutError::Timeout) => break,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    if path.exists() {
        Ok(path)
    } else {
        Err(anyhow!("timed out waiting for {path:?}"))
    }
}

fn blocking_find_matching_file(
    root: PathBuf,
    timeout: Duration,
    predicate: &mut impl FnMut(&Path) -> bool,
) -> Result<PathBuf> {
    let root = wait_for_path_exists_blocking(root, timeout)?;

    if let Some(found) = scan_for_match(&root, predicate) {
        return Ok(found);
    }

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(remaining) {
            Ok(Ok(_event)) => {
                if let Some(found) = scan_for_match(&root, predicate) {
                    return Ok(found);
                }
            }
            Ok(Err(err)) => return Err(err.into()),
            Err(RecvTimeoutError::Timeout) => break,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    if let Some(found) = scan_for_match(&root, predicate) {
        Ok(found)
    } else {
        Err(anyhow!("timed out waiting for matching file in {root:?}"))
    }
}

fn scan_for_match(root: &Path, predicate: &mut impl FnMut(&Path) -> bool) -> Option<PathBuf> {
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }
        if predicate(path) {
            return Some(path.to_path_buf());
        }
    }
    None
}

fn nearest_existing_ancestor(path: &Path) -> PathBuf {
    let mut current = path;
    loop {
        if current.exists() {
            return current.to_path_buf();
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return PathBuf::from("."),
        }
    }
}
