use clap::Parser;
use codex_app_server::AppServerRuntimeOptions;
use codex_app_server::AppServerTransport;
use codex_app_server::AppServerWebsocketAuthArgs;
use codex_app_server::PluginStartupTasks;
use codex_app_server::run_main_with_transport_options;
use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_config::LoaderOverrides;
use codex_protocol::protocol::SessionSource;
use codex_utils_cli::CliConfigOverrides;
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct AppServerArgs {
    /// Transport endpoint URL. Supported values: `stdio://` (default),
    /// `unix://`, `unix://PATH`, `ws://IP:PORT`, `off`.
    #[arg(
        long = "listen",
        value_name = "URL",
        default_value = AppServerTransport::DEFAULT_LISTEN_URL
    )]
    listen: AppServerTransport,

    /// Session source used to derive product restrictions and metadata.
    #[arg(
        long = "session-source",
        value_name = "SOURCE",
        default_value = "vscode",
        value_parser = SessionSource::from_startup_arg
    )]
    session_source: SessionSource,

    #[command(flatten)]
    auth: AppServerWebsocketAuthArgs,

    /// Hidden test hook used by integration tests that spawn the production
    /// app-server binary.
    #[arg(long = "disable-plugin-startup-tasks-for-tests", hide = true)]
    disable_plugin_startup_tasks_for_tests: bool,

    /// Hidden test hook used by integration tests that spawn the production
    /// app-server binary.
    #[arg(
        long = "disable-managed-config-for-tests",
        hide = true,
        conflicts_with = "managed_config_path_for_tests"
    )]
    disable_managed_config_for_tests: bool,

    /// Hidden test hook used by integration tests that spawn the production
    /// app-server binary.
    #[arg(
        long = "managed-config-path-for-tests",
        value_name = "PATH",
        hide = true,
        conflicts_with = "disable_managed_config_for_tests"
    )]
    managed_config_path_for_tests: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        let args = AppServerArgs::parse();
        let loader_overrides = loader_overrides_from_args(&args);
        let transport = args.listen;
        let session_source = args.session_source;
        let auth = args.auth.try_into_settings()?;
        let mut runtime_options = AppServerRuntimeOptions::default();
        if args.disable_plugin_startup_tasks_for_tests {
            runtime_options.plugin_startup_tasks = PluginStartupTasks::Skip;
        }

        run_main_with_transport_options(
            arg0_paths,
            CliConfigOverrides::default(),
            loader_overrides,
            /*default_analytics_enabled*/ false,
            transport,
            session_source,
            auth,
            runtime_options,
        )
        .await?;
        Ok(())
    })
}

fn loader_overrides_from_args(args: &AppServerArgs) -> LoaderOverrides {
    if args.disable_managed_config_for_tests {
        LoaderOverrides::without_managed_config_for_tests()
    } else if let Some(path) = args.managed_config_path_for_tests.clone() {
        LoaderOverrides::with_managed_config_path_for_tests(path)
    } else {
        LoaderOverrides::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn loader_overrides_default_args_use_default_loader_overrides() {
        let args = AppServerArgs::try_parse_from(["codex-app-server"]).unwrap();

        assert_eq!(
            loader_overrides_from_args(&args),
            LoaderOverrides::default()
        );
    }

    #[test]
    fn loader_overrides_disable_managed_config_arg_uses_test_overrides() {
        let args = AppServerArgs::try_parse_from([
            "codex-app-server",
            "--disable-managed-config-for-tests",
        ])
        .unwrap();

        assert_eq!(
            loader_overrides_from_args(&args),
            LoaderOverrides::without_managed_config_for_tests()
        );
    }

    #[test]
    fn loader_overrides_managed_config_path_arg_uses_explicit_path() {
        let managed_config_path = PathBuf::from("managed_config.toml");
        let args = AppServerArgs::try_parse_from([
            "codex-app-server",
            "--managed-config-path-for-tests",
            "managed_config.toml",
        ])
        .unwrap();

        assert_eq!(
            loader_overrides_from_args(&args),
            LoaderOverrides::with_managed_config_path_for_tests(managed_config_path)
        );
    }

    #[test]
    fn loader_overrides_managed_config_test_args_conflict() {
        let result = AppServerArgs::try_parse_from([
            "codex-app-server",
            "--disable-managed-config-for-tests",
            "--managed-config-path-for-tests",
            "managed_config.toml",
        ]);

        assert!(result.is_err());
    }
}
