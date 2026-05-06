use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use serde::Deserialize;
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::debug;

use crate::tools::registry::PostToolUsePayload;
use crate::tools::registry::PreToolUsePayload;

const ENABLE_ENV: &str = "WIZARD_CODEX_OPERATION_CACHE";
const BRIDGE_ENV: &str = "WIZARD_CODEX_CACHE_BRIDGE_PY";
const PYTHON_ENV: &str = "WIZARD_CODEX_CACHE_PYTHON";
const TIMEOUT_ENV: &str = "WIZARD_CODEX_CACHE_TIMEOUT_MS";
const DEFAULT_TIMEOUT_MS: u64 = 1_500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationCacheHit {
    pub(crate) text: String,
    pub(crate) duration: Duration,
}

#[derive(Debug, Clone)]
struct OperationCacheConfig {
    bridge: PathBuf,
    python: String,
    timeout: Duration,
}

#[derive(Debug, Deserialize)]
struct LookupResponse {
    #[serde(default)]
    hit: bool,
    #[serde(default)]
    text: String,
}

pub(crate) async fn lookup(payload: &PreToolUsePayload, cwd: &Path) -> Option<OperationCacheHit> {
    let config = OperationCacheConfig::from_env()?;
    let scope = ProjectCacheScope::from_cwd(cwd);
    let input = serde_json::json!({
        "event": cache_event(payload.tool_name.name(), &payload.tool_input, cwd, &scope),
    });
    let started = Instant::now();
    let output = run_bridge(&config, "pre", input, cwd, &scope).await?;
    let response: LookupResponse = serde_json::from_str(&output).ok()?;
    if response.hit {
        Some(OperationCacheHit {
            text: response.text,
            duration: started.elapsed(),
        })
    } else {
        None
    }
}

pub(crate) async fn store(payload: &PostToolUsePayload, cwd: &Path) {
    let Some(config) = OperationCacheConfig::from_env() else {
        return;
    };
    let scope = ProjectCacheScope::from_cwd(cwd);
    let input = serde_json::json!({
        "event": cache_event(payload.tool_name.name(), &payload.tool_input, cwd, &scope),
        "output": output_text(&payload.tool_response),
        "success": true,
    });
    let _ = run_bridge(&config, "post", input, cwd, &scope).await;
}

fn cache_event(
    tool_name: &str,
    tool_input: &Value,
    cwd: &Path,
    scope: &ProjectCacheScope,
) -> Value {
    serde_json::json!({
        "tool_name": tool_name,
        "tool_input": tool_input,
        "cwd": cwd.display().to_string(),
        "repo_root": scope.repo_root.display().to_string(),
        "repo_name": &scope.repo_name,
        "system_cache_namespace": &scope.system_cache_namespace,
    })
}

fn output_text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| serde_json::to_string(value).unwrap_or_default())
}

async fn run_bridge(
    config: &OperationCacheConfig,
    action: &str,
    input: Value,
    cwd: &Path,
    scope: &ProjectCacheScope,
) -> Option<String> {
    let mut command = Command::new(&config.python);
    command
        .arg(&config.bridge)
        .arg(action)
        .current_dir(cwd)
        .env("CODEX_PROJECT_DIR", cwd)
        .env("CODEX_PROJECT_ROOT", &scope.repo_root)
        .env("CODEX_PROJECT_NAME", &scope.repo_name)
        .env(
            "CODEX_PROJECT_CACHE_NAMESPACE",
            &scope.system_cache_namespace,
        )
        .env("WIZARD_AGENT", "codex")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            debug!("operation cache bridge spawn failed: {err}");
            return None;
        }
    };

    if let Some(mut stdin) = child.stdin.take()
        && let Err(err) = stdin.write_all(input.to_string().as_bytes()).await
    {
        let _ = child.kill().await;
        debug!("operation cache bridge stdin write failed: {err}");
        return None;
    }

    match timeout(config.timeout, child.wait_with_output()).await {
        Ok(Ok(output)) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).to_string())
        }
        Ok(Ok(output)) => {
            debug!(
                "operation cache bridge exited with {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
            None
        }
        Ok(Err(err)) => {
            debug!("operation cache bridge wait failed: {err}");
            None
        }
        Err(_) => {
            debug!(
                "operation cache bridge timed out after {}ms",
                config.timeout.as_millis()
            );
            None
        }
    }
}

impl OperationCacheConfig {
    fn from_env() -> Option<Self> {
        if env_disabled(ENABLE_ENV) {
            return None;
        }
        let bridge = bridge_path_from_env_or_default()?;
        let python = std::env::var(PYTHON_ENV).unwrap_or_else(|_| "python".to_string());
        let timeout = timeout_from_env();
        Some(Self {
            bridge,
            python,
            timeout,
        })
    }
}

fn env_disabled(name: &str) -> bool {
    matches!(
        std::env::var(name)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "0" | "false" | "no" | "off"
    )
}

fn bridge_path_from_env_or_default() -> Option<PathBuf> {
    if let Some(bridge) = std::env::var_os(BRIDGE_ENV).map(PathBuf::from) {
        return bridge.exists().then_some(bridge);
    }
    default_bridge_candidates()
        .into_iter()
        .find(|candidate| candidate.exists())
}

