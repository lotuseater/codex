use crate::DesktopAutomationError;
use crate::DesktopAutomationResult;
use serde_json::Value;

#[cfg(not(windows))]
pub async fn execute_dab_tool(
    _tool_name: &str,
    _input: Value,
) -> Result<DesktopAutomationResult, DesktopAutomationError> {
    Err(DesktopAutomationError::Unsupported(
        "native DAB is only available on Windows".to_string(),
    ))
}

#[cfg(windows)]
mod imp {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use std::process::Stdio;
    use std::time::Duration;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;
    use tokio::process::Command;
    use tokio::time::timeout;

    const BRIDGE_TIMEOUT_SECONDS: u64 = 45;

    pub async fn execute_dab_tool(
        tool_name: &str,
        input: Value,
    ) -> Result<DesktopAutomationResult, DesktopAutomationError> {
        let input_json = serde_json::to_vec(&input)
            .map_err(|err| DesktopAutomationError::Bridge(err.to_string()))?;
        let input_b64 = STANDARD.encode(input_json);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let script_path =
            std::env::temp_dir().join(format!("codex-dab-{}-{timestamp}.ps1", std::process::id()));
        std::fs::write(&script_path, BRIDGE_SCRIPT).map_err(DesktopAutomationError::Spawn)?;
        let child = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
            ])
            .arg("-File")
            .arg(&script_path)
            .env("CODEX_DAB_TOOL", tool_name)
            .env("CODEX_DAB_INPUT_B64", input_b64)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| {
                let _ = std::fs::remove_file(&script_path);
                DesktopAutomationError::Spawn(err)
            })?;

        let output = match timeout(
            Duration::from_secs(BRIDGE_TIMEOUT_SECONDS),
            child.wait_with_output(),
        )
        .await
        {
            Ok(result) => match result {
                Ok(output) => output,
                Err(err) => {
                    let _ = std::fs::remove_file(&script_path);
                    return Err(DesktopAutomationError::Spawn(err));
                }
            },
            Err(_) => {
                let _ = std::fs::remove_file(&script_path);
                return Err(DesktopAutomationError::Timeout(BRIDGE_TIMEOUT_SECONDS));
            }
        };
        let _ = std::fs::remove_file(&script_path);

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            return Err(DesktopAutomationError::Bridge(format!(
                "desktop automation bridge failed for `{tool_name}` with status {:?}; stdout: {}; stderr: {}",
                output.status.code(),
                output_part(&stdout),
                output_part(&stderr)
            )));
        }

        let value = parse_bridge_stdout(tool_name, output.status.code(), &stdout, &stderr)?;
        let image_url = value
            .get("image_url")
            .and_then(Value::as_str)
            .map(str::to_string);
        Ok(DesktopAutomationResult::with_image(value, image_url))
    }

    fn parse_bridge_stdout(
        tool_name: &str,
        status_code: Option<i32>,
        stdout: &str,
        stderr: &str,
    ) -> Result<Value, DesktopAutomationError> {
        if stdout.is_empty() {
            return Err(DesktopAutomationError::Bridge(format!(
                "desktop automation bridge returned no JSON for `{tool_name}` with status {status_code:?}; stderr: {}",
                output_part(stderr)
            )));
        }

        serde_json::from_str(stdout).map_err(DesktopAutomationError::InvalidJson)
    }

    fn output_part(text: &str) -> &str {
        if text.is_empty() { "<empty>" } else { text }
    }

    const BRIDGE_SCRIPT: &str = include_str!("dab_bridge_windows.ps1");

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parse_bridge_stdout_reports_empty_output() {
            let err = parse_bridge_stdout("dab_find_window", Some(0), "", "").unwrap_err();

            assert!(matches!(
                err,
                DesktopAutomationError::Bridge(message)
                    if message == "desktop automation bridge returned no JSON for `dab_find_window` with status Some(0); stderr: <empty>"
            ));
        }

        #[test]
        fn parse_bridge_stdout_includes_stderr_for_empty_output() {
            let err =
                parse_bridge_stdout("dab_find_window", Some(0), "", "process failed").unwrap_err();

            assert!(matches!(
                err,
                DesktopAutomationError::Bridge(message)
                    if message == "desktop automation bridge returned no JSON for `dab_find_window` with status Some(0); stderr: process failed"
            ));
        }

        #[test]
        fn bridge_script_guards_targeted_foreground_actions() {
            assert!(BRIDGE_SCRIPT.contains(
                "'dab_click' {\n            $hasTarget = Test-HasTarget $ArgsObj\n            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }\n            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }"
            ));
            assert!(BRIDGE_SCRIPT.contains(
                "'dab_send_keys' {\n            $hasTarget = Test-HasTarget $ArgsObj\n            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }\n            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }"
            ));
            assert!(BRIDGE_SCRIPT.contains(
                "'dab_screenshot' {\n            $hasTarget = Test-HasTarget $ArgsObj\n            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }\n            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }"
            ));
        }

        #[test]
        fn bridge_script_exposes_extended_dab_capabilities() {
            for needle in [
                "'dab_drag' {",
                "'dab_scroll' {",
                "'dab_terminal_tabs' {",
                "'dab_terminal_focus' {",
                "Get-MissingNumericFields $ArgsObj @('x', 'y', 'end_x', 'end_y')",
                "missing or invalid numeric drag fields",
                "Convert-UiRectNumber $rect.X",
                "windows = @(Find-Windows $ArgsObj)",
                "codex_submit|claude_submit",
            ] {
                assert!(
                    BRIDGE_SCRIPT.contains(needle),
                    "bridge script should contain {needle:?}"
                );
            }
        }

        #[test]
        fn execute_dab_find_window_live_canary_when_enabled() {
            if std::env::var_os("CODEX_DAB_LIVE_TEST").is_none() {
                return;
            }

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("create live DAB test runtime");
            let result = runtime.block_on(async {
                execute_dab_tool("dab_find_window", serde_json::json!({ "limit": 5 }))
                    .await
                    .expect("dab_find_window should return JSON")
            });

            assert!(result.ok);
            assert!(result.output.get("windows").is_some_and(Value::is_array));
        }
    }
}

#[cfg(windows)]
pub use imp::execute_dab_tool;
