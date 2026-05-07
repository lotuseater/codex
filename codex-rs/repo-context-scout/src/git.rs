use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use crate::slash_path;
use crate::types::ChangedAreas;
use crate::types::ChangedPath;
use crate::types::Result;
use crate::types::ScoutError;

pub(crate) fn git_root(project_root: &Path) -> Result<PathBuf> {
    if !project_root.exists() {
        return Err(ScoutError::InvalidProjectRoot(project_root.to_path_buf()));
    }
    let Some(output) = git_text(project_root, &["rev-parse", "--show-toplevel"]) else {
        return Ok(project_root.to_path_buf());
    };
    let trimmed = output.trim();
    if trimmed.is_empty() {
        Ok(project_root.to_path_buf())
    } else {
        Ok(PathBuf::from(trimmed))
    }
}

pub(crate) fn current_git_head(project_root: &Path) -> Option<String> {
    git_text(project_root, &["rev-parse", "HEAD"]).map(|text| text.trim().to_string())
}

pub(crate) fn read_changed_areas(project_root: &Path) -> ChangedAreas {
    let mut paths = BTreeMap::new();
    if let Some(status) = git_bytes(
        project_root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    ) {
        parse_porcelain_z(&status, &mut paths);
    }
    if let Some(untracked) = git_text(
        project_root,
        &["ls-files", "--others", "--exclude-standard"],
    ) {
        for line in untracked
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            paths
                .entry(line.replace('\\', "/"))
                .or_insert_with(|| "??".to_string());
        }
    }
    ChangedAreas {
        paths: paths
            .into_iter()
            .map(|(path, status)| ChangedPath { path, status })
            .collect(),
    }
}

fn parse_porcelain_z(bytes: &[u8], paths: &mut BTreeMap<String, String>) {
    let text = String::from_utf8_lossy(bytes);
    let mut parts = text.split('\0').filter(|part| !part.is_empty()).peekable();
    while let Some(part) = parts.next() {
        if part.len() < 4 {
            continue;
        }
        let status = part.chars().take(2).collect::<String>();
        let path = part[3..].replace('\\', "/");
        if status.contains('R') || status.contains('C') {
            if let Some(new_path) = parts.next() {
                paths.insert(new_path.replace('\\', "/"), status);
                continue;
            }
        }
        paths.insert(path, status);
    }
}

fn git_text(project_root: &Path, args: &[&str]) -> Option<String> {
    git_bytes(project_root, args).map(|bytes| String::from_utf8_lossy(&bytes).to_string())
}

fn git_bytes(project_root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

pub(crate) fn relative_slash_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    Some(slash_path(relative))
}