fn default_bridge_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for var_name in ["USERPROFILE", "HOME"] {
        let Some(home) = std::env::var_os(var_name).map(PathBuf::from) else {
            continue;
        };
        candidates.push(
            home.join("Documents")
                .join("GitHub")
                .join("Wizard_Erasmus")
                .join("src")
                .join("mcp")
                .join("hooks")
                .join("codex_cache_bridge_cli.py"),
        );
    }
    candidates
}

fn timeout_from_env() -> Duration {
    let millis = std::env::var(TIMEOUT_ENV)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .unwrap_or(DEFAULT_TIMEOUT_MS);
    Duration::from_millis(millis)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectCacheScope {
    repo_root: PathBuf,
    repo_name: String,
    system_cache_namespace: String,
}

impl ProjectCacheScope {
    fn from_cwd(cwd: &Path) -> Self {
        let repo_root = find_repo_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
        let repo_name = repo_name(&repo_root);
        let root_hash = stable_path_hash(&repo_root);
        let namespace_repo_name = cache_component(&repo_name);
        Self {
            repo_root,
            system_cache_namespace: format!("{namespace_repo_name}-{root_hash}"),
            repo_name,
        }
    }
}

fn find_repo_root(cwd: &Path) -> Option<PathBuf> {
    cwd.ancestors()
        .find(|path| path.join(".git").exists())
        .map(Path::to_path_buf)
}

fn repo_name(repo_root: &Path) -> String {
    repo_root
        .file_name()
        .map(|name| name.to_string_lossy())
        .filter(|name| !name.is_empty())
        .map(|name| name.into_owned())
        .unwrap_or_else(|| "workspace".to_string())
}

fn cache_component(value: &str) -> String {
    let mut component = String::with_capacity(value.len());
    let mut pending_separator = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            if pending_separator && !component.is_empty() {
                component.push('-');
            }
            component.push(ch);
            pending_separator = false;
        } else if !component.is_empty() {
            pending_separator = true;
        }
    }

    let component = component
        .trim_matches(|ch| ch == '-' || ch == '.')
        .to_string();
    if component.is_empty() {
        "workspace".to_string()
    } else {
        component
    }
}

fn stable_path_hash(path: &Path) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in normalized_path_for_hash(path).bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn normalized_path_for_hash(path: &Path) -> String {
    let path = path.display().to_string().replace('\\', "/");
    if cfg!(windows) {
        path.to_ascii_lowercase()
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn cache_event_includes_resolved_cwd() {
        let cwd = Path::new(r"C:\repo\subdir");
        let scope = ProjectCacheScope::from_cwd(cwd);
        let event = cache_event("bash", &json!({ "command": "Get-ChildItem" }), cwd, &scope);

        assert_eq!(
            event,
            json!({
                "tool_name": "bash",
                "tool_input": { "command": "Get-ChildItem" },
                "cwd": r"C:\repo\subdir",
                "repo_root": scope.repo_root.display().to_string(),
                "repo_name": &scope.repo_name,
                "system_cache_namespace": &scope.system_cache_namespace,
            })
        );
    }

    #[test]
    fn cache_event_uses_hook_facing_shape() {
        let cwd = Path::new(r"C:\repo");
        let scope = ProjectCacheScope::from_cwd(cwd);
        assert_eq!(
            cache_event(
                "Bash",
                &json!({"command": "Get-Content -Path src/lib.rs"}),
                cwd,
                &scope,
            ),
            json!({
                "tool_name": "Bash",
                "tool_input": {"command": "Get-Content -Path src/lib.rs"},
                "cwd": r"C:\repo",
                "repo_root": scope.repo_root.display().to_string(),
                "repo_name": &scope.repo_name,
                "system_cache_namespace": &scope.system_cache_namespace,
            })
        );
    }

    #[test]
    fn project_cache_scope_uses_git_root_name_for_subdirectories() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let repo = temp_dir.path().join("sample-repo");
        let nested = repo.join("src");
        std::fs::create_dir_all(&nested).expect("create nested");
        std::fs::create_dir(repo.join(".git")).expect("create git marker");

        let scope = ProjectCacheScope::from_cwd(&nested);

        assert_eq!(scope.repo_root, repo);
        assert_eq!(scope.repo_name, "sample-repo");
        assert!(scope.system_cache_namespace.starts_with("sample-repo-"));
    }

    #[test]
    fn project_cache_scope_sanitizes_system_namespace_repo_component() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let repo = temp_dir.path().join("sample repo!(copy)");
        let nested = repo.join("src");
        std::fs::create_dir_all(&nested).expect("create nested");
        std::fs::create_dir(repo.join(".git")).expect("create git marker");

        let scope = ProjectCacheScope::from_cwd(&nested);

        assert_eq!(scope.repo_name, "sample repo!(copy)");
        assert!(
            scope
                .system_cache_namespace
                .starts_with("sample-repo-copy-")
        );
        assert!(!scope.system_cache_namespace.contains(' '));
        assert!(!scope.system_cache_namespace.contains('!'));
        assert!(!scope.system_cache_namespace.contains('('));
        assert!(!scope.system_cache_namespace.contains(')'));
    }

    #[test]
    fn output_text_preserves_strings_and_serializes_structured_values() {
        assert_eq!(output_text(&json!("plain output")), "plain output");
        assert_eq!(output_text(&json!({"ok": true})), "{\"ok\":true}");
    }
}
