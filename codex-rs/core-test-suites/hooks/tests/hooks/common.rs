//! Shared fixtures, hook-script writers, and log readers for the hooks integration tests.

pub(crate) use std::fs;
pub(crate) use std::path::Path;

pub(crate) use anyhow::Context;
pub(crate) use anyhow::Result;
pub(crate) use codex_core::config::Config;
pub(crate) use codex_core::config::Constrained;
pub(crate) use codex_core_test_runtime::hooks::trust_discovered_hooks;
pub(crate) use codex_core_test_runtime::hooks::trust_hooks;
pub(crate) use codex_core_test_runtime::managed_network_requirements_loader;
pub(crate) use codex_core_test_runtime::responses::ev_apply_patch_custom_tool_call;
pub(crate) use codex_core_test_runtime::responses::ev_assistant_message;
pub(crate) use codex_core_test_runtime::responses::ev_completed;
pub(crate) use codex_core_test_runtime::responses::ev_completed_with_tokens;
pub(crate) use codex_core_test_runtime::responses::ev_custom_tool_call;
pub(crate) use codex_core_test_runtime::responses::ev_function_call;
pub(crate) use codex_core_test_runtime::responses::ev_message_item_added;
pub(crate) use codex_core_test_runtime::responses::ev_output_text_delta;
pub(crate) use codex_core_test_runtime::responses::ev_response_created;
pub(crate) use codex_core_test_runtime::responses::mount_compact_json_once;
pub(crate) use codex_core_test_runtime::responses::mount_sse_once;
pub(crate) use codex_core_test_runtime::responses::mount_sse_sequence;
pub(crate) use codex_core_test_runtime::responses::sse;
pub(crate) use codex_core_test_runtime::responses::start_mock_server;
pub(crate) use codex_core_test_runtime::skip_if_no_network;
pub(crate) use codex_core_test_runtime::skip_if_windows;
pub(crate) use codex_core_test_runtime::streaming_sse::StreamingSseChunk;
pub(crate) use codex_core_test_runtime::streaming_sse::start_streaming_sse_server;
pub(crate) use codex_core_test_runtime::test_codex::test_codex;
pub(crate) use codex_core_test_runtime::wait_for_event;
pub(crate) use codex_features::Feature;
pub(crate) use codex_model_provider_info::ModelProviderInfo;
pub(crate) use codex_model_provider_info::built_in_model_providers;
pub(crate) use codex_plugin::PluginHookSource;
pub(crate) use codex_plugin::PluginId;
pub(crate) use codex_protocol::items::parse_hook_prompt_fragment;
pub(crate) use codex_protocol::models::ContentItem;
pub(crate) use codex_protocol::models::PermissionProfile;
pub(crate) use codex_protocol::models::ResponseItem;
pub(crate) use codex_protocol::permissions::NetworkSandboxPolicy;
pub(crate) use codex_protocol::protocol::AskForApproval;
pub(crate) use codex_protocol::protocol::EventMsg;
pub(crate) use codex_protocol::protocol::Op;
pub(crate) use codex_protocol::protocol::RolloutItem;
pub(crate) use codex_protocol::protocol::RolloutLine;
pub(crate) use codex_protocol::user_input::UserInput;
pub(crate) use codex_utils_absolute_path::AbsolutePathBuf;
pub(crate) use pretty_assertions::assert_eq;
pub(crate) use serde_json::Value;
pub(crate) use std::sync::Arc;
pub(crate) use std::time::Duration;
pub(crate) use tempfile::TempDir;
pub(crate) use tokio::sync::oneshot;
pub(crate) use tokio::time::sleep;
pub(crate) use tokio::time::timeout;

