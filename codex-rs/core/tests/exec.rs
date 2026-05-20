mod support;

#[cfg(not(target_os = "windows"))]
#[path = "suite/abort_tasks.rs"]
mod abort_tasks;
#[path = "suite/apply_patch_cli.rs"]
mod apply_patch_cli;
#[path = "suite/exec.rs"]
mod exec;
#[path = "suite/exec_policy.rs"]
mod exec_policy;
#[path = "suite/shell_command.rs"]
mod shell_command;
#[path = "suite/shell_serialization.rs"]
mod shell_serialization;
#[path = "suite/shell_snapshot.rs"]
mod shell_snapshot;
#[path = "suite/unified_exec.rs"]
mod unified_exec;
#[path = "suite/user_shell_cmd.rs"]
mod user_shell_cmd;
#[cfg(target_os = "windows")]
#[path = "suite/windows_sandbox.rs"]
mod windows_sandbox;
