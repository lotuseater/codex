#![allow(clippy::unwrap_used, clippy::expect_used)]

pub(crate) use anyhow::Context;
pub(crate) use anyhow::Result;
pub(crate) use codex_config::types::ApprovalsReviewer;
pub(crate) use codex_core::CodexThread;
pub(crate) use codex_core::config::Constrained;
pub(crate) use codex_core::sandboxing::SandboxPermissions;
pub(crate) use codex_core_test_runtime::managed_network_requirements_loader;
pub(crate) use codex_core_test_runtime::responses::ev_apply_patch_custom_tool_call;
pub(crate) use codex_core_test_runtime::responses::ev_assistant_message;
pub(crate) use codex_core_test_runtime::responses::ev_completed;
pub(crate) use codex_core_test_runtime::responses::ev_function_call;
pub(crate) use codex_core_test_runtime::responses::ev_response_created;
pub(crate) use codex_core_test_runtime::responses::mount_sse_once;
pub(crate) use codex_core_test_runtime::responses::mount_sse_once_match;
pub(crate) use codex_core_test_runtime::responses::sse;
pub(crate) use codex_core_test_runtime::responses::start_mock_server;
pub(crate) use codex_core_test_runtime::skip_if_no_network;
pub(crate) use codex_core_test_runtime::test_codex::TestCodex;
pub(crate) use codex_core_test_runtime::test_codex::test_codex;
pub(crate) use codex_core_test_runtime::test_codex::turn_permission_fields;
pub(crate) use codex_core_test_runtime::wait_for_event;
pub(crate) use codex_core_test_runtime::wait_for_event_with_timeout;
pub(crate) use codex_core_test_runtime::zsh_fork::build_zsh_fork_test;
pub(crate) use codex_core_test_runtime::zsh_fork::restrictive_workspace_write_profile;
pub(crate) use codex_core_test_runtime::zsh_fork::zsh_fork_runtime;
pub(crate) use codex_features::Feature;
pub(crate) use codex_protocol::approvals::NetworkApprovalProtocol;
pub(crate) use codex_protocol::approvals::NetworkPolicyAmendment;
pub(crate) use codex_protocol::approvals::NetworkPolicyRuleAction;
pub(crate) use codex_protocol::protocol::ApplyPatchApprovalRequestEvent;
pub(crate) use codex_protocol::protocol::AskForApproval;
pub(crate) use codex_protocol::protocol::EventMsg;
pub(crate) use codex_protocol::protocol::ExecApprovalRequestEvent;
pub(crate) use codex_protocol::protocol::ExecPolicyAmendment;
pub(crate) use codex_protocol::protocol::GranularApprovalConfig;
pub(crate) use codex_protocol::protocol::Op;
pub(crate) use codex_protocol::protocol::ReviewDecision;
pub(crate) use codex_protocol::protocol::SandboxPolicy;
pub(crate) use codex_protocol::user_input::UserInput;
pub(crate) use pretty_assertions::assert_eq;
pub(crate) use regex_lite::Regex;
pub(crate) use serde_json::Value;
pub(crate) use serde_json::json;
pub(crate) use std::env;
pub(crate) use std::fs;
pub(crate) use std::path::PathBuf;
pub(crate) use std::sync::Arc;
pub(crate) use std::time::Duration;
pub(crate) use tempfile::TempDir;
pub(crate) use wiremock::Mock;
pub(crate) use wiremock::MockServer;
pub(crate) use wiremock::Request;
pub(crate) use wiremock::ResponseTemplate;
pub(crate) use wiremock::matchers::method;
pub(crate) use wiremock::matchers::path;

