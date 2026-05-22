#![cfg(not(target_os = "windows"))]

use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use codex_core_test_runtime::hooks::trust_discovered_hooks;
use codex_core_test_runtime::responses::ev_apply_patch_custom_tool_call;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::mount_sse_sequence;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::responses::start_mock_server;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::test_codex::test_codex;
use pretty_assertions::assert_eq;
use serde_json::Value;

fn write_post_tool_use_hook(
    home: &Path,
    matcher: Option<&str>,
    mode: &str,
    reason: &str,
) -> Result<()> {
    let script_path = home.join("post_tool_use_hook.py");
    let log_path = home.join("post_tool_use_hook_log.jsonl");
    let mode_json = serde_json::to_string(mode).context("serialize post tool use mode")?;
    let reason_json = serde_json::to_string(reason).context("serialize post tool use reason")?;
    let script = format!(
        r#"import json
from pathlib import Path
import sys

log_path = Path(r"{log_path}")
mode = {mode_json}
reason = {reason_json}

payload = json.load(sys.stdin)

with log_path.open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")

if mode == "context":
    print(json.dumps({{
        "hookSpecificOutput": {{
            "hookEventName": "PostToolUse",
            "additionalContext": reason
        }}
    }}))
elif mode == "decision_block":
    print(json.dumps({{
        "decision": "block",
        "reason": reason
    }}))
elif mode == "continue_false":
    print(json.dumps({{
        "continue": False,
        "stopReason": reason
    }}))
elif mode == "exit_2":
    sys.stderr.write(reason + "\n")
    raise SystemExit(2)
"#,
        log_path = log_path.display(),
    );

    let mut group = serde_json::json!({
        "hooks": [{
            "type": "command",
            "command": format!("python3 {}", script_path.display()),
            "statusMessage": "running post tool use hook",
        }]
    });
    if let Some(matcher) = matcher {
        group["matcher"] = Value::String(matcher.to_string());
    }

    let hooks = serde_json::json!({
        "hooks": {
            "PostToolUse": [group]
        }
    });

    fs::write(&script_path, script).context("write post tool use hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

fn read_post_tool_use_hook_inputs(home: &Path) -> Result<Vec<serde_json::Value>> {
    read_hook_inputs_from_log(home.join("post_tool_use_hook_log.jsonl").as_path())
}

fn read_hook_inputs_from_log(log_path: &Path) -> Result<Vec<serde_json::Value>> {
    fs::read_to_string(log_path)
        .with_context(|| format!("read hook log {}", log_path.display()))?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("parse hook log line"))
        .collect()
}

#[tokio::test]
async fn post_tool_use_records_additional_context_for_apply_patch() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "posttooluse-apply-patch";
    let file_name = "post_tool_use_apply_patch.txt";
    let patch = format!(
        r#"*** Begin Patch
*** Add File: {file_name}
+patched
*** End Patch"#
    );
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_apply_patch_custom_tool_call(call_id, &patch),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "apply_patch post hook context observed"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let post_context = "Remember the apply_patch post-tool note.";
    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) =
                write_post_tool_use_hook(home, Some("^apply_patch$"), "context", post_context)
            {
                panic!("failed to write post tool use hook test fixture: {error}");
            }
        })
        .with_config(|config| {
            trust_discovered_hooks(config);
        });
    let test = builder.build(&server).await?;

    test.submit_turn("apply the patch with post hook").await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    assert!(
        requests[1]
            .message_input_texts("developer")
            .contains(&post_context.to_string()),
        "follow-up request should include apply_patch post tool use context",
    );
    assert!(
        test.workspace_path(file_name).exists(),
        "apply_patch should create the file"
    );

    let hook_inputs = read_post_tool_use_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["tool_name"], "apply_patch");
    assert_eq!(hook_inputs[0]["tool_use_id"], call_id);
    assert_eq!(hook_inputs[0]["tool_input"]["command"], patch);
    let tool_response = hook_inputs[0]["tool_response"]
        .as_str()
        .context("apply_patch tool_response should be a string")?;
    assert!(tool_response.starts_with("Exit code: 0"));
    assert!(tool_response.contains("Success. Updated the following files:"));
    assert!(tool_response.contains("A post_tool_use_apply_patch.txt"));

    Ok(())
}

#[tokio::test]
async fn post_tool_use_records_apply_patch_context_with_edit_alias() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "posttooluse-apply-patch-edit";
    let file_name = "post_tool_use_apply_patch_edit.txt";
    let patch = format!(
        r#"*** Begin Patch
*** Add File: {file_name}
+patched
*** End Patch"#
    );
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_apply_patch_custom_tool_call(call_id, &patch),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "apply_patch edit hook context observed"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let post_context = "Remember the edit alias post-tool note.";
    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) =
                write_post_tool_use_hook(home, Some("^Edit$"), "context", post_context)
            {
                panic!("failed to write post tool use hook test fixture: {error}");
            }
        })
        .with_config(|config| {
            trust_discovered_hooks(config);
        });
    let test = builder.build(&server).await?;

    test.submit_turn("apply the patch with edit alias post hook")
        .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    assert!(
        requests[1]
            .message_input_texts("developer")
            .contains(&post_context.to_string()),
        "follow-up request should include apply_patch post tool use context",
    );
    assert!(
        test.workspace_path(file_name).exists(),
        "apply_patch should create the file"
    );

    let hook_inputs = read_post_tool_use_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["tool_name"], "apply_patch");
    assert_eq!(hook_inputs[0]["tool_use_id"], call_id);
    assert_eq!(hook_inputs[0]["tool_input"]["command"], patch);

    Ok(())
}
