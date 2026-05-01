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
    let input = serde_json::json!({
        "event": cache_event(payload.tool_name.name(), &payload.tool_input),
    });
    let started = Instant::now();
    let output = run_bridge(&config, "pre", input, cwd).await?;
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
    let input = serde_json::json!({
        "event": cache_event(payload.tool_name.name(), &payload.tool_input),
        "output": output_text(&payload.tool_response),
        "success": true,
    });
    let _ = run_bridge(&config, "post", input, cwd).await;
}

fn cache_event(tool_name: &str, tool_input: &Value) -> Value {
    serde_json::json!({
        "tool_name": tool_name,
        "tool_input": tool_input,
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
) -> Option<String> {
    let mut command = Command::new(&config.python);
    command
        .arg(&config.bridge)
        .arg(action)
        .current_dir(cwd)
        .env("CODEX_PROJECT_DIR", cwd)
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
        if !env_enabled(ENABLE_ENV) {
            return None;
        }
        let bridge = std::env::var_os(BRIDGE_ENV).map(PathBuf::from)?;
        let python = std::env::var(PYTHON_ENV).unwrap_or_else(|_| "python".to_string());
        let timeout = timeout_from_env();
        Some(Self {
            bridge,
            python,
            timeout,
        })
    }
}

fn env_enabled(name: &str) -> bool {
    matches!(
        std::env::var(name)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn timeout_from_env() -> Duration {
    let millis = std::env::var(TIMEOUT_ENV)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .unwrap_or(DEFAULT_TIMEOUT_MS);
    Duration::from_millis(millis)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::cache_event;
    use super::output_text;

    #[test]
    fn cache_event_uses_hook_facing_shape() {
        assert_eq!(
            cache_event("Bash", &json!({"command": "Get-Content -Path src/lib.rs"})),
            json!({
                "tool_name": "Bash",
                "tool_input": {"command": "Get-Content -Path src/lib.rs"},
            })
        );
    }

    #[test]
    fn output_text_preserves_strings_and_serializes_structured_values() {
        assert_eq!(output_text(&json!("plain output")), "plain output");
        assert_eq!(output_text(&json!({"ok": true})), "{\"ok\":true}");
    }
}