#[derive(Clone, Copy)]
pub(crate) enum TargetPath {
    Workspace(&'static str),
    OutsideWorkspace(&'static str),
}

impl TargetPath {
    pub(crate) fn resolve_for_patch(self, test: &TestCodex) -> (PathBuf, String) {
        match self {
            TargetPath::Workspace(name) => {
                let path = test.cwd.path().join(name);
                (path, name.to_string())
            }
            TargetPath::OutsideWorkspace(name) => {
                let path = env::current_dir()
                    .expect("current dir should be available")
                    .join(name);
                (path.clone(), path.display().to_string())
            }
        }
    }
}

#[derive(Clone)]
pub(crate) enum ActionKind {
    WriteFile {
        target: TargetPath,
        content: &'static str,
    },
    FetchUrlNoProxy {
        endpoint: &'static str,
        response_body: &'static str,
    },
    FetchUrl {
        endpoint: &'static str,
        response_body: &'static str,
    },
    RunCommand {
        command: &'static str,
    },
    RunCommandWithPolicy {
        command: &'static str,
        policy_src: &'static str,
    },
    RunCommandWithPrefixRule {
        command: &'static str,
        prefix_rule: &'static [&'static str],
    },
    RunUnifiedExecCommand {
        command: &'static str,
        justification: Option<&'static str>,
    },
    ApplyPatchFreeform {
        target: TargetPath,
        content: &'static str,
    },
    ApplyPatchShell {
        target: TargetPath,
        content: &'static str,
    },
}

pub(crate) const DEFAULT_UNIFIED_EXEC_JUSTIFICATION: &str =
    "Requires escalated permissions to bypass the sandbox in tests.";

impl ActionKind {
    pub(crate) fn policy_src(&self) -> Option<&'static str> {
        match self {
            ActionKind::RunCommandWithPolicy { policy_src, .. } => Some(*policy_src),
            ActionKind::WriteFile { .. }
            | ActionKind::FetchUrlNoProxy { .. }
            | ActionKind::FetchUrl { .. }
            | ActionKind::RunCommand { .. }
            | ActionKind::RunCommandWithPrefixRule { .. }
            | ActionKind::RunUnifiedExecCommand { .. }
            | ActionKind::ApplyPatchFreeform { .. }
            | ActionKind::ApplyPatchShell { .. } => None,
        }
    }

    pub(crate) async fn prepare(
        &self,
        test: &TestCodex,
        server: &MockServer,
        call_id: &str,
        sandbox_permissions: SandboxPermissions,
    ) -> Result<(Value, Option<String>)> {
        match self {
            ActionKind::WriteFile { target, content } => {
                let (path, _) = target.resolve_for_patch(test);
                let _ = fs::remove_file(&path);
                let path_str = path.display().to_string();
                let script = format!(
                    "from pathlib import Path; path = Path({path_str:?}); content = {content:?}; path.write_text(content, encoding='utf-8'); print(path.read_text(encoding='utf-8'), end='')",
                );
                let command = format!("python3 -c {script:?}");
                let event = shell_event(
                    call_id,
                    &command,
                    /*timeout_ms*/ 5_000,
                    sandbox_permissions,
                )?;
                Ok((event, Some(command)))
            }
            ActionKind::FetchUrl {
                endpoint,
                response_body,
            } => {
                Mock::given(method("GET"))
                    .and(path(*endpoint))
                    .respond_with(
                        ResponseTemplate::new(200).set_body_string(response_body.to_string()),
                    )
                    .mount(server)
                    .await;

                let url = format!("{}{}", server.uri(), endpoint);
                let escaped_url = url.replace('\'', "\\'");
                let script = format!(
                    "import sys\nimport urllib.request\nurl = '{escaped_url}'\ntry:\n    data = urllib.request.urlopen(url, timeout=2).read().decode()\n    print('OK:' + data.strip())\nexcept Exception as exc:\n    print('ERR:' + exc.__class__.__name__)\n    sys.exit(1)",
                );

                let command = format!("python3 -c \"{script}\"");
                let event = shell_event(
                    call_id,
                    &command,
                    /*timeout_ms*/ 5_000,
                    sandbox_permissions,
                )?;
                Ok((event, Some(command)))
            }
            ActionKind::FetchUrlNoProxy {
                endpoint,
                response_body,
            } => {
                Mock::given(method("GET"))
                    .and(path(*endpoint))
                    .respond_with(
                        ResponseTemplate::new(200).set_body_string(response_body.to_string()),
                    )
                    .mount(server)
                    .await;

                let url = format!("{}{}", server.uri(), endpoint);
                let escaped_url = url.replace('\'', "\\'");
                let script = format!(
                    "import sys\nimport urllib.request\nurl = '{escaped_url}'\nopener = urllib.request.build_opener(urllib.request.ProxyHandler({{}}))\ntry:\n    data = opener.open(url, timeout=2).read().decode()\n    print('OK:' + data.strip())\nexcept Exception as exc:\n    print('ERR:' + exc.__class__.__name__)\n    sys.exit(1)",
                );

                let command = format!("python3 -c \"{script}\"");
                let event = shell_event(
                    call_id,
                    &command,
                    /*timeout_ms*/ 5_000,
                    sandbox_permissions,
                )?;
                Ok((event, Some(command)))
            }
            ActionKind::RunCommand { command } => {
                // Bazel Linux runners can be heavily oversubscribed while this
                // matrix runs, so avoid making scheduling latency look like an
                // approval behavior failure.
                let event = shell_event(
                    call_id,
                    command,
                    /*timeout_ms*/ 30_000,
                    sandbox_permissions,
                )?;
                Ok((event, Some(command.to_string())))
            }
            ActionKind::RunCommandWithPolicy { command, .. } => {
                // Bazel Linux runners can be heavily oversubscribed while this
                // matrix runs, so avoid making scheduling latency look like an
                // approval behavior failure.
                let event = shell_event(
                    call_id,
                    command,
                    /*timeout_ms*/ 30_000,
                    sandbox_permissions,
                )?;
                Ok((event, Some(command.to_string())))
            }
            ActionKind::RunCommandWithPrefixRule {
                command,
                prefix_rule,
            } => {
                let event = shell_event_with_prefix_rule(
                    call_id,
                    command,
                    /*timeout_ms*/ 30_000,
                    sandbox_permissions,
                    Some(prefix_rule.iter().map(|part| (*part).to_string()).collect()),
                )?;
                Ok((event, Some(command.to_string())))
            }
            ActionKind::RunUnifiedExecCommand {
                command,
                justification,
            } => {
                let event = exec_command_event(
                    call_id,
                    command,
                    Some(1000),
                    sandbox_permissions,
                    *justification,
                )?;
                Ok((event, Some(command.to_string())))
            }
            ActionKind::ApplyPatchFreeform { target, content } => {
                let (path, patch_path) = target.resolve_for_patch(test);
                let _ = fs::remove_file(&path);
                let patch = build_add_file_patch(&patch_path, content);
                Ok((ev_apply_patch_custom_tool_call(call_id, &patch), None))
            }
            ActionKind::ApplyPatchShell { target, content } => {
                let (path, patch_path) = target.resolve_for_patch(test);
                let _ = fs::remove_file(&path);
                let patch = build_add_file_patch(&patch_path, content);
                let command = shell_apply_patch_command(&patch);
                // Bazel may need to launch the configured Codex helper binary
                // to apply the verified patch, which can exceed the normal
                // short command timeout on slower CI runners.
                let timeout_ms = 30_000;
                let event = shell_event(call_id, &command, timeout_ms, sandbox_permissions)?;
                Ok((event, Some(command)))
            }
        }
    }
}

