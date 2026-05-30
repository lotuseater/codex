use super::common::*;

#[tokio::test]
async fn pre_tool_use_hook_spills_large_additional_context() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-shell-command-large-context";
    let command = "printf pre-tool-output".to_string();
    let args = serde_json::json!({ "command": command });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                codex_core_test_runtime::responses::ev_function_call(
                    call_id,
                    "shell_command",
                    &serde_json::to_string(&args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "pre hook context observed"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;
    let additional_context = "remember the pre tool reef ".repeat(800);

    let mut builder = test_codex()
        .with_pre_build_hook({
            let additional_context = additional_context.clone();
            move |home| {
                if let Err(error) =
                    write_pre_tool_use_hook(home, Some("^Bash$"), "context", &additional_context)
                {
                    panic!("failed to write pre tool use hook test fixture: {error}");
                }
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    test.submit_turn("run the shell command with large pre hook context")
        .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    let developer_messages = requests[1].message_input_texts("developer");
    let developer_message = developer_messages
        .iter()
        .find(|message| spilled_hook_output_path(message).is_some())
        .context("spilled developer hook message")?;
    assert!(developer_message.contains("tokens truncated"));
    let path = spilled_hook_output_path(developer_message).context("spill path")?;
    assert_eq!(fs::read_to_string(path)?, additional_context);

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_blocks_shell_command_before_execution() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-shell-command";
    let marker = std::env::temp_dir().join("pretooluse-shell-command-marker");
    let command = format!("printf blocked > {}", marker.display());
    let args = serde_json::json!({ "command": command });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                codex_core_test_runtime::responses::ev_function_call(
                    call_id,
                    "shell_command",
                    &serde_json::to_string(&args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "hook blocked it"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) =
                write_pre_tool_use_hook(home, Some("^Bash$"), "json_deny", "blocked by pre hook")
            {
                panic!("failed to write pre tool use hook test fixture: {error}");
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    if marker.exists() {
        fs::remove_file(&marker).context("remove leftover pre tool use marker")?;
    }

    test.submit_turn_with_permission_profile(
        "run the blocked shell command",
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    let output_item = requests[1].function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("shell command output string");
    assert!(
        output.contains("Command blocked by PreToolUse hook: blocked by pre hook"),
        "blocked tool output should surface the hook reason",
    );
    assert!(
        output.contains(&format!("Command: {command}")),
        "blocked tool output should surface the blocked command",
    );
    assert!(
        !marker.exists(),
        "blocked command should not create marker file"
    );

    let hook_inputs = read_pre_tool_use_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["hook_event_name"], "PreToolUse");
    assert_eq!(hook_inputs[0]["tool_name"], "Bash");
    assert_eq!(hook_inputs[0]["tool_use_id"], call_id);
    assert_eq!(hook_inputs[0]["tool_input"]["command"], command);
    let transcript_path = hook_inputs[0]["transcript_path"]
        .as_str()
        .expect("pre tool use hook transcript_path");
    assert!(
        !transcript_path.is_empty(),
        "pre tool use hook should receive a non-empty transcript_path",
    );
    assert!(
        Path::new(transcript_path).exists(),
        "pre tool use hook transcript_path should be materialized on disk",
    );
    assert!(
        hook_inputs[0]["turn_id"]
            .as_str()
            .is_some_and(|turn_id| !turn_id.is_empty())
    );

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_records_additional_context_for_shell_command() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-shell-command-context";
    let command = "printf pre-tool-output".to_string();
    let args = serde_json::json!({ "command": command });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                codex_core_test_runtime::responses::ev_function_call(
                    call_id,
                    "shell_command",
                    &serde_json::to_string(&args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "pre hook context observed"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let pre_context = "Remember the bash pre-tool note.";
    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) =
                write_pre_tool_use_hook(home, Some("^Bash$"), "context", pre_context)
            {
                panic!("failed to write pre tool use hook test fixture: {error}");
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    test.submit_turn("run the shell command with pre hook")
        .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    assert!(
        requests[1]
            .message_input_texts("developer")
            .contains(&pre_context.to_string()),
        "follow-up request should include pre tool use additional context",
    );
    let output_item = requests[1].function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("shell command output string");
    assert!(
        output.contains("pre-tool-output"),
        "shell command output should still reach the model",
    );

    Ok(())
}

#[tokio::test]
async fn blocked_pre_tool_use_records_additional_context_for_shell_command() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-shell-command-blocked-context";
    let marker = std::env::temp_dir().join("pretooluse-shell-command-blocked-context-marker");
    let command = format!("printf blocked > {}", marker.display());
    let args = serde_json::json!({ "command": command });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                codex_core_test_runtime::responses::ev_function_call(
                    call_id,
                    "shell_command",
                    &serde_json::to_string(&args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "blocked pre hook context observed"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let pre_context = "blocked by pre hook with context";
    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) =
                write_pre_tool_use_hook(home, Some("^Bash$"), "json_deny_with_context", pre_context)
            {
                panic!("failed to write pre tool use hook test fixture: {error}");
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    if marker.exists() {
        fs::remove_file(&marker).context("remove leftover pre tool use marker")?;
    }

    test.submit_turn_with_permission_profile(
        "run the blocked shell command with pre hook context",
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    assert!(
        requests[1]
            .message_input_texts("developer")
            .contains(&pre_context.to_string()),
        "follow-up request should include blocked pre tool use additional context",
    );
    let output_item = requests[1].function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("shell command output string");
    assert!(
        output.contains("Command blocked by PreToolUse hook: blocked by pre hook with context"),
        "blocked tool output should still surface the hook reason",
    );
    assert!(
        !marker.exists(),
        "blocked command should not create marker file"
    );
    Ok(())
}

#[derive(Clone, Copy)]
enum BashRewriteSurface {
    ExecCommand,
    ShellCommand,
}

impl BashRewriteSurface {
    fn slug(self) -> &'static str {
        match self {
            BashRewriteSurface::ExecCommand => "exec-command",
            BashRewriteSurface::ShellCommand => "shell-command",
        }
    }

    fn tool_call(self, call_id: &str, command_text: &str) -> Result<Value> {
        match self {
            BashRewriteSurface::ExecCommand => Ok(ev_function_call(
                call_id,
                "exec_command",
                &serde_json::to_string(&serde_json::json!({ "cmd": command_text }))?,
            )),
            BashRewriteSurface::ShellCommand => Ok(ev_function_call(
                call_id,
                "shell_command",
                &serde_json::to_string(&serde_json::json!({ "command": command_text }))?,
            )),
        }
    }

    fn original_command(self, marker: &Path) -> String {
        match self {
            BashRewriteSurface::ExecCommand | BashRewriteSurface::ShellCommand => {
                format!("printf original > {}", marker.display())
            }
        }
    }

    fn rewritten_command(self, marker: &Path) -> String {
        match self {
            BashRewriteSurface::ExecCommand | BashRewriteSurface::ShellCommand => {
                format!("printf rewritten > {}", marker.display())
            }
        }
    }

    fn configure(self, config: &mut Config) {
        trust_discovered_hooks(config);
        if matches!(self, BashRewriteSurface::ExecCommand) {
            config.use_experimental_unified_exec_tool = true;
            if let Err(error) = config.features.enable(Feature::UnifiedExec) {
                panic!("test config should allow feature update: {error}");
            }
        }
    }
}

async fn assert_pre_tool_use_rewrites_bash_surface(surface: BashRewriteSurface) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let slug = surface.slug();
    let call_id = format!("pretooluse-{slug}-rewrite");
    let original_marker = std::env::temp_dir().join(format!("pretooluse-{slug}-original-marker"));
    let rewritten_marker = std::env::temp_dir().join(format!("pretooluse-{slug}-rewritten-marker"));
    let original_command = surface.original_command(&original_marker);
    let rewritten_command = surface.rewritten_command(&rewritten_marker);
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                surface.tool_call(&call_id, &original_command)?,
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "hook rewrote it"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let updated_input = serde_json::json!({ "command": rewritten_command });
    let mut builder = test_codex()
        .with_pre_build_hook(move |home| {
            if let Err(error) = write_updating_pre_tool_use_hook(home, "^Bash$", &updated_input) {
                panic!("failed to write updating pre tool use hook fixture: {error}");
            }
        })
        .with_config(move |config| surface.configure(config));
    let test = builder.build(&server).await?;

    if original_marker.exists() {
        fs::remove_file(&original_marker).context("remove stale original pre tool marker")?;
    }
    if rewritten_marker.exists() {
        fs::remove_file(&rewritten_marker).context("remove stale rewritten pre tool marker")?;
    }

    test.submit_turn_with_permission_profile(
        &format!("run the rewritten {slug} command"),
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    requests[1].function_call_output(&call_id);
    assert!(
        !original_marker.exists(),
        "original {slug} command should not execute after rewrite"
    );
    assert_eq!(
        fs::read_to_string(&rewritten_marker).context("read rewritten pre tool marker")?,
        "rewritten"
    );

    let hook_inputs = read_pre_tool_use_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["tool_input"]["command"], original_command);

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_rewrites_shell_command_before_execution() -> Result<()> {
    assert_pre_tool_use_rewrites_bash_surface(BashRewriteSurface::ShellCommand).await
}

#[tokio::test]
async fn pre_tool_use_rewrites_exec_command_before_execution() -> Result<()> {
    assert_pre_tool_use_rewrites_bash_surface(BashRewriteSurface::ExecCommand).await
}

#[tokio::test]
async fn pre_tool_use_rewrites_code_mode_nested_exec_command_before_execution() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-code-mode-rewrite";
    let original_marker = std::env::temp_dir().join("pretooluse-code-mode-original-marker");
    let rewritten_marker = std::env::temp_dir().join("pretooluse-code-mode-rewritten-marker");
    let original_command = format!("printf original > {}", original_marker.display());
    let rewritten_command = format!("printf rewritten > {}", rewritten_marker.display());
    let original_command_json =
        serde_json::to_string(&original_command).context("serialize original command")?;
    let code = format!(
        r#"
const output = await tools.exec_command({{ cmd: {original_command_json} }});
text(output.output);
"#
    );
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_custom_tool_call(call_id, "exec", &code),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "hook rewrote the nested command"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let updated_input = serde_json::json!({ "command": rewritten_command });
    let mut builder = test_codex()
        .with_model("test-gpt-5.1-codex")
        .with_pre_build_hook(move |home| {
            if let Err(error) = write_updating_pre_tool_use_hook(home, "^Bash$", &updated_input) {
                panic!("failed to write updating pre tool use hook fixture: {error}");
            }
        })
        .with_config(|config| {
            let _ = config.features.enable(Feature::CodeMode);
            trust_discovered_hooks(config);
        });
    let test = builder.build(&server).await?;

    if original_marker.exists() {
        fs::remove_file(&original_marker).context("remove stale original pre tool marker")?;
    }
    if rewritten_marker.exists() {
        fs::remove_file(&rewritten_marker).context("remove stale rewritten pre tool marker")?;
    }

    test.submit_turn_with_permission_profile(
        "run the rewritten shell command from code mode",
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    requests[1].custom_tool_call_output(call_id);
    assert!(
        !original_marker.exists(),
        "original nested shell command should not execute after rewrite"
    );
    assert_eq!(
        fs::read_to_string(&rewritten_marker)
            .context("read rewritten code mode pre tool marker")?,
        "rewritten"
    );

    let hook_inputs = read_pre_tool_use_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["tool_input"]["command"], original_command);

    Ok(())
}

#[tokio::test]
async fn plugin_pre_tool_use_blocks_shell_command_before_execution() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "plugin-pretooluse-shell-command";
    let marker = std::env::temp_dir().join("plugin-pretooluse-shell-command-marker");
    let command = format!("printf blocked > {}", marker.display());
    let args = serde_json::json!({ "command": command });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                codex_core_test_runtime::responses::ev_function_call(
                    call_id,
                    "shell_command",
                    &serde_json::to_string(&args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "plugin hook blocked it"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let home = Arc::new(TempDir::new()?);
    let plugin_root = home.path().join("plugins/cache/test/sample/local");
    let hooks_dir = plugin_root.join("hooks");
    fs::create_dir_all(plugin_root.join(".codex-plugin"))
        .context("create plugin manifest directory")?;
    fs::create_dir_all(&hooks_dir).context("create plugin hooks directory")?;
    fs::write(
        plugin_root.join(".codex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    )
    .context("write plugin manifest")?;
    fs::write(
        home.path().join("config.toml"),
        r#"[plugins."sample@test"]
enabled = true
"#,
    )
    .context("write plugin config")?;

    let script_path = hooks_dir.join("pre_tool_use_hook.py");
    let log_path = hooks_dir.join("pre_tool_use_hook_log.jsonl");
    fs::write(
        &script_path,
        format!(
            r#"import json
from pathlib import Path
import sys

payload = json.load(sys.stdin)
with Path(r"{log_path}").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")

print(json.dumps({{
    "hookSpecificOutput": {{
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": "blocked by plugin hook"
    }}
}}))
"#,
            log_path = log_path.display(),
        ),
    )
    .context("write plugin pre tool use hook script")?;
    let plugin_hooks_json = r#"{
  "hooks": {
    "PreToolUse": [{
      "matcher": "^Bash$",
      "hooks": [{
        "type": "command",
        "command": "python3 ${PLUGIN_ROOT}/hooks/pre_tool_use_hook.py"
      }]
    }]
  }
}"#;
    let plugin_hooks_path = hooks_dir.join("hooks.json");
    fs::write(&plugin_hooks_path, plugin_hooks_json).context("write plugin hooks config")?;
    let plugin_root_abs =
        AbsolutePathBuf::try_from(plugin_root.clone()).context("absolute plugin root")?;
    let plugin_hooks_path_abs =
        AbsolutePathBuf::try_from(plugin_hooks_path).context("absolute plugin hooks path")?;
    let plugin_data_root =
        AbsolutePathBuf::try_from(plugin_root.join("data")).context("absolute plugin data root")?;
    let plugin_hook_sources = vec![PluginHookSource {
        plugin_id: PluginId::parse("sample@test").context("plugin id")?,
        plugin_root: plugin_root_abs,
        plugin_data_root,
        source_path: plugin_hooks_path_abs,
        source_relative_path: "hooks/hooks.json".to_string(),
        hooks: serde_json::from_str::<codex_config::HooksFile>(plugin_hooks_json)
            .context("parse plugin hooks")?
            .hooks,
    }];

    let mut builder = test_codex()
        .with_home(Arc::clone(&home))
        .with_config(move |config| {
            config
                .features
                .enable(Feature::Plugins)
                .expect("test config should allow feature update");
            trust_plugin_hooks(config, plugin_hook_sources);
        });
    let test = builder.build(&server).await?;

    if marker.exists() {
        fs::remove_file(&marker).context("remove leftover plugin pre tool use marker")?;
    }

    test.submit_turn_with_policy(
        "run the shell command blocked by a plugin hook",
        codex_protocol::protocol::SandboxPolicy::DangerFullAccess,
    )
    .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    let output_item = requests[1].function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("shell command output string");
    assert!(
        output.contains("Command blocked by PreToolUse hook: blocked by plugin hook"),
        "blocked tool output should surface the plugin hook reason",
    );
    assert!(
        !marker.exists(),
        "plugin hook should block shell command execution"
    );

    let hook_inputs = read_hook_inputs_from_log(&log_path)?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["hook_event_name"], "PreToolUse");
    assert_eq!(hook_inputs[0]["tool_name"], "Bash");
    assert_eq!(hook_inputs[0]["tool_use_id"], call_id);
    assert_eq!(hook_inputs[0]["tool_input"]["command"], command);

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_blocks_shell_when_defined_in_config_toml() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-config-toml";
    let marker = std::env::temp_dir().join("pretooluse-config-toml-marker");
    let command = format!("printf blocked > {}", marker.display());
    let args = serde_json::json!({ "command": command });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                codex_core_test_runtime::responses::ev_function_call(
                    call_id,
                    "shell_command",
                    &serde_json::to_string(&args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "config.toml hook blocked it"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) = write_pre_tool_use_hook_toml(
                home,
                "pre_tool_use_config_hook.py",
                "pre_tool_use_config_hook_log.jsonl",
                Some("^Bash$"),
                "json_deny",
                "blocked by config toml hook",
            ) {
                panic!("failed to write config.toml hook test fixture: {error}");
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    if marker.exists() {
        fs::remove_file(&marker).context("remove leftover config.toml marker")?;
    }

    test.submit_turn_with_permission_profile(
        "run the blocked shell command from config toml",
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    let output_item = requests[1].function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("shell command output string");
    assert!(
        output.contains("Command blocked by PreToolUse hook: blocked by config toml hook"),
        "blocked tool output should surface the config.toml hook reason",
    );
    assert!(
        !marker.exists(),
        "config.toml hook should block command execution"
    );

    let hook_inputs = read_hook_inputs_from_log(
        test.codex_home_path()
            .join("pre_tool_use_config_hook_log.jsonl")
            .as_path(),
    )?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["hook_event_name"], "PreToolUse");
    assert_eq!(hook_inputs[0]["tool_use_id"], call_id);
    assert_eq!(hook_inputs[0]["tool_input"]["command"], command);

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_merges_hooks_json_and_config_toml() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-merged-sources";
    let command = "printf merged-hooks".to_string();
    let args = serde_json::json!({ "command": command });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                codex_core_test_runtime::responses::ev_function_call(
                    call_id,
                    "shell_command",
                    &serde_json::to_string(&args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "merged hook context observed"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) = write_pre_tool_use_hook(home, Some("^Bash$"), "allow", "unused") {
                panic!("failed to write hooks.json hook fixture: {error}");
            }
            if let Err(error) = write_pre_tool_use_hook_toml(
                home,
                "pre_tool_use_toml_hook.py",
                "pre_tool_use_toml_hook_log.jsonl",
                Some("^Bash$"),
                "allow",
                "unused",
            ) {
                panic!("failed to write config.toml hook fixture: {error}");
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    test.submit_turn("run the shell command with merged hook sources")
        .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    let output_item = requests[1].function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("shell command output string");
    assert!(
        output.contains("merged-hooks"),
        "shell command output should still reach the model",
    );

    let json_hook_inputs = read_pre_tool_use_hook_inputs(test.codex_home_path())?
        .into_iter()
        .map(|hook_input| {
            serde_json::json!({
                "hook_event_name": hook_input["hook_event_name"],
                "tool_name": hook_input["tool_name"],
                "tool_use_id": hook_input["tool_use_id"],
                "tool_input": hook_input["tool_input"],
            })
        })
        .collect::<Vec<_>>();
    let toml_hook_inputs = read_hook_inputs_from_log(
        test.codex_home_path()
            .join("pre_tool_use_toml_hook_log.jsonl")
            .as_path(),
    )?
    .into_iter()
    .map(|hook_input| {
        serde_json::json!({
            "hook_event_name": hook_input["hook_event_name"],
            "tool_name": hook_input["tool_name"],
            "tool_use_id": hook_input["tool_use_id"],
            "tool_input": hook_input["tool_input"],
        })
    })
    .collect::<Vec<_>>();
    let expected_hook_inputs = vec![serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_use_id": call_id,
        "tool_input": {
            "command": command,
        },
    })];
    assert_eq!(expected_hook_inputs, json_hook_inputs);
    assert_eq!(expected_hook_inputs, toml_hook_inputs);

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_blocks_exec_command_before_execution() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-exec-command";
    let marker = std::env::temp_dir().join("pretooluse-exec-command-marker");
    let command = format!("printf blocked > {}", marker.display());
    let args = serde_json::json!({ "cmd": command });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                codex_core_test_runtime::responses::ev_function_call(
                    call_id,
                    "exec_command",
                    &serde_json::to_string(&args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "exec command blocked"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) =
                write_pre_tool_use_hook(home, Some("^Bash$"), "exit_2", "blocked exec command")
            {
                panic!("failed to write pre tool use hook test fixture: {error}");
            }
        })
        .with_config(|config| {
            config.use_experimental_unified_exec_tool = true;
            trust_discovered_hooks(config);
            config
                .features
                .enable(Feature::UnifiedExec)
                .expect("test config should allow feature update");
        });
    let test = builder.build(&server).await?;

    if marker.exists() {
        fs::remove_file(&marker).context("remove leftover exec marker")?;
    }

    test.submit_turn("run the blocked exec command").await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    let output_item = requests[1].function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("exec command output string");
    assert!(
        output.contains("Command blocked by PreToolUse hook: blocked exec command"),
        "blocked exec command output should surface the hook reason",
    );
    assert!(
        output.contains(&format!("Command: {command}")),
        "blocked exec command output should surface the blocked command",
    );
    assert!(!marker.exists(), "blocked exec command should not execute");

    let hook_inputs = read_pre_tool_use_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["tool_use_id"], call_id);
    assert_eq!(hook_inputs[0]["tool_input"]["command"], command);
    assert!(
        hook_inputs[0]["turn_id"]
            .as_str()
            .is_some_and(|turn_id| !turn_id.is_empty())
    );

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_blocks_apply_patch_before_execution() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-apply-patch";
    let file_name = "pre_tool_use_apply_patch.txt";
    let patch = format!(
        r#"*** Begin Patch
*** Add File: {file_name}
+blocked
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
                ev_assistant_message("msg-1", "apply_patch blocked"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) = write_pre_tool_use_hook(
                home,
                Some("^apply_patch$"),
                "json_deny",
                "blocked apply_patch",
            ) {
                panic!("failed to write pre tool use hook test fixture: {error}");
            }
        })
        .with_config(|config| {
            trust_discovered_hooks(config);
        });
    let test = builder.build(&server).await?;

    test.submit_turn("apply the blocked patch").await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    let output_item = requests[1].custom_tool_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("apply_patch output string");
    assert!(
        output.contains("Command blocked by PreToolUse hook: blocked apply_patch"),
        "blocked apply_patch output should surface the hook reason",
    );
    assert!(
        !test.workspace_path(file_name).exists(),
        "blocked apply_patch should not create the file"
    );

    let hook_inputs = read_pre_tool_use_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["tool_name"], "apply_patch");
    assert_eq!(hook_inputs[0]["tool_use_id"], call_id);
    assert_eq!(hook_inputs[0]["tool_input"]["command"], patch);

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_rewrites_apply_patch_before_execution() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-apply-patch-rewrite";
    let original_file = "pre_tool_use_apply_patch_original.txt";
    let rewritten_file = "pre_tool_use_apply_patch_rewritten.txt";
    let original_patch = format!(
        r#"*** Begin Patch
*** Add File: {original_file}
+original
*** End Patch"#
    );
    let rewritten_patch = format!(
        r#"*** Begin Patch
*** Add File: {rewritten_file}
+rewritten
*** End Patch"#
    );
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_apply_patch_custom_tool_call(call_id, &original_patch),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "apply_patch rewritten"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let updated_input = serde_json::json!({ "command": rewritten_patch });
    let mut builder = test_codex()
        .with_pre_build_hook(move |home| {
            if let Err(error) =
                write_updating_pre_tool_use_hook(home, "^apply_patch$", &updated_input)
            {
                panic!("failed to write updating pre tool use hook fixture: {error}");
            }
        })
        .with_config(|config| {
            trust_discovered_hooks(config);
        });
    let test = builder.build(&server).await?;

    test.submit_turn("apply the rewritten patch").await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    requests[1].custom_tool_call_output(call_id);
    assert!(
        !test.workspace_path(original_file).exists(),
        "original patch should not create its target file"
    );
    assert_eq!(
        fs::read_to_string(test.workspace_path(rewritten_file))
            .context("read rewritten apply_patch file")?,
        "rewritten\n"
    );

    let hook_inputs = read_pre_tool_use_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["tool_input"]["command"], original_patch);

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_blocks_apply_patch_with_write_alias() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-apply-patch-write";
    let file_name = "pre_tool_use_apply_patch_write.txt";
    let patch = format!(
        r#"*** Begin Patch
*** Add File: {file_name}
+blocked
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
                ev_assistant_message("msg-1", "apply_patch blocked by Write alias"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) =
                write_pre_tool_use_hook(home, Some("^Write$"), "json_deny", "blocked write alias")
            {
                panic!("failed to write pre tool use hook test fixture: {error}");
            }
        })
        .with_config(|config| {
            trust_discovered_hooks(config);
        });
    let test = builder.build(&server).await?;

    test.submit_turn("apply the patch blocked by Write alias")
        .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    let output_item = requests[1].custom_tool_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("apply_patch output string");
    assert!(
        output.contains("Command blocked by PreToolUse hook: blocked write alias"),
        "blocked apply_patch output should surface the hook reason",
    );
    assert!(
        !test.workspace_path(file_name).exists(),
        "blocked apply_patch should not create the file"
    );

    let hook_inputs = read_pre_tool_use_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["tool_name"], "apply_patch");
    assert_eq!(hook_inputs[0]["tool_use_id"], call_id);
    assert_eq!(hook_inputs[0]["tool_input"]["command"], patch);

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_blocks_local_function_tool_before_execution() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-local-function-tool";
    let args = serde_json::json!({});
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(call_id, "test_sync_tool", &serde_json::to_string(&args)?),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "local function hook blocked it"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let reason = "blocked local function pre hook";
    let mut builder = test_codex()
        .with_model("test-gpt-5.1-codex")
        .with_pre_build_hook(|home| {
            if let Err(error) =
                write_pre_tool_use_hook(home, Some("^test_sync_tool$"), "json_deny", reason)
            {
                panic!("failed to write pre tool use hook test fixture: {error}");
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    test.submit_turn("call the local function tool with the pre hook")
        .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    let output_item = requests[1].function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("blocked local function tool output string");
    assert!(
        output.contains(&format!(
            "Tool call blocked by PreToolUse hook: {reason}. Tool: test_sync_tool"
        )),
        "blocked local function output should surface the hook reason and tool name",
    );

    let hook_inputs = read_pre_tool_use_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["hook_event_name"], "PreToolUse");
    assert_eq!(hook_inputs[0]["tool_name"], "test_sync_tool");
    assert_eq!(hook_inputs[0]["tool_use_id"], call_id);
    assert_eq!(hook_inputs[0]["tool_input"], args);

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_rewrites_local_function_tool_before_execution() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-local-function-tool-rewrite";
    let original_args = serde_json::json!({
        "barrier": {
            "id": "pretooluse-local-function-invalid-barrier",
            "participants": 0,
        }
    });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(
                    call_id,
                    "test_sync_tool",
                    &serde_json::to_string(&original_args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "local function hook rewrote it"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let updated_input = serde_json::json!({});
    let mut builder = test_codex()
        .with_model("test-gpt-5.1-codex")
        .with_pre_build_hook(move |home| {
            if let Err(error) =
                write_updating_pre_tool_use_hook(home, "^test_sync_tool$", &updated_input)
            {
                panic!("failed to write updating pre tool use hook test fixture: {error}");
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    test.submit_turn("call the local function tool with the pre hook rewrite")
        .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    let output_item = requests[1].function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("rewritten local function tool output string");
    assert_eq!(output, "ok");

    let hook_inputs = read_pre_tool_use_hook_inputs(test.codex_home_path())?;
    assert_eq!(hook_inputs.len(), 1);
    assert_eq!(hook_inputs[0]["tool_input"], original_args);

    Ok(())
}
