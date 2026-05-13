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
        use std::io::Write;
        use std::path::Path;

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
            let bridge_script = BRIDGE_SCRIPT.replace("\r\n", "\n");
            assert!(bridge_script.contains(
                "'dab_click' {\n            $hasTarget = Test-HasTarget $ArgsObj\n            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }\n            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }"
            ));
            assert!(bridge_script.contains(
                "'dab_send_keys' {\n            $hasTarget = Test-HasTarget $ArgsObj\n            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }\n            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }"
            ));
            assert!(bridge_script.contains(
                "'dab_screenshot' {\n            $hasTarget = Test-HasTarget $ArgsObj\n            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }\n            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }"
            ));
        }

        #[test]
        fn bridge_script_exposes_extended_dab_capabilities() {
            for needle in [
                "'dab_drag' {",
                "'dab_scroll' {",
                "'dab_prepare_window' {",
                "'dab_locate_visual' {",
                "'dab_terminal_tabs' {",
                "'dab_terminal_focus' {",
                "CodexDabVision",
                "outer_rect",
                "click_point",
                "Test-RectOffScreen",
                "MoveWindow",
                "IsIconic",
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

        fn write_visual_fixture_bmp(path: &Path) {
            const WIDTH: usize = 420;
            const HEIGHT: usize = 180;
            let mut pixels = vec![255_u8; WIDTH * HEIGHT * 3];

            let mut set_pixel = |x: usize, y: usize, rgb: [u8; 3]| {
                if x >= WIDTH || y >= HEIGHT {
                    return;
                }
                let idx = (y * WIDTH + x) * 3;
                pixels[idx] = rgb[0];
                pixels[idx + 1] = rgb[1];
                pixels[idx + 2] = rgb[2];
            };

            let mut draw_rect = |left: usize,
                                 top: usize,
                                 right: usize,
                                 bottom: usize,
                                 rgb: [u8; 3],
                                 thickness: usize| {
                for t in 0..thickness {
                    for x in left..=right {
                        set_pixel(x, top + t, rgb);
                        set_pixel(x, bottom.saturating_sub(t), rgb);
                    }
                    for y in top..=bottom {
                        set_pixel(left + t, y, rgb);
                        set_pixel(right.saturating_sub(t), y, rgb);
                    }
                }
            };

            draw_rect(24, 36, 350, 96, [45, 45, 45], 2);
            draw_rect(44, 54, 70, 80, [0, 0, 0], 2);
            draw_rect(84, 54, 304, 80, [215, 215, 215], 1);

            for (x, y) in [
                (50, 68),
                (52, 70),
                (54, 72),
                (56, 74),
                (58, 72),
                (60, 70),
                (62, 66),
                (64, 62),
                (66, 58),
            ] {
                set_pixel(x, y, [35, 35, 35]);
                set_pixel(x + 1, y, [35, 35, 35]);
            }

            let row_stride = (WIDTH * 3).div_ceil(4) * 4;
            let pixel_bytes = row_stride * HEIGHT;
            let file_size = 14 + 40 + pixel_bytes;
            let mut file = Vec::with_capacity(file_size);
            file.extend_from_slice(b"BM");
            file.extend_from_slice(&(file_size as u32).to_le_bytes());
            file.extend_from_slice(&[0, 0, 0, 0]);
            file.extend_from_slice(&(54_u32).to_le_bytes());
            file.extend_from_slice(&(40_u32).to_le_bytes());
            file.extend_from_slice(&(WIDTH as i32).to_le_bytes());
            file.extend_from_slice(&(HEIGHT as i32).to_le_bytes());
            file.extend_from_slice(&(1_u16).to_le_bytes());
            file.extend_from_slice(&(24_u16).to_le_bytes());
            file.extend_from_slice(&(0_u32).to_le_bytes());
            file.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
            file.extend_from_slice(&(2835_i32).to_le_bytes());
            file.extend_from_slice(&(2835_i32).to_le_bytes());
            file.extend_from_slice(&(0_u32).to_le_bytes());
            file.extend_from_slice(&(0_u32).to_le_bytes());

            let padding = vec![0_u8; row_stride - WIDTH * 3];
            for y in (0..HEIGHT).rev() {
                for x in 0..WIDTH {
                    let idx = (y * WIDTH + x) * 3;
                    file.push(pixels[idx + 2]);
                    file.push(pixels[idx + 1]);
                    file.push(pixels[idx]);
                }
                file.extend_from_slice(&padding);
            }

            let mut handle = std::fs::File::create(path).expect("create visual fixture bmp");
            handle.write_all(&file).expect("write visual fixture bmp");
        }

        #[test]
        fn execute_dab_locate_visual_detects_challenge_box() {
            let path = std::env::temp_dir().join(format!(
                "codex-dab-visual-fixture-{}.bmp",
                std::process::id()
            ));
            write_visual_fixture_bmp(&path);

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("create DAB locator test runtime");
            let result = runtime.block_on(async {
                execute_dab_tool(
                    "dab_locate_visual",
                    serde_json::json!({
                        "path": path.to_string_lossy(),
                        "kind": "captcha",
                        "max_candidates": 3,
                    }),
                )
                .await
                .expect("dab_locate_visual should return JSON")
            });
            let _ = std::fs::remove_file(&path);

            assert!(result.ok);
            assert_eq!(
                result.output.get("kind").and_then(Value::as_str),
                Some("captcha")
            );
            assert!(
                result
                    .output
                    .get("elapsed_ms")
                    .and_then(Value::as_f64)
                    .is_some_and(|elapsed| elapsed < 100.0),
                "visual locator should do the image analysis in under 100 ms: {:?}",
                result.output.get("elapsed_ms")
            );

            let candidates = result
                .output
                .get("candidates")
                .and_then(Value::as_array)
                .expect("visual locator should return candidates");
            let first = candidates
                .first()
                .expect("visual locator should find challenge");
            assert_eq!(first.get("kind").and_then(Value::as_str), Some("captcha"));
            assert!(first.get("outer_rect").is_some());
            assert!(first.get("inner_rect").is_some());
            let click_x = first
                .get("click_point")
                .and_then(|point| point.get("x"))
                .and_then(Value::as_i64)
                .expect("challenge candidate should include click x");
            let click_y = first
                .get("click_point")
                .and_then(|point| point.get("y"))
                .and_then(Value::as_i64)
                .expect("challenge candidate should include click y");
            assert!((55..=58).contains(&click_x));
            assert!((65..=68).contains(&click_y));
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
