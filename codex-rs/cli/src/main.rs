use clap::Args;
use clap::CommandFactory;
use clap::FromArgMatches;
use clap::Parser;
use codex_app_server_daemon::LifecycleCommand as AppServerLifecycleCommand;
use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_chatgpt::apply_command::ApplyCommand;
use codex_chatgpt::apply_command::run_apply_command;
use codex_cli::read_access_token_from_stdin;
use codex_cli::read_api_key_from_stdin;
use codex_cli::run_login_status;
use codex_cli::run_login_with_access_token;
use codex_cli::run_login_with_api_key;
use codex_cli::run_login_with_chatgpt;
use codex_cli::run_login_with_device_code;
use codex_cli::run_logout;
use codex_cloud_tasks::Cli as CloudTasksCli;
use codex_exec::Cli as ExecCli;
use codex_exec::Command as ExecCommand;
use codex_exec::ReviewArgs;
use codex_execpolicy::ExecPolicyCheckCommand;
use codex_responses_api_proxy::Args as ResponsesApiProxyArgs;
use codex_tui::Cli as TuiCli;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_cli::CliConfigOverrides;
use codex_utils_cli::ProfileV2Name;
use std::path::PathBuf;

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod app_cmd;
mod app_server_cmd;
mod cli_types;
mod completion;
mod debug_cmd;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod desktop_app;
mod dispatch;
mod doctor;
mod exec_server_cmd;
mod features_cmd;
mod interactive_cmd;
mod marketplace_cmd;
mod mcp_cmd;
mod plugin_cmd;
mod sandbox_cmd;
mod update_cmd;
#[cfg(not(windows))]
mod wsl_paths;

#[cfg(test)]
mod tests;

use crate::completion::CompletionCommand;
use crate::completion::print_completion;
use crate::marketplace_cmd::MarketplaceCli;
use crate::mcp_cmd::McpCli;
use doctor::DoctorCommand;

pub(crate) use cli_types::*;
pub(crate) use dispatch::*;

use codex_config::LoaderOverrides;
use codex_core::config::find_codex_home;
use codex_features::is_known_feature_key;

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        cli_main(arg0_paths).await?;
        Ok(())
    })
}
