//! Protocol/domain fixtures and test utilities that intentionally avoid
//! depending on `codex-core`.

use anyhow::Context as _;
use anyhow::ensure;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_absolute_path::test_support::PathExt as _;
use codex_utils_cargo_bin::CargoBinError;
use regex_lite::Regex;
use std::path::PathBuf;
use tempfile::TempDir;

#[track_caller]
pub fn assert_regex_match<'s>(pattern: &str, actual: &'s str) -> regex_lite::Captures<'s> {
    let regex = Regex::new(pattern).unwrap_or_else(|err| {
        panic!("failed to compile regex {pattern:?}: {err}");
    });
    regex
        .captures(actual)
        .unwrap_or_else(|| panic!("regex {pattern:?} did not match {actual:?}"))
}

pub fn test_path_buf_with_windows(unix_path: &str, windows_path: Option<&str>) -> PathBuf {
    if cfg!(windows) {
        if let Some(windows) = windows_path {
            PathBuf::from(windows)
        } else {
            let mut path = PathBuf::from(r"C:\");
            path.extend(
                unix_path
                    .trim_start_matches('/')
                    .split('/')
                    .filter(|segment| !segment.is_empty()),
            );
            path
        }
    } else {
        PathBuf::from(unix_path)
    }
}

pub fn test_path_buf(unix_path: &str) -> PathBuf {
    test_path_buf_with_windows(unix_path, /*windows_path*/ None)
}

pub fn test_absolute_path_with_windows(
    unix_path: &str,
    windows_path: Option<&str>,
) -> AbsolutePathBuf {
    AbsolutePathBuf::from_absolute_path(test_path_buf_with_windows(unix_path, windows_path))
        .expect("test path should be absolute")
}

pub fn test_absolute_path(unix_path: &str) -> AbsolutePathBuf {
    test_absolute_path_with_windows(unix_path, /*windows_path*/ None)
}

pub trait TempDirExt {
    fn abs(&self) -> AbsolutePathBuf;
}

impl TempDirExt for TempDir {
    fn abs(&self) -> AbsolutePathBuf {
        self.path().abs()
    }
}

pub fn test_tmp_path() -> AbsolutePathBuf {
    test_absolute_path_with_windows("/tmp", Some(r"C:\Users\codex\AppData\Local\Temp"))
}

pub fn test_tmp_path_buf() -> PathBuf {
    test_tmp_path().into_path_buf()
}

/// Fetch a DotSlash resource and return the resolved executable/file path.
pub fn fetch_dotslash_file(
    dotslash_file: &std::path::Path,
    dotslash_cache: Option<&std::path::Path>,
) -> anyhow::Result<PathBuf> {
    let mut command = std::process::Command::new("dotslash");
    command.arg("--").arg("fetch").arg(dotslash_file);
    if let Some(dotslash_cache) = dotslash_cache {
        command.env("DOTSLASH_CACHE", dotslash_cache);
    }
    let output = command.output().with_context(|| {
        format!(
            "failed to run dotslash to fetch resource {}",
            dotslash_file.display()
        )
    })?;
    ensure!(
        output.status.success(),
        "dotslash fetch failed for {}: {}",
        dotslash_file.display(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let fetched_path = String::from_utf8(output.stdout)
        .context("dotslash fetch output was not utf8")?
        .trim()
        .to_string();
    ensure!(!fetched_path.is_empty(), "dotslash fetch output was empty");
    let fetched_path = PathBuf::from(fetched_path);
    ensure!(
        fetched_path.is_file(),
        "dotslash returned non-file path: {}",
        fetched_path.display()
    );
    Ok(fetched_path)
}

pub fn load_sse_fixture_with_id_from_str(raw: &str, id: &str) -> String {
    let replaced = raw.replace("__ID__", id);
    let events: Vec<serde_json::Value> =
        serde_json::from_str(&replaced).expect("parse JSON fixture");
    sse(events)
}

fn sse(events: Vec<serde_json::Value>) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    for event in events {
        let kind = event.get("type").and_then(|value| value.as_str()).unwrap();
        writeln!(&mut out, "event: {kind}").unwrap();
        if !event.as_object().map(|object| object.len() == 1).unwrap_or(false) {
            write!(&mut out, "data: {event}\n\n").unwrap();
        } else {
            out.push('\n');
        }
    }
    out
}

const REMOTE_ENV_ENV_VAR: &str = "CODEX_TEST_REMOTE_ENV";

pub fn remote_env_env_var() -> &'static str {
    REMOTE_ENV_ENV_VAR
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteEnvConfig {
    pub container_name: String,
}

pub fn get_remote_test_env() -> Option<RemoteEnvConfig> {
    if std::env::var_os(REMOTE_ENV_ENV_VAR).is_none() {
        eprintln!("Skipping test because {REMOTE_ENV_ENV_VAR} is not set.");
        return None;
    }

    let container_name = std::env::var(REMOTE_ENV_ENV_VAR)
        .unwrap_or_else(|_| panic!("{REMOTE_ENV_ENV_VAR} must be set"));
    assert!(
        !container_name.trim().is_empty(),
        "{REMOTE_ENV_ENV_VAR} must not be empty"
    );

    Some(RemoteEnvConfig { container_name })
}

pub fn stdio_server_bin() -> Result<String, CargoBinError> {
    codex_utils_cargo_bin::cargo_bin("test_stdio_server").map(|p| p.to_string_lossy().to_string())
}
