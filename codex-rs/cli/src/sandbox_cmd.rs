//! Implements the `codex sandbox` subcommand dispatch.
//!
//! The body is moved verbatim from the inline `Subcommand::Sandbox` match arm in
//! `main.rs`. Platform selection of the host sandbox backend remains gated by the
//! same `#[cfg(...)]` attributes; behavior is unchanged.

use codex_arg0::Arg0DispatchPaths;
use codex_utils_cli::CliConfigOverrides;

use crate::HostSandboxArgs;
use crate::loader_overrides_for_profile;
use crate::prepend_config_flags;
use crate::reject_remote_mode_for_subcommand;
use codex_tui::Cli as TuiCli;

pub async fn run_sandbox(
    mut sandbox_cli: HostSandboxArgs,
    interactive: &TuiCli,
    root_config_overrides: CliConfigOverrides,
    arg0_paths: &Arg0DispatchPaths,
    root_remote: Option<&str>,
    root_remote_auth_token_env: Option<&str>,
) -> anyhow::Result<()> {
    reject_remote_mode_for_subcommand(root_remote, root_remote_auth_token_env, "sandbox")?;
    let config_profile = sandbox_cli
        .config_profile
        .as_ref()
        .or(interactive.config_profile_v2.as_ref());
    let loader_overrides = loader_overrides_for_profile(config_profile)?;
    prepend_config_flags(
        &mut sandbox_cli.config_overrides,
        root_config_overrides.clone(),
    );
    #[cfg(target_os = "macos")]
    codex_cli::run_command_under_seatbelt(
        sandbox_cli,
        arg0_paths.codex_linux_sandbox_exe.clone(),
        loader_overrides,
    )
    .await?;
    #[cfg(target_os = "linux")]
    codex_cli::run_command_under_landlock(
        sandbox_cli,
        arg0_paths.codex_linux_sandbox_exe.clone(),
        loader_overrides,
    )
    .await?;
    #[cfg(target_os = "windows")]
    codex_cli::run_command_under_windows_sandbox(
        sandbox_cli,
        arg0_paths.codex_linux_sandbox_exe.clone(),
        loader_overrides,
    )
    .await?;
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = loader_overrides;
        anyhow::bail!("`codex sandbox` is not supported on this operating system");
    }

    Ok(())
}