pub(crate) const FIRST_CONTINUATION_PROMPT: &str = "Retry with exactly the phrase meow meow meow.";
pub(crate) const SECOND_CONTINUATION_PROMPT: &str = "Now tighten it to just: meow.";
pub(crate) const BLOCKED_PROMPT_CONTEXT: &str = "Remember the blocked lighthouse note.";
pub(crate) const PERMISSION_REQUEST_HOOK_MATCHER: &str = "^Bash$";
pub(crate) const PERMISSION_REQUEST_ALLOW_REASON: &str = "should not be used for allow";

pub(crate) fn restrictive_workspace_write_profile() -> PermissionProfile {
    PermissionProfile::workspace_write_with(
        &[],
        NetworkSandboxPolicy::Restricted,
        /*exclude_tmpdir_env_var*/ true,
        /*exclude_slash_tmp*/ true,
    )
}

pub(crate) fn network_workspace_write_profile() -> PermissionProfile {
    PermissionProfile::workspace_write_with(
        &[],
        NetworkSandboxPolicy::Enabled,
        /*exclude_tmpdir_env_var*/ false,
        /*exclude_slash_tmp*/ false,
    )
}

pub(crate) fn non_openai_model_provider(server: &wiremock::MockServer) -> ModelProviderInfo {
    let mut provider =
        built_in_model_providers(/* openai_base_url */ /*openai_base_url*/ None)["openai"].clone();
    provider.name = "OpenAI (test)".into();
    provider.base_url = Some(format!("{}/v1", server.uri()));
    provider.supports_websockets = false;
    provider
}

pub(crate) fn trust_plugin_hooks(config: &mut Config, plugin_hook_sources: Vec<PluginHookSource>) {
    if let Err(err) = config.features.enable(Feature::CodexHooks) {
        panic!("test config should allow feature update: {err}");
    }
    let listed = codex_hooks::list_hooks(codex_hooks::HooksConfig {
        feature_enabled: true,
        config_layer_stack: Some(config.config_layer_stack.clone()),
        plugin_hook_sources,
        ..codex_hooks::HooksConfig::default()
    });
    assert!(
        !listed.hooks.is_empty(),
        "trusted plugin hook fixture should discover at least one hook"
    );
    trust_hooks(config, listed.hooks);
}

