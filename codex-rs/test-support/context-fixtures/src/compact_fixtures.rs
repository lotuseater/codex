use std::fs;
use std::path::Path;

use serde_json::Value;
use serde_json::json;

pub const FIRST_REPLY: &str = "FIRST_REPLY";
pub const SUMMARY_TEXT: &str = "SUMMARY_ONLY_CONTEXT";
pub const COMPACT_WARNING_MESSAGE: &str = "Heads up: Long threads and multiple compactions can cause the model to be less accurate. Start a new thread when possible to keep threads small and targeted.";

pub fn auto_summary(summary: &str) -> String {
    summary.to_string()
}

pub fn body_contains_text(body: &str, text: &str) -> bool {
    body.contains(&json_fragment(text))
}

fn json_fragment(text: &str) -> String {
    serde_json::to_string(text)
        .expect("serialize text to JSON")
        .trim_matches('"')
        .to_string()
}

pub fn read_hook_inputs(path: &Path) -> Vec<Value> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read hook input log {}: {err}", path.display()));
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|err| panic!("failed to parse hook input log line: {err}"))
        })
        .collect()
}

fn python_hook_command(script_path: &Path) -> String {
    format!("python3 \"{}\"", script_path.display())
}

pub fn write_unsupported_blocking_pre_compact_hook(home: &Path) {
    let script_path = home.join("pre_compact_block.py");
    let log_path = home.join("pre_compact_block_log.jsonl");
    let script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
with Path(r"{log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")

print(json.dumps({{"decision": "block", "reason": "blocked by policy"}}))
"#,
        log_path = log_path.display(),
    );
    let hooks = json!({
        "hooks": {
            "PreCompact": [{
                "matcher": "manual",
                "hooks": [{
                    "type": "command",
                    "command": python_hook_command(&script_path),
                    "statusMessage": "checking compact policy",
                }]
            }]
        }
    });

    fs::write(&script_path, script).expect("write pre compact hook script");
    fs::write(home.join("hooks.json"), hooks.to_string()).expect("write hooks.json");
}

pub fn write_matching_compact_hooks(home: &Path) {
    let auto_script_path = home.join("pre_compact_auto.py");
    let auto_log_path = home.join("pre_compact_auto_log.jsonl");
    let manual_post_script_path = home.join("post_compact_manual.py");
    let manual_post_log_path = home.join("post_compact_manual_log.jsonl");
    let auto_script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
with Path(r"{auto_log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")
"#,
        auto_log_path = auto_log_path.display(),
    );
    let manual_post_script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
with Path(r"{manual_post_log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")
"#,
        manual_post_log_path = manual_post_log_path.display(),
    );
    let hooks = json!({
        "hooks": {
            "PreCompact": [{
                "matcher": "auto",
                "hooks": [{
                    "type": "command",
                    "command": python_hook_command(&auto_script_path),
                }]
            }],
            "PostCompact": [{
                "matcher": "manual",
                "hooks": [{
                    "type": "command",
                    "command": python_hook_command(&manual_post_script_path),
                }]
            }]
        }
    });

    fs::write(&auto_script_path, auto_script).expect("write auto pre compact hook script");
    fs::write(&manual_post_script_path, manual_post_script)
        .expect("write manual post compact hook script");
    fs::write(home.join("hooks.json"), hooks.to_string()).expect("write hooks.json");
}