pub(crate) fn build_add_file_patch(patch_path: &str, content: &str) -> String {
    format!("*** Begin Patch\n*** Add File: {patch_path}\n+{content}\n*** End Patch\n")
}

pub(crate) fn shell_apply_patch_command(patch: &str) -> String {
    let mut script = String::from("apply_patch <<'PATCH'\n");
    script.push_str(patch);
    if !patch.ends_with('\n') {
        script.push('\n');
    }
    script.push_str("PATCH\n");
    script
}

pub(crate) fn shell_event(
    call_id: &str,
    command: &str,
    timeout_ms: u64,
    sandbox_permissions: SandboxPermissions,
) -> Result<Value> {
    shell_event_with_prefix_rule(
        call_id,
        command,
        timeout_ms,
        sandbox_permissions,
        /*prefix_rule*/ None,
    )
}

pub(crate) fn shell_event_with_prefix_rule(
    call_id: &str,
    command: &str,
    timeout_ms: u64,
    sandbox_permissions: SandboxPermissions,
    prefix_rule: Option<Vec<String>>,
) -> Result<Value> {
    let mut args = json!({
        "command": command,
        "timeout_ms": timeout_ms,
    });
    if sandbox_permissions.requests_sandbox_override() {
        args["sandbox_permissions"] = json!(sandbox_permissions);
    }
    if let Some(prefix_rule) = prefix_rule {
        args["prefix_rule"] = json!(prefix_rule);
    }
    let args_str = serde_json::to_string(&args)?;
    Ok(ev_function_call(call_id, "shell_command", &args_str))
}

