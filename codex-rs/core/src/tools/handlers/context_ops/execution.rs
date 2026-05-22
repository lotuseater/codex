use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use codex_exec_server::ExecEnvPolicy;
use codex_exec_server::ExecOutputStream;
use codex_exec_server::ExecParams as ExecServerParams;
use codex_exec_server::ProcessId;
use codex_protocol::config_types::ShellEnvironmentPolicy;
use codex_utils_absolute_path::AbsolutePathBuf;

use crate::exec::ExecCapturePolicy;
use crate::exec::ExecExpiration;
use crate::exec::ExecParams;
use crate::exec::process_exec_tool_call;
use crate::exec_env::CODEX_THREAD_ID_ENV_VAR;
use crate::exec_env::create_env;
use crate::session::turn_context::TurnEnvironment;
use crate::tools::context::ToolInvocation;
use codex_tool_execution_api::FunctionCallError;

const CONTEXT_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const REMOTE_READ_WAIT: Duration = Duration::from_millis(100);

static NEXT_CONTEXT_PROCESS_ID: AtomicU64 = AtomicU64::new(1);

pub(super) struct ContextCommandOutput {
    pub(super) exit_code: i32,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
    pub(super) timed_out: bool,
}

impl ContextCommandOutput {
    pub(super) fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).to_string()
    }

    pub(super) fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).trim().to_string()
    }
}

pub(super) fn primary_environment(
    invocation: &ToolInvocation,
) -> Result<&TurnEnvironment, FunctionCallError> {
    invocation.turn.environments.primary().ok_or_else(|| {
        FunctionCallError::RespondToModel("context ops are unavailable in this session".to_string())
    })
}

pub(super) fn resolve_workdir(
    turn_environment: &TurnEnvironment,
    workdir: Option<&str>,
) -> AbsolutePathBuf {
    workdir.filter(|workdir| !workdir.is_empty()).map_or_else(
        || turn_environment.cwd.clone(),
        |workdir| turn_environment.cwd.join(workdir),
    )
}

pub(super) async fn read_file(
    invocation: &ToolInvocation,
    turn_environment: &TurnEnvironment,
    path: &AbsolutePathBuf,
) -> Result<Vec<u8>, FunctionCallError> {
    let mut sandbox = invocation
        .turn
        .file_system_sandbox_context(/*additional_permissions*/ None, &turn_environment.cwd);
    sandbox.cwd = Some(turn_environment.cwd.clone());
    turn_environment
        .environment
        .get_filesystem()
        .read_file(path, Some(&sandbox))
        .await
        .map_err(|err| FunctionCallError::RespondToModel(format!("failed to read file: {err}")))
}

pub(super) async fn run_command(
    invocation: &ToolInvocation,
    turn_environment: &TurnEnvironment,
    cwd: &AbsolutePathBuf,
    command: Vec<String>,
) -> Result<ContextCommandOutput, FunctionCallError> {
    if turn_environment.environment.is_remote() {
        run_remote_command(invocation, turn_environment, cwd, command).await
    } else {
        run_local_sandboxed_command(invocation, turn_environment, cwd, command).await
    }
}

async fn run_local_sandboxed_command(
    invocation: &ToolInvocation,
    turn_environment: &TurnEnvironment,
    cwd: &AbsolutePathBuf,
    command: Vec<String>,
) -> Result<ContextCommandOutput, FunctionCallError> {
    let output = process_exec_tool_call(
        ExecParams {
            command,
            cwd: cwd.clone(),
            expiration: ExecExpiration::DefaultTimeout,
            capture_policy: ExecCapturePolicy::FullBuffer,
            env: create_env(
                &invocation.turn.shell_environment_policy,
                Some(invocation.session.conversation_id),
            ),
            network: invocation.turn.network.clone(),
            sandbox_permissions: Default::default(),
            windows_sandbox_level: invocation.turn.windows_sandbox_level,
            windows_sandbox_private_desktop: invocation
                .turn
                .config
                .permissions
                .windows_sandbox_private_desktop,
            justification: None,
            arg0: None,
        },
        &invocation.turn.permission_profile(),
        &turn_environment.cwd,
        &invocation.turn.codex_linux_sandbox_exe,
        invocation.turn.features.use_legacy_landlock(),
        /*stdout_stream*/ None,
    )
    .await
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;

    Ok(ContextCommandOutput {
        exit_code: output.exit_code,
        stdout: output.stdout.text.into_bytes(),
        stderr: output.stderr.text.into_bytes(),
        timed_out: output.timed_out,
    })
}

async fn run_remote_command(
    invocation: &ToolInvocation,
    turn_environment: &TurnEnvironment,
    cwd: &AbsolutePathBuf,
    command: Vec<String>,
) -> Result<ContextCommandOutput, FunctionCallError> {
    let process_id = ProcessId::new(format!(
        "context-ops-{}",
        NEXT_CONTEXT_PROCESS_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let mut env = HashMap::new();
    env.insert(
        CODEX_THREAD_ID_ENV_VAR.to_string(),
        invocation.session.conversation_id.to_string(),
    );
    let started = turn_environment
        .environment
        .get_exec_backend()
        .start(ExecServerParams {
            process_id,
            argv: command,
            cwd: cwd.to_path_buf(),
            env_policy: Some(exec_env_policy_from_shell_policy(
                &invocation.turn.shell_environment_policy,
            )),
            env,
            tty: false,
            pipe_stdin: false,
            arg0: None,
        })
        .await
        .map_err(|err| {
            FunctionCallError::RespondToModel(format!("failed to run command: {err}"))
        })?;

    let process = started.process;
    let started_at = Instant::now();
    let mut after_seq = None;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_code = None;

    loop {
        let Some(remaining) = CONTEXT_COMMAND_TIMEOUT.checked_sub(started_at.elapsed()) else {
            let _ = process.terminate().await;
            return Ok(ContextCommandOutput {
                exit_code: exit_code.unwrap_or(-1),
                stdout,
                stderr,
                timed_out: true,
            });
        };
        let wait_ms = remaining.min(REMOTE_READ_WAIT).as_millis().max(1) as u64;
        let response = process
            .read(after_seq, /*max_bytes*/ None, Some(wait_ms))
            .await
            .map_err(|err| {
                FunctionCallError::RespondToModel(format!("failed to read command output: {err}"))
            })?;
        after_seq = Some(response.next_seq);
        if let Some(failure) = response.failure {
            return Err(FunctionCallError::RespondToModel(format!(
                "command failed: {failure}"
            )));
        }
        for chunk in response.chunks {
            match chunk.stream {
                ExecOutputStream::Stdout => stdout.extend(chunk.chunk.into_inner()),
                ExecOutputStream::Stderr => stderr.extend(chunk.chunk.into_inner()),
                ExecOutputStream::Pty => stdout.extend(chunk.chunk.into_inner()),
            }
        }
        if let Some(code) = response.exit_code {
            exit_code = Some(code);
        }
        if response.closed {
            break;
        }
    }

    Ok(ContextCommandOutput {
        exit_code: exit_code.unwrap_or(0),
        stdout,
        stderr,
        timed_out: false,
    })
}

fn exec_env_policy_from_shell_policy(policy: &ShellEnvironmentPolicy) -> ExecEnvPolicy {
    ExecEnvPolicy {
        inherit: policy.inherit.clone(),
        ignore_default_excludes: policy.ignore_default_excludes,
        exclude: policy.exclude.iter().map(ToString::to_string).collect(),
        r#set: policy.r#set.clone(),
        include_only: policy
            .include_only
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}