pub(crate) fn write_stop_hook(home: &Path, block_prompts: &[&str]) -> Result<()> {
    let script_path = home.join("stop_hook.py");
    let log_path = home.join("stop_hook_log.jsonl");
    let prompts_json =
        serde_json::to_string(block_prompts).context("serialize stop hook prompts for test")?;
    let script = format!(
        r#"import json
from pathlib import Path
import sys

log_path = Path(r"{log_path}")
block_prompts = {prompts_json}

payload = json.load(sys.stdin)
existing = []
if log_path.exists():
    existing = [line for line in log_path.read_text(encoding="utf-8").splitlines() if line.strip()]

with log_path.open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")

invocation_index = len(existing)
if invocation_index < len(block_prompts):
    print(json.dumps({{"decision": "block", "reason": block_prompts[invocation_index]}}))
else:
    print(json.dumps({{"systemMessage": f"stop hook pass {{invocation_index + 1}} complete"}}))
"#,
        log_path = log_path.display(),
        prompts_json = prompts_json,
    );
    let hooks = serde_json::json!({
        "hooks": {
            "Stop": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", script_path.display()),
                    "statusMessage": "running stop hook",
                }]
            }]
        }
    });

    fs::write(&script_path, script).context("write stop hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn write_parallel_stop_hooks(home: &Path, prompts: &[&str]) -> Result<()> {
    let hook_entries = prompts
        .iter()
        .enumerate()
        .map(|(index, prompt)| {
            let script_path = home.join(format!("stop_hook_{index}.py"));
            let script = format!(
                r#"import json
import sys

payload = json.load(sys.stdin)
if payload["stop_hook_active"]:
    print(json.dumps({{"systemMessage": "done"}}))
else:
    print(json.dumps({{"decision": "block", "reason": {prompt:?}}}))
"#
            );
            fs::write(&script_path, script).with_context(|| {
                format!(
                    "write stop hook script fixture at {}",
                    script_path.display()
                )
            })?;
            Ok(serde_json::json!({
                "type": "command",
                "command": format!("python3 {}", script_path.display()),
            }))
        })
        .collect::<Result<Vec<_>>>()?;

    let hooks = serde_json::json!({
        "hooks": {
            "Stop": [{
                "hooks": hook_entries,
            }]
        }
    });

    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn write_user_prompt_submit_hook(
    home: &Path,
    blocked_prompt: &str,
    additional_context: &str,
) -> Result<()> {
    let script_path = home.join("user_prompt_submit_hook.py");
    let log_path = home.join("user_prompt_submit_hook_log.jsonl");
    let log_path = log_path.display();
    let blocked_prompt_json =
        serde_json::to_string(blocked_prompt).context("serialize blocked prompt for test")?;
    let additional_context_json = serde_json::to_string(additional_context)
        .context("serialize user prompt submit additional context for test")?;
    let script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
with Path(r"{log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")

if payload.get("prompt") == {blocked_prompt_json}:
    print(json.dumps({{
        "decision": "block",
        "reason": "blocked by hook",
        "hookSpecificOutput": {{
            "hookEventName": "UserPromptSubmit",
            "additionalContext": {additional_context_json}
        }}
    }}))
"#,
    );
    let hooks = serde_json::json!({
        "hooks": {
            "UserPromptSubmit": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", script_path.display()),
                    "statusMessage": "running user prompt submit hook",
                }]
            }]
        }
    });

    fs::write(&script_path, script).context("write user prompt submit hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn write_session_start_and_user_prompt_submit_order_hooks(home: &Path) -> Result<()> {
    let session_start_script_path = home.join("session_start_order_hook.py");
    let user_prompt_submit_script_path = home.join("user_prompt_submit_order_hook.py");
    let log_path = home.join("hook_order_log.jsonl");

    let session_start_script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
with Path(r"{log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps({{
        "hook_event_name": payload.get("hook_event_name"),
        "source": payload.get("source"),
    }}) + "\n")
"#,
        log_path = log_path.display(),
    );
    let user_prompt_submit_script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
with Path(r"{log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps({{
        "hook_event_name": payload.get("hook_event_name"),
        "prompt": payload.get("prompt"),
    }}) + "\n")
"#,
        log_path = log_path.display(),
    );
    let hooks = serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", session_start_script_path.display()),
                    "statusMessage": "running session start order hook",
                }]
            }],
            "UserPromptSubmit": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", user_prompt_submit_script_path.display()),
                    "statusMessage": "running user prompt submit order hook",
                }]
            }]
        }
    });

    fs::write(&session_start_script_path, session_start_script)
        .context("write session start order hook script")?;
    fs::write(&user_prompt_submit_script_path, user_prompt_submit_script)
        .context("write user prompt submit order hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn write_pre_tool_use_hook(
    home: &Path,
    matcher: Option<&str>,
    mode: &str,
    reason: &str,
) -> Result<()> {
    let script_path = home.join("pre_tool_use_hook.py");
    let log_path = home.join("pre_tool_use_hook_log.jsonl");
    let mode_json = serde_json::to_string(mode).context("serialize pre tool use mode")?;
    let reason_json = serde_json::to_string(reason).context("serialize pre tool use reason")?;
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

if mode == "json_deny":
    print(json.dumps({{
        "hookSpecificOutput": {{
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason
        }}
    }}))
elif mode == "context":
    print(json.dumps({{
        "hookSpecificOutput": {{
            "hookEventName": "PreToolUse",
            "additionalContext": reason
        }}
    }}))
elif mode == "json_deny_with_context":
    print(json.dumps({{
        "hookSpecificOutput": {{
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
            "additionalContext": reason
        }}
    }}))
elif mode == "exit_2":
    sys.stderr.write(reason + "\n")
    raise SystemExit(2)
"#,
        log_path = log_path.display(),
        mode_json = mode_json,
        reason_json = reason_json,
    );

    let mut group = serde_json::json!({
        "hooks": [{
            "type": "command",
            "command": format!("python3 {}", script_path.display()),
            "statusMessage": "running pre tool use hook",
        }]
    });
    if let Some(matcher) = matcher {
        group["matcher"] = Value::String(matcher.to_string());
    }

    let hooks = serde_json::json!({
        "hooks": {
            "PreToolUse": [group]
        }
    });

    fs::write(&script_path, script).context("write pre tool use hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn write_updating_pre_tool_use_hook(
    home: &Path,
    matcher: &str,
    updated_input: &Value,
) -> Result<()> {
    let script_path = home.join("pre_tool_use_hook.py");
    let log_path = home.join("pre_tool_use_hook_log.jsonl");
    let updated_input_json =
        serde_json::to_string(updated_input).context("serialize updated pre tool input")?;
    let script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)