pub(crate) fn exec_command_event(
    call_id: &str,
    cmd: &str,
    yield_time_ms: Option<u64>,
    sandbox_permissions: SandboxPermissions,
    justification: Option<&str>,
) -> Result<Value> {
    let mut args = json!({
        "cmd": cmd.to_string(),
    });
    if let Some(yield_time_ms) = yield_time_ms {
        args["yield_time_ms"] = json!(yield_time_ms);
    }
    if sandbox_permissions.requests_sandbox_override() {
        args["sandbox_permissions"] = json!(sandbox_permissions);
        let reason = justification.unwrap_or(DEFAULT_UNIFIED_EXEC_JUSTIFICATION);
        args["justification"] = json!(reason);
    }
    let args_str = serde_json::to_string(&args)?;
    Ok(ev_function_call(call_id, "exec_command", &args_str))
}

#[derive(Clone)]
pub(crate) enum Expectation {
    FileCreated {
        target: TargetPath,
        content: &'static str,
    },
    FileCreatedNoExitCode {
        target: TargetPath,
        content: &'static str,
    },
    PatchApplied {
        target: TargetPath,
        content: &'static str,
    },
    FileNotCreated {
        target: TargetPath,
        message_contains: &'static [&'static str],
    },
    NetworkSuccess {
        body_contains: &'static str,
    },
    NetworkSuccessNoExitCode {
        body_contains: &'static str,
    },
    NetworkFailure {
        expect_tag: &'static str,
    },
    CommandSuccess {
        stdout_contains: &'static str,
    },
    CommandSuccessNoExitCode {
        stdout_contains: &'static str,
    },
    CommandFailure {
        output_contains: &'static str,
    },
}

