use crate::DesktopAutomationResult;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;
use std::cmp::Ordering;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const DEFAULT_MAX_DEPTH: usize = 5;
const DEFAULT_LIMIT: usize = 80;

#[derive(Debug, Clone, Serialize, PartialEq)]
struct HarnessCandidate {
    path: String,
    provider: &'static str,
    capability: &'static str,
    reason: &'static str,
    confidence: f32,
}

pub fn detect(input: Value, cwd: &Path) -> DesktopAutomationResult {
    let root = input
        .get("root")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| PathBuf::from(value.trim()))
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            }
        })
        .unwrap_or_else(|| cwd.to_path_buf());
    let max_depth = input
        .get("max_depth")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_MAX_DEPTH);
    let limit = input
        .get("limit")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_LIMIT);

    let mut candidates = Vec::new();
    scan_dir(&root, 0, max_depth, &mut candidates);
    candidates.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
    });
    candidates.truncate(limit);

    DesktopAutomationResult::text(json!({
        "ok": true,
        "root": root.display().to_string(),
        "provider_selection": candidates,
        "preferred_order": [
            "app_native_harness",
            "repo_visual_e2e_harness",
            "native_dab",
            "wizard_dab_compatibility",
            "plain_screenshot"
        ]
    }))
}

fn scan_dir(dir: &Path, depth: usize, max_depth: usize, candidates: &mut Vec<HarnessCandidate>) {
    if depth > max_depth || skip_dir(dir) {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            scan_dir(&path, depth + 1, max_depth, candidates);
        } else if file_type.is_file()
            && let Some(candidate) = candidate_for_file(&path)
        {
            candidates.push(candidate);
        }
    }
}

fn skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                ".git" | "target" | "node_modules" | ".next" | "dist" | "build"
            )
        })
        .unwrap_or(false)
}

fn candidate_for_file(path: &Path) -> Option<HarnessCandidate> {
    let file_name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    let path_lower = path
        .display()
        .to_string()
        .replace('\\', "/")
        .to_ascii_lowercase();

    let (provider, capability, reason, confidence) = if file_name == "test_gui_automation.py" {
        (
            "app_native_harness",
            "desktop_gui",
            "test_gui_automation.py",
            0.98,
        )
    } else if file_name == "uiautomationharness.psm1"
        || file_name == "test_ui_automation.ps1"
        || path_lower.contains("/tests/") && file_name.contains("uiautomation")
    {
        (
            "app_native_harness",
            "windows_uia",
            "UIAutomation test harness",
            0.96,
        )
    } else if file_name == "visual_autotest.py" {
        (
            "repo_visual_e2e_harness",
            "visual_e2e",
            "visual_autotest.py",
            0.94,
        )
    } else if file_name == "android_visual_e2e.py" {
        (
            "repo_visual_e2e_harness",
            "android_visual_e2e",
            "android_visual_e2e.py",
            0.92,
        )
    } else if file_name.starts_with("playwright.config.") {
        (
            "repo_visual_e2e_harness",
            "browser_visual_e2e",
            "Playwright config",
            0.9,
        )
    } else if path_lower.contains("/winappdriver/") {
        (
            "app_native_harness",
            "winappdriver",
            "WinAppDriver dependency",
            0.86,
        )
    } else if file_name == "gui_automation.cpp" || file_name == "gui_automation.hpp" {
        (
            "app_native_harness",
            "desktop_gui",
            "native gui_automation source",
            0.84,
        )
    } else {
        return None;
    };

    Some(HarnessCandidate {
        path: path.display().to_string(),
        provider,
        capability,
        reason,
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn finds_known_gui_harness_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let tests = temp.path().join("tests");
        fs::create_dir_all(&tests).expect("create tests");
        fs::write(temp.path().join("test_gui_automation.py"), "").expect("write harness");
        fs::write(tests.join("visual_autotest.py"), "").expect("write visual test");

        let result = detect(json!({}), temp.path());
        let providers = result
            .output
            .get("provider_selection")
            .and_then(Value::as_array)
            .expect("providers");

        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0]["provider"], "app_native_harness");
    }

    #[test]
    fn resolves_relative_scan_root_against_cwd() {
        let temp = tempfile::tempdir().expect("tempdir");
        let app = temp.path().join("app");
        fs::create_dir_all(&app).expect("create app");
        fs::write(app.join("test_gui_automation.py"), "").expect("write harness");

        let result = detect(json!({"root": "app"}), temp.path());
        let root = result
            .output
            .get("root")
            .and_then(Value::as_str)
            .expect("root");

        assert_eq!(root, app.display().to_string());
    }
}