with Path(r"{log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")

print(json.dumps({{
    "hookSpecificOutput": {{
        "hookEventName": "PreToolUse",
        "permissionDecision": "allow",
        "updatedInput": {updated_input_json}
    }}
}}))
"#,
        log_path = log_path.display(),
        updated_input_json = updated_input_json,
    );
    let hooks = serde_json::json!({
        "hooks": {
            "PreToolUse": [{
                "matcher": matcher,
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", script_path.display()),
                    "statusMessage": "rewriting pre tool input",
                }]
            }]
        }
    });

    fs::write(&script_path, script).context("write updating pre tool use hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn write_pre_tool_use_hook_toml(
    home: &Path,
    script_name: &str,
    log_name: &str,
    matcher: Option<&str>,
    mode: &str,
    reason: &str,
) -> Result<()> {
    let script_path = home.join(script_name);
    let log_path = home.join(log_name);
    let mode_json = serde_json::to_string(mode).context("serialize pre tool use mode")?;
    let reason_json = serde_json::to_string(reason).context("serialize pre tool use reason")?;
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

if mode == "json_deny":
    print(json.dumps({{
        "hookSpecificOutput": {{
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason
        }}
    }}))
elif mode == "exit_2":
    sys.stderr.write(reason + "\n")
    raise SystemExit(2)
"#,
        log_path = log_path.display(),
        mode_json = mode_json,
        reason_json = reason_json,
    );
    let matcher_line = matcher
        .map(|matcher| format!("matcher = '{matcher}'\n"))
        .unwrap_or_default();
    let config_toml = format!(
        r#"[features]
hooks = true

[hooks]

[[hooks.PreToolUse]]
{matcher_line}

[[hooks.PreToolUse.hooks]]
type = "command"
command = 'python3 {script_path}'
statusMessage = "running pre tool use hook"
"#,
        matcher_line = matcher_line,
        script_path = script_path.display(),
    );

    fs::write(&script_path, script).context("write TOML pre tool use hook script")?;
    fs::write(home.join("config.toml"), config_toml).context("write config.toml hooks")?;
    Ok(())
}