impl Expectation {
    pub(crate) fn verify(&self, test: &TestCodex, result: &CommandResult) -> Result<()> {
        match self {
            Expectation::FileCreated { target, content } => {
                let (path, _) = target.resolve_for_patch(test);
                assert_eq!(
                    result.exit_code,
                    Some(0),
                    "expected successful exit for {path:?}"
                );
                assert!(
                    result.stdout.contains(content),
                    "stdout missing {content:?}: {}",
                    result.stdout
                );
                let file_contents = fs::read_to_string(&path)?;
                assert!(
                    file_contents.contains(content),
                    "file contents missing {content:?}: {file_contents}"
                );
                let _ = fs::remove_file(path);
            }
            Expectation::FileCreatedNoExitCode { target, content } => {
                let (path, _) = target.resolve_for_patch(test);
                assert!(
                    result.exit_code.is_none() || result.exit_code == Some(0),
                    "expected no exit code for {path:?}",
                );
                assert!(
                    result.stdout.contains(content),
                    "stdout missing {content:?}: {}",
                    result.stdout
                );
                let file_contents = fs::read_to_string(&path)?;
                assert!(
                    file_contents.contains(content),
                    "file contents missing {content:?}: {file_contents}"
                );
                let _ = fs::remove_file(path);
            }
            Expectation::PatchApplied { target, content } => {
                let (path, _) = target.resolve_for_patch(test);
                match result.exit_code {
                    Some(0) | None => {
                        if result.exit_code.is_none() {
                            assert!(
                                result.stdout.contains("Success."),
                                "patch output missing success indicator: {}",
                                result.stdout
                            );
                        }
                    }
                    Some(code) => panic!(
                        "expected successful patch exit for {:?}, got {code} with stdout {}",
                        path, result.stdout
                    ),
                }
                let file_contents = fs::read_to_string(&path)?;
                assert!(
                    file_contents.contains(content),
                    "patched file missing {content:?}: {file_contents}"
                );
                let _ = fs::remove_file(path);
            }
            Expectation::FileNotCreated {
                target,
                message_contains,
            } => {
                let (path, _) = target.resolve_for_patch(test);
                assert_ne!(
                    result.exit_code,
                    Some(0),
                    "expected non-zero exit for {path:?}"
                );
                for needle in *message_contains {
                    if needle.contains('|') {
                        let options: Vec<&str> = needle.split('|').collect();
                        let matches_any =
                            options.iter().any(|option| result.stdout.contains(option));
                        assert!(
                            matches_any,
                            "stdout missing one of {options:?}: {}",
                            result.stdout
                        );
                    } else {
                        assert!(
                            result.stdout.contains(needle),
                            "stdout missing {needle:?}: {}",
                            result.stdout
                        );
                    }
                }
                assert!(
                    !path.exists(),
                    "command should not create {path:?}, but file exists"
                );
            }
            Expectation::NetworkSuccess { body_contains } => {
                assert_eq!(
                    result.exit_code,
                    Some(0),
                    "expected successful network exit: {}",
                    result.stdout
                );
                assert!(
                    result.stdout.contains("OK:"),
                    "stdout missing OK prefix: {}",
                    result.stdout
                );
                assert!(
                    result.stdout.contains(body_contains),
                    "stdout missing body text {body_contains:?}: {}",
                    result.stdout
                );
            }
            Expectation::NetworkSuccessNoExitCode { body_contains } => {
                assert!(
                    result.exit_code.is_none() || result.exit_code == Some(0),
                    "expected no exit code for successful network call: {}",
                    result.stdout
                );
                assert!(
                    result.stdout.contains("OK:"),
                    "stdout missing OK prefix: {}",
                    result.stdout
                );
                assert!(
                    result.stdout.contains(body_contains),
                    "stdout missing body text {body_contains:?}: {}",
                    result.stdout
                );
            }
            Expectation::NetworkFailure { expect_tag } => {
                assert_ne!(
                    result.exit_code,
                    Some(0),
                    "expected non-zero exit for network failure: {}",
                    result.stdout
                );
                assert!(
                    result.stdout.contains("ERR:"),
                    "stdout missing ERR prefix: {}",
                    result.stdout
                );
                assert!(
                    result.stdout.contains(expect_tag),
                    "stdout missing expected tag {expect_tag:?}: {}",
                    result.stdout
                );
            }
            Expectation::CommandSuccess { stdout_contains } => {
                assert_eq!(
                    result.exit_code,
                    Some(0),
                    "expected successful trusted command exit: {}",
                    result.stdout
                );
                assert!(
                    result.stdout.contains(stdout_contains),
                    "trusted command stdout missing {stdout_contains:?}: {}",
                    result.stdout
                );
            }
            Expectation::CommandSuccessNoExitCode { stdout_contains } => {
                assert!(
                    result.exit_code.is_none() || result.exit_code == Some(0),
                    "expected no exit code for trusted command: {}",
                    result.stdout
                );
                assert!(
                    result.stdout.contains(stdout_contains),
                    "trusted command stdout missing {stdout_contains:?}: {}",
                    result.stdout
                );
            }
            Expectation::CommandFailure { output_contains } => {
                assert_ne!(
                    result.exit_code,
                    Some(0),
                    "expected non-zero exit for command failure: {}",
                    result.stdout
                );
                assert!(
                    result.stdout.contains(output_contains),
                    "command failure stderr missing {output_contains:?}: {}",
                    result.stdout
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) enum Outcome {
    Auto,
    ExecApproval {
        decision: ReviewDecision,
        expected_reason: Option<&'static str>,
    },
    ExecApprovalWithAmendment {
        decision: ReviewDecision,
        expected_reason: Option<&'static str>,
        expected_execpolicy_amendment: Option<&'static [&'static str]>,
    },
    PatchApproval {
        decision: ReviewDecision,
        expected_reason: Option<&'static str>,
    },
}

#[derive(Clone)]
pub(crate) struct ScenarioSpec {
    pub(crate) name: &'static str,
    pub(crate) approval_policy: AskForApproval,
    pub(crate) sandbox_policy: SandboxPolicy,
    pub(crate) action: ActionKind,
    pub(crate) sandbox_permissions: SandboxPermissions,
    pub(crate) features: Vec<Feature>,
    pub(crate) model_override: Option<&'static str>,
    pub(crate) outcome: Outcome,
    pub(crate) expectation: Expectation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScenarioGroup {
    DangerFullAccess,
    ReadOnly,
    WorkspaceWrite,
    ApplyPatch,
    UnifiedExec,
}

pub(crate) struct CommandResult {
    pub(crate) exit_code: Option<i64>,
    pub(crate) stdout: String,
}

pub(crate) async fn submit_turn(
    test: &TestCodex,
    prompt: &str,
    approval_policy: AskForApproval,
    sandbox_policy: SandboxPolicy,
) -> Result<()> {
    let session_model = test.session_configured.model.clone();

    test.codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: prompt.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: test.cwd.path().to_path_buf(),
            approval_policy,
            approvals_reviewer: Some(ApprovalsReviewer::User),
            sandbox_policy,
            permission_profile: None,
            model: session_model,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    Ok(())
}

pub(crate) fn parse_result(item: &Value) -> CommandResult {
    let output_str = item
        .get("output")
        .and_then(Value::as_str)
        .expect("shell output payload");
    match serde_json::from_str::<Value>(output_str) {
        Ok(parsed) => {
            let exit_code = parsed["metadata"]["exit_code"].as_i64();
            let stdout = parsed["output"].as_str().unwrap_or_default().to_string();
            CommandResult { exit_code, stdout }
        }
        Err(_) => {
            let structured = Regex::new(r"(?s)^Exit code:\s*(-?\d+).*?Output:\n(.*)$").unwrap();
            let regex =
                Regex::new(r"(?s)^.*?Process exited with code (\d+)\n.*?Output:\n(.*)$").unwrap();
            // parse freeform output
            if let Some(captures) = structured.captures(output_str) {
                let exit_code = captures.get(1).unwrap().as_str().parse::<i64>().unwrap();
                let output = captures.get(2).unwrap().as_str();
                CommandResult {
                    exit_code: Some(exit_code),
                    stdout: output.to_string(),
                }
            } else if let Some(captures) = regex.captures(output_str) {
                let exit_code = captures.get(1).unwrap().as_str().parse::<i64>().unwrap();
                let output = captures.get(2).unwrap().as_str();
                CommandResult {
                    exit_code: Some(exit_code),
                    stdout: output.to_string(),
                }
            } else {
                CommandResult {
                    exit_code: None,
                    stdout: output_str.to_string(),
                }
            }
        }
    }
}

pub(crate) async fn expect_exec_approval(
    test: &TestCodex,
    expected_command: &str,
) -> ExecApprovalRequestEvent {
    let event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::ExecApprovalRequest(_) | EventMsg::TurnComplete(_)
        )
    })
    .await;

    match event {
        EventMsg::ExecApprovalRequest(approval) => {
            let last_arg = approval
                .command
                .last()
                .map(std::string::String::as_str)
                .unwrap_or_default();
            assert_eq!(last_arg, expected_command);
            approval
        }
        EventMsg::TurnComplete(_) => panic!("expected approval request before completion"),
        other => panic!("unexpected event: {other:?}"),
    }
}

pub(crate) async fn expect_patch_approval(
    test: &TestCodex,
    expected_call_id: &str,
) -> ApplyPatchApprovalRequestEvent {
    let event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::ApplyPatchApprovalRequest(_) | EventMsg::TurnComplete(_)
        )
    })
    .await;

    match event {
        EventMsg::ApplyPatchApprovalRequest(approval) => {
            assert_eq!(approval.call_id, expected_call_id);
            approval
        }
        EventMsg::TurnComplete(_) => panic!("expected patch approval request before completion"),
        other => panic!("unexpected event: {other:?}"),
    }
}

