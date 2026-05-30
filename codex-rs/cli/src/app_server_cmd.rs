//! Implements the `codex app-server` subcommand dispatch and its helpers.
//!
//! The destructure, validation, and nested subcommand match are moved verbatim
//! from the inline `Subcommand::AppServer` match arm in `main.rs`. Validation
//! order (strict-config then remote-mode rejection) and behavior are unchanged.

use codex_app_server_daemon::BootstrapOptions as AppServerBootstrapOptions;
use codex_app_server_daemon::LifecycleCommand as AppServerLifecycleCommand;
use codex_app_server_daemon::RemoteControlMode as AppServerRemoteControlMode;
use codex_arg0::Arg0DispatchPaths;
use codex_core::config::find_codex_home;
use codex_utils_cli::CliConfigOverrides;

use crate::AppServerCommand;
use crate::AppServerDaemonSubcommand;
use crate::AppServerSubcommand;
use crate::print_app_server_daemon_output;
use crate::reject_remote_mode_for_app_server_subcommand;
use crate::reject_strict_config_for_app_server_subcommand;

pub async fn run_app_server(
    app_server_cli: AppServerCommand,
    root_config_overrides: CliConfigOverrides,
    arg0_paths: Arg0DispatchPaths,
    root_strict_config: bool,
    root_remote: Option<&str>,
    root_remote_auth_token_env: Option<&str>,
) -> anyhow::Result<()> {
    let AppServerCommand {
        subcommand,
        strict_config: app_server_strict_config,
        listen,
        remote_control,
        analytics_default_enabled,
        auth,
    } = app_server_cli;
    let strict_config = app_server_strict_config || root_strict_config;
    reject_strict_config_for_app_server_subcommand(strict_config, subcommand.as_ref())?;
    reject_remote_mode_for_app_server_subcommand(
        root_remote,
        root_remote_auth_token_env,
        subcommand.as_ref(),
    )?;
    match subcommand {
        None => {
            let transport = listen;
            let auth = auth.try_into_settings()?;
            let runtime_options = codex_app_server::AppServerRuntimeOptions {
                remote_control_enabled: remote_control,
                ..Default::default()
            };
            codex_app_server::run_main_with_transport_options(
                arg0_paths,
                root_config_overrides,
                codex_config::LoaderOverrides::default(),
                strict_config,
                analytics_default_enabled,
                transport,
                codex_protocol::protocol::SessionSource::VSCode,
                auth,
                runtime_options,
            )
            .await?;
        }
        Some(AppServerSubcommand::Daemon(daemon_cli)) => match daemon_cli.subcommand {
            AppServerDaemonSubcommand::Start => {
                print_app_server_daemon_output(AppServerLifecycleCommand::Start).await?;
            }
            AppServerDaemonSubcommand::Bootstrap(bootstrap_cli) => {
                let output =
                    codex_app_server_daemon::bootstrap(AppServerBootstrapOptions {
                        remote_control_enabled: bootstrap_cli.remote_control,
                    })
                    .await?;
                println!("{}", serde_json::to_string(&output)?);
            }
            AppServerDaemonSubcommand::Restart => {
                print_app_server_daemon_output(AppServerLifecycleCommand::Restart).await?;
            }
            AppServerDaemonSubcommand::EnableRemoteControl => {
                print_app_server_remote_control_output(AppServerRemoteControlMode::Enabled)
                    .await?;
            }
            AppServerDaemonSubcommand::DisableRemoteControl => {
                print_app_server_remote_control_output(
                    AppServerRemoteControlMode::Disabled,
                )
                .await?;
            }
            AppServerDaemonSubcommand::Stop => {
                print_app_server_daemon_output(AppServerLifecycleCommand::Stop).await?;
            }
            AppServerDaemonSubcommand::Version => {
                print_app_server_daemon_output(AppServerLifecycleCommand::Version).await?;
            }
            AppServerDaemonSubcommand::PidUpdateLoop => {
                codex_app_server_daemon::run_pid_update_loop().await?;
            }
        },
        Some(AppServerSubcommand::Proxy(proxy_cli)) => {
            let socket_path = match proxy_cli.socket_path {
                Some(socket_path) => socket_path,
                None => {
                    let codex_home = find_codex_home()?;
                    codex_app_server::app_server_control_socket_path(&codex_home)?
                }
            };
            codex_stdio_to_uds::run(socket_path.as_path()).await?;
        }
        Some(AppServerSubcommand::GenerateTs(gen_cli)) => {
            let options = codex_app_server_protocol::GenerateTsOptions {
                experimental_api: gen_cli.experimental,
                ..Default::default()
            };
            codex_app_server_protocol::generate_ts_with_options(
                &gen_cli.out_dir,
                gen_cli.prettier.as_deref(),
                options,
            )?;
        }
        Some(AppServerSubcommand::GenerateJsonSchema(gen_cli)) => {
            codex_app_server_protocol::generate_json_with_experimental(
                &gen_cli.out_dir,
                gen_cli.experimental,
            )?;
        }
        Some(AppServerSubcommand::GenerateInternalJsonSchema(gen_cli)) => {
            codex_app_server_protocol::generate_internal_json_schema(&gen_cli.out_dir)?;
        }
    }

    Ok(())
}

async fn print_app_server_remote_control_output(
    mode: AppServerRemoteControlMode,
) -> anyhow::Result<()> {
    let output = codex_app_server_daemon::set_remote_control(mode).await?;
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