pub(crate) fn write_permission_request_hook(
    home: &Path,
    matcher: Option<&str>,
    mode: &str,
    reason: &str,
) -> Result<()> {
    let script_path = home.join("permission_request_hook.py");
    let log_path = home.join("permission_request_hook_log.jsonl");
    let mode_json = serde_json::to_string(mode).context("serialize permission request mode")?;
    let reason_json =
        serde_json::to_string(reason).context("serialize permission request reason")?;
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

if mode == "allow":
    print(json.dumps({{
        "hookSpecificOutput": {{
            "hookEventName": "PermissionRequest",
            "decision": {{"behavior": "allow"}}
        }}
    }}))
elif mode == "deny":
    print(json.dumps({{
        "hookSpecificOutput": {{
            "hookEventName": "PermissionRequest",
            "decision": {{
                "behavior": "deny",
                "message": reason
            }}
        }}
    }}))
elif mode == "exit_2":
    sys.stderr.write(reason + "\n")
    raise SystemExit(2)
"#,
        log_path = log_path.display(),
        mode_json = mode_json,
        reason_json = reason_json,
    );

    let mut group = serde_json::json!({
        "hooks": [{
            "type": "command",
            "command": format!("python3 {}", script_path.display()),
            "statusMessage": "running permission request hook",
        }]
    });
    if let Some(matcher) = matcher {
        group["matcher"] = Value::String(matcher.to_string());
    }

    let hooks = serde_json::json!({
        "hooks": {
            "PermissionRequest": [group]
        }
    });

    fs::write(&script_path, script).context("write permission request hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn install_allow_permission_request_hook(home: &Path) -> Result<()> {
    write_permission_request_hook(
        home,
        Some(PERMISSION_REQUEST_HOOK_MATCHER),
        "allow",
        PERMISSION_REQUEST_ALLOW_REASON,
    )
}

pub(crate) fn write_post_tool_use_hook(
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
        mode_json = mode_json,
        reason_json = reason_json,
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

pub(crate) fn write_logging_pre_and_blocking_post_tool_use_hooks(home: &Path, feedback: &str) -> Result<()> {
    let pre_script_path = home.join("pre_tool_use_hook.py");
    let pre_log_path = home.join("pre_tool_use_hook_log.jsonl");
    let post_script_path = home.join("post_tool_use_hook.py");
    let post_log_path = home.join("post_tool_use_hook_log.jsonl");
    let feedback_json =
        serde_json::to_string(feedback).context("serialize post tool use feedback")?;
    let pre_script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
with Path(r"{pre_log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")
"#,
        pre_log_path = pre_log_path.display(),
    );
    let post_script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
with Path(r"{post_log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")
sys.stderr.write({feedback_json} + "\n")
raise SystemExit(2)
"#,
        post_log_path = post_log_path.display(),
    );
    let hooks = serde_json::json!({
        "hooks": {
            "PreToolUse": [{
                "matcher": "Bash",
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", pre_script_path.display()),
                    "statusMessage": "running pre tool use hook",
                }]
            }],
            "PostToolUse": [{
                "matcher": "Bash",
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", post_script_path.display()),
                    "statusMessage": "running post tool use hook",
                }]
            }]
        }
    });

    fs::write(&pre_script_path, pre_script).context("write pre tool use hook script")?;
    fs::write(&post_script_path, post_script).context("write post tool use hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn write_session_start_hook_recording_transcript(home: &Path) -> Result<()> {
    let script_path = home.join("session_start_hook.py");
    let log_path = home.join("session_start_hook_log.jsonl");
    let script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
transcript_path = payload.get("transcript_path")
record = {{
    "transcript_path": transcript_path,
    "exists": Path(transcript_path).exists() if transcript_path else False,
}}

with Path(r"{log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(record) + "\n")
"#,
        log_path = log_path.display(),
    );
    let hooks = serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", script_path.display()),
                    "statusMessage": "running session start hook",
                }]
            }]
        }
    });

    fs::write(&script_path, script).context("write session start hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn write_session_start_hook_with_context(home: &Path, additional_context: &str) -> Result<()> {
    let script_path = home.join("session_start_hook.py");
    let additional_context_json = serde_json::to_string(additional_context)
        .context("serialize session start additional context for test")?;
    let script = format!(
        r#"import json

print(json.dumps({{
    "hookSpecificOutput": {{
        "hookEventName": "SessionStart",
        "additionalContext": {additional_context_json}
    }}
}}))
"#,
    );
    let hooks = serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", script_path.display()),
                    "statusMessage": "running session start hook",
                }]
            }]
        }
    });

    fs::write(&script_path, script).context("write session start hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn write_compact_session_start_hook_with_context(
    home: &Path,
    additional_context: &str,
) -> Result<()> {
    let script_path = home.join("compact_session_start_hook.py");
    let log_path = home.join("session_start_hook_log.jsonl");
    let additional_context_json = serde_json::to_string(additional_context)
        .context("serialize compact session start additional context for test")?;
    let script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
with Path(r"{log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")

print(json.dumps({{
    "hookSpecificOutput": {{
        "hookEventName": "SessionStart",
        "additionalContext": {additional_context_json}
    }}
}}))
"#,
        log_path = log_path.display(),
    );
    let hooks = serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "matcher": "compact",
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", script_path.display()),
                    "statusMessage": "running compact session start hook",
                }]
            }]
        }
    });

    fs::write(&script_path, script).context("write compact session start hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn write_resume_and_compact_session_start_hook_with_context(
    home: &Path,
    resume_context: &str,
    compact_context: &str,
) -> Result<()> {
    let script_path = home.join("resume_and_compact_session_start_hook.py");
    let log_path = home.join("session_start_hook_log.jsonl");
    let resume_context_json = serde_json::to_string(resume_context)
        .context("serialize resume session start additional context for test")?;
    let compact_context_json = serde_json::to_string(compact_context)
        .context("serialize compact session start additional context for test")?;
    let script = format!(
        r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
with Path(r"{log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")

contexts = {{
    "resume": {resume_context_json},
    "compact": {compact_context_json},
}}
print(json.dumps({{
    "hookSpecificOutput": {{
        "hookEventName": "SessionStart",
        "additionalContext": contexts[payload["source"]]
    }}
}}))
"#,
        log_path = log_path.display(),
    );
    let hooks = serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "matcher": "resume",
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", script_path.display()),
                    "statusMessage": "running resume session start hook",
                }]
            }, {
                "matcher": "compact",
                "hooks": [{
                    "type": "command",
                    "command": format!("python3 {}", script_path.display()),
                    "statusMessage": "running compact session start hook",
                }]
            }]
        }
    });

    fs::write(&script_path, script)
        .context("write resume and compact session start hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

pub(crate) fn rollout_hook_prompt_texts(text: &str) -> Result<Vec<String>> {
    let mut texts = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rollout: RolloutLine = serde_json::from_str(trimmed).context("parse rollout line")?;
        if let RolloutItem::ResponseItem(ResponseItem::Message { role, content, .. }) = rollout.item
            && role == "user"
        {
            for item in content {
                if let ContentItem::InputText { text } = item
                    && let Some(fragment) = parse_hook_prompt_fragment(&text)
                {
                    texts.push(fragment.text);
                }
            }
        }
    }
    Ok(texts)
}