pub(crate) async fn wait_for_completion_without_approval(test: &TestCodex) {
    let event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::ExecApprovalRequest(_) | EventMsg::TurnComplete(_)
        )
    })
    .await;

    match event {
        EventMsg::TurnComplete(_) => {}
        EventMsg::ExecApprovalRequest(event) => {
            panic!("unexpected approval request: {:?}", event.command)
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

pub(crate) async fn wait_for_completion(test: &TestCodex) {
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
}

pub(crate) fn body_contains(req: &Request, text: &str) -> bool {
    let is_zstd = req
        .headers
        .get("content-encoding")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|entry| entry.trim().eq_ignore_ascii_case("zstd"))
        });
    let bytes = if is_zstd {
        zstd::stream::decode_all(std::io::Cursor::new(&req.body)).ok()
    } else {
        Some(req.body.clone())
    };
    bytes
        .and_then(|body| String::from_utf8(body).ok())
        .is_some_and(|body| body.contains(text))
}

pub(crate) async fn wait_for_spawned_thread(test: &TestCodex) -> Result<Arc<CodexThread>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        let ids = test.thread_manager.list_thread_ids().await;
        if let Some(thread_id) = ids
            .iter()
            .find(|id| **id != test.session_configured.thread_id)
        {
            return test
                .thread_manager
                .get_thread(*thread_id)
                .await
                .map_err(anyhow::Error::from);
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("timed out waiting for spawned thread");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