pub(crate) fn request_hook_prompt_texts(
    request: &codex_core_test_runtime::responses::ResponsesRequest,
) -> Vec<String> {
    request
        .message_input_texts("user")
        .into_iter()
        .filter_map(|text| parse_hook_prompt_fragment(&text).map(|fragment| fragment.text))
        .collect()
}

pub(crate) fn spilled_hook_output_path(text: &str) -> Option<&str> {
    text.lines()
        .find_map(|line| line.strip_prefix("Full hook output saved to: "))
}

pub(crate) fn read_stop_hook_inputs(home: &Path) -> Result<Vec<serde_json::Value>> {
    fs::read_to_string(home.join("stop_hook_log.jsonl"))
        .context("read stop hook log")?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("parse stop hook log line"))
        .collect()
}

pub(crate) fn read_pre_tool_use_hook_inputs(home: &Path) -> Result<Vec<serde_json::Value>> {
    read_hook_inputs_from_log(home.join("pre_tool_use_hook_log.jsonl").as_path())
}

pub(crate) fn read_permission_request_hook_inputs(home: &Path) -> Result<Vec<serde_json::Value>> {
    fs::read_to_string(home.join("permission_request_hook_log.jsonl"))
        .context("read permission request hook log")?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("parse permission request hook log line"))
        .collect()
}

pub(crate) fn assert_permission_request_hook_input(
    hook_input: &Value,
    tool_name: &str,
    command: &str,
    description: Option<&str>,
) {
    assert_eq!(hook_input["hook_event_name"], "PermissionRequest");
    assert_eq!(hook_input["tool_name"], tool_name);
    assert_eq!(hook_input["tool_input"]["command"], command);
    assert_eq!(
        hook_input["tool_input"]["description"],
        description.map_or(Value::Null, Value::from)
    );
    assert!(hook_input.get("approval_attempt").is_none());
    assert!(hook_input.get("sandbox_permissions").is_none());
    assert!(hook_input.get("additional_permissions").is_none());
    assert!(hook_input.get("justification").is_none());
    assert!(hook_input.get("host").is_none());
    assert!(hook_input.get("protocol").is_none());
}

pub(crate) fn assert_single_permission_request_hook_input(
    home: &Path,
    command: &str,
    description: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    assert_single_permission_request_hook_input_for_tool(home, "Bash", command, description)
}

pub(crate) fn assert_single_permission_request_hook_input_for_tool(
    home: &Path,
    tool_name: &str,
    command: &str,
    description: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    let hook_inputs = read_permission_request_hook_inputs(home)?;
    assert_eq!(hook_inputs.len(), 1);
    assert_permission_request_hook_input(&hook_inputs[0], tool_name, command, description);
    Ok(hook_inputs)
}

pub(crate) fn read_post_tool_use_hook_inputs(home: &Path) -> Result<Vec<serde_json::Value>> {
    read_hook_inputs_from_log(home.join("post_tool_use_hook_log.jsonl").as_path())
}

pub(crate) fn read_hook_inputs_from_log(log_path: &Path) -> Result<Vec<serde_json::Value>> {
    fs::read_to_string(log_path)
        .with_context(|| format!("read hook log {}", log_path.display()))?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("parse hook log line"))
        .collect()
}

pub(crate) fn read_session_start_hook_inputs(home: &Path) -> Result<Vec<serde_json::Value>> {
    fs::read_to_string(home.join("session_start_hook_log.jsonl"))
        .context("read session start hook log")?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("parse session start hook log line"))
        .collect()
}

pub(crate) fn read_user_prompt_submit_hook_inputs(home: &Path) -> Result<Vec<serde_json::Value>> {
    fs::read_to_string(home.join("user_prompt_submit_hook_log.jsonl"))
        .context("read user prompt submit hook log")?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("parse user prompt submit hook log line"))
        .collect()
}

pub(crate) fn read_hook_order_inputs(home: &Path) -> Result<Vec<serde_json::Value>> {
    read_hook_inputs_from_log(home.join("hook_order_log.jsonl").as_path())
}

pub(crate) fn ev_message_item_done(id: &str, text: &str) -> Value {
    serde_json::json!({
        "type": "response.output_item.done",
        "item": {
            "type": "message",
            "role": "assistant",
            "id": id,
            "content": [{"type": "output_text", "text": text}]
        }
    })
}

pub(crate) fn sse_event(event: Value) -> String {
    sse(vec![event])
}

pub(crate) fn request_message_input_texts(body: &[u8], role: &str) -> Vec<String> {
    let body: Value = match serde_json::from_slice(body) {
        Ok(body) => body,
        Err(error) => panic!("parse request body: {error}"),
    };
    body.get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        .filter(|item| item.get("role").and_then(Value::as_str) == Some(role))
        .filter_map(|item| item.get("content").and_then(Value::as_array))
        .flatten()
        .filter(|span| span.get("type").and_then(Value::as_str) == Some("input_text"))
        .filter_map(|span| span.get("text").and_then(Value::as_str).map(str::to_owned))
        .collect()
}
