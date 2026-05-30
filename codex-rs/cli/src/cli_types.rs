use super::*;

/// Codex CLI
///
/// If no subcommand is specified, options will be forwarded to the interactive CLI.
#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    // If a sub‑command is given, ignore requirements of the default args.
    subcommand_negates_reqs = true,
    // The executable is sometimes invoked via a platform‑specific name like
    // `codex-x86_64-unknown-linux-musl`, but the help output should always use
    // the generic `codex` command name that users run.
    bin_name = "codex",
    override_usage = "codex [OPTIONS] [PROMPT]\n       codex [OPTIONS] <COMMAND> [ARGS]"
)]
pub(crate) struct MultitoolCli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    pub feature_toggles: FeatureToggles,

    #[clap(flatten)]
    pub(crate) remote: InteractiveRemoteOptions,

    #[clap(flatten)]
    pub(crate) interactive: TuiCli,

    #[clap(subcommand)]
    pub(crate) subcommand: Option<Subcommand>,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Subcommand {
    /// Run Codex non-interactively.
    #[clap(visible_alias = "e")]
    Exec(ExecCli),

    /// Run a code review non-interactively.
    Review(ReviewCommand),

    /// Manage login.
    Login(LoginCommand),

    /// Remove stored authentication credentials.
    Logout(LogoutCommand),

    /// Manage external MCP servers for Codex.
    Mcp(McpCli),

    /// Manage Codex plugins.
    Plugin(PluginCli),

    /// Start Codex as an MCP server (stdio).
    McpServer(McpServerCommand),

    /// [experimental] Run the app server or related tooling.
    AppServer(AppServerCommand),

    /// [experimental] Manage the app-server daemon with remote control enabled.
    RemoteControl(RemoteControlCommand),

    /// Launch the Codex desktop app (opens the app installer if missing).
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    App(app_cmd::AppCommand),

    /// Generate shell completion scripts.
    Completion(CompletionCommand),

    /// Update Codex to the latest version.
    Update,

    /// Diagnose local Codex installation, config, auth, and runtime health.
    Doctor(DoctorCommand),

    /// Run commands within a Codex-provided sandbox.
    Sandbox(HostSandboxArgs),

    /// Debugging tools.
    Debug(DebugCommand),

    /// Execpolicy tooling.
    #[clap(hide = true)]
    Execpolicy(ExecpolicyCommand),

    /// Apply the latest diff produced by Codex agent as a `git apply` to your local working tree.
    #[clap(visible_alias = "a")]
    Apply(ApplyCommand),

    /// Resume a previous interactive session (picker by default; use --last to continue the most recent).
    Resume(ResumeCommand),

    /// Fork a previous interactive session (picker by default; use --last to fork the most recent).
    Fork(ForkCommand),

    /// [EXPERIMENTAL] Browse tasks from Codex Cloud and apply changes locally.
    #[clap(name = "cloud", alias = "cloud-tasks")]
    Cloud(CloudTasksCli),

    /// Internal: run the responses API proxy.
    #[clap(hide = true)]
    ResponsesApiProxy(ResponsesApiProxyArgs),

    /// Internal: relay stdio to a Unix domain socket.
    #[clap(hide = true, name = "stdio-to-uds")]
    StdioToUds(StdioToUdsCommand),

    /// [EXPERIMENTAL] Run the standalone exec-server service.
    ExecServer(ExecServerCommand),

    /// Inspect feature flags.
    Features(FeaturesCli),
}

#[derive(Debug, Parser)]
#[command(bin_name = "codex plugin")]
pub(crate) struct PluginCli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    pub(crate) subcommand: PluginSubcommand,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum PluginSubcommand {
    /// Manage plugin marketplaces for Codex.
    Marketplace(MarketplaceCli),
}

#[derive(Debug, Parser)]
pub(crate) struct DebugCommand {
    #[command(subcommand)]
    pub(crate) subcommand: DebugSubcommand,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum DebugSubcommand {
    /// Render the raw model catalog as JSON.
    Models(DebugModelsCommand),

    /// Render the model-visible prompt input list as JSON.
    PromptInput(DebugPromptInputCommand),

    /// Replay a rollout trace bundle and write reduced state JSON.
    #[clap(hide = true)]
    TraceReduce(DebugTraceReduceCommand),

    /// Internal: reset local memory state for a fresh start.
    #[clap(hide = true)]
    ClearMemories,
}

#[derive(Debug, Parser)]
pub(crate) struct DebugPromptInputCommand {
    /// Optional user prompt to append after session context.
    #[arg(value_name = "PROMPT")]
    pub(crate) prompt: Option<String>,

    /// Optional image(s) to attach to the user prompt.
    #[arg(long = "image", short = 'i', value_name = "FILE", value_delimiter = ',', num_args = 1..)]
    pub(crate) images: Vec<PathBuf>,
}

#[derive(Debug, Parser)]
pub(crate) struct DebugModelsCommand {
    /// Skip refresh and dump only the bundled catalog shipped with this binary.
    #[arg(long = "bundled", default_value_t = false)]
    pub(crate) bundled: bool,
}

#[derive(Debug, Parser)]
pub(crate) struct ReviewCommand {
    /// Error out when config.toml contains fields that are not recognized by this version of Codex.
    #[arg(long = "strict-config", default_value_t = false)]
    pub(crate) strict_config: bool,

    #[clap(flatten)]
    pub(crate) args: ReviewArgs,
}

#[derive(Debug, Parser)]
pub(crate) struct McpServerCommand {
    /// Error out when config.toml contains fields that are not recognized by this version of Codex.
    #[arg(long = "strict-config", default_value_t = false)]
    pub(crate) strict_config: bool,
}

#[derive(Debug, Parser)]
pub(crate) struct DebugTraceReduceCommand {
    /// Trace bundle directory containing manifest.json and trace.jsonl.
    #[arg(value_name = "TRACE_BUNDLE")]
    pub(crate) trace_bundle: PathBuf,

    /// Output path for reduced RolloutTrace JSON. Defaults to TRACE_BUNDLE/state.json.
    #[arg(long = "output", short = 'o', value_name = "FILE")]
    pub(crate) output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub(crate) struct ResumeCommand {
    /// Conversation/session id (UUID) or thread name. UUIDs take precedence if it parses.
    /// If omitted, use --last to pick the most recent recorded session.
    #[arg(value_name = "SESSION_ID")]
    pub(crate) session_id: Option<String>,

    /// Continue the most recent session without showing the picker.
    #[arg(long = "last", default_value_t = false)]
    pub(crate) last: bool,

    /// Show all sessions (disables cwd filtering and shows CWD column).
    #[arg(long = "all", default_value_t = false)]
    pub(crate) all: bool,

    /// Include non-interactive sessions in the resume picker and --last selection.
    #[arg(long = "include-non-interactive", default_value_t = false)]
    pub(crate) include_non_interactive: bool,

    #[clap(flatten)]
    pub(crate) remote: InteractiveRemoteOptions,

    #[clap(flatten)]
    pub(crate) config_overrides: TuiCli,
}

#[derive(Debug, Parser)]
pub(crate) struct ForkCommand {
    /// Conversation/session id (UUID). When provided, forks this session.
    /// If omitted, use --last to pick the most recent recorded session.
    #[arg(value_name = "SESSION_ID")]
    pub(crate) session_id: Option<String>,

    /// Fork the most recent session without showing the picker.
    #[arg(long = "last", default_value_t = false, conflicts_with = "session_id")]
    pub(crate) last: bool,

    /// Show all sessions (disables cwd filtering and shows CWD column).
    #[arg(long = "all", default_value_t = false)]
    pub(crate) all: bool,

    #[clap(flatten)]
    pub(crate) remote: InteractiveRemoteOptions,

    #[clap(flatten)]
    pub(crate) config_overrides: TuiCli,
}

#[cfg(target_os = "macos")]
pub(crate) type HostSandboxArgs = codex_cli::SeatbeltCommand;
#[cfg(target_os = "linux")]
pub(crate) type HostSandboxArgs = codex_cli::LandlockCommand;
#[cfg(target_os = "windows")]
pub(crate) type HostSandboxArgs = codex_cli::WindowsCommand;

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub(crate) type HostSandboxArgs = UnsupportedSandboxArgs;

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
#[derive(Debug, Parser)]
pub(crate) struct UnsupportedSandboxArgs {
    /// Layer $CODEX_HOME/<name>.config.toml on top of the base user config.
    #[arg(long = "profile", short = 'p')]
    pub config_profile: Option<ProfileV2Name>,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    /// Full command args to run under the host sandbox.
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Parser)]
pub(crate) struct ExecpolicyCommand {
    #[command(subcommand)]
    pub(crate) sub: ExecpolicySubcommand,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum ExecpolicySubcommand {
    /// Check execpolicy files against a command.
    #[clap(name = "check")]
    Check(ExecPolicyCheckCommand),
}

#[derive(Debug, Parser)]
pub(crate) struct LoginCommand {
    #[clap(skip)]
    pub(crate) config_overrides: CliConfigOverrides,

    #[arg(
        long = "with-api-key",
        help = "Read the API key from stdin (e.g. `printenv OPENAI_API_KEY | codex login --with-api-key`)"
    )]
    pub(crate) with_api_key: bool,

    #[arg(
        long = "with-access-token",
        help = "Read the access token from stdin (e.g. `printenv CODEX_ACCESS_TOKEN | codex login --with-access-token`)"
    )]
    pub(crate) with_access_token: bool,

    #[arg(
        long = "api-key",
        num_args = 0..=1,
        default_missing_value = "",
        value_name = "API_KEY",
        help = "(deprecated) Previously accepted the API key directly; now exits with guidance to use --with-api-key",
        hide = true
    )]
    pub(crate) api_key: Option<String>,

    #[arg(long = "device-auth")]
    pub(crate) use_device_code: bool,

    /// EXPERIMENTAL: Use custom OAuth issuer base URL (advanced)
    /// Override the OAuth issuer base URL (advanced)
    #[arg(long = "experimental_issuer", value_name = "URL", hide = true)]
    pub(crate) issuer_base_url: Option<String>,

    /// EXPERIMENTAL: Use custom OAuth client ID (advanced)
    #[arg(long = "experimental_client-id", value_name = "CLIENT_ID", hide = true)]
    pub(crate) client_id: Option<String>,

    #[command(subcommand)]
    pub(crate) action: Option<LoginSubcommand>,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum LoginSubcommand {
    /// Show login status.
    Status,
}

#[derive(Debug, Parser)]
pub(crate) struct LogoutCommand {
    #[clap(skip)]
    pub(crate) config_overrides: CliConfigOverrides,
}

#[derive(Debug, Parser)]
pub(crate) struct AppServerCommand {
    /// Omit to run the app server; specify a subcommand for tooling.
    #[command(subcommand)]
    pub(crate) subcommand: Option<AppServerSubcommand>,

    /// Error out when config.toml contains fields that are not recognized by this version of Codex.
    #[arg(long = "strict-config", default_value_t = false)]
    pub(crate) strict_config: bool,

    /// Transport endpoint URL. Supported values: `stdio://` (default),
    /// `unix://`, `unix://PATH`, `ws://IP:PORT`, `off`.
    #[arg(
        long = "listen",
        value_name = "URL",
        default_value = codex_app_server::AppServerTransport::DEFAULT_LISTEN_URL
    )]
    pub(crate) listen: codex_app_server::AppServerTransport,

    /// Enable remote control for this app-server process.
    #[arg(long = "remote-control", hide = true)]
    pub(crate) remote_control: bool,

    /// Controls whether analytics are enabled by default.
    ///
    /// Analytics are disabled by default for app-server. Users have to explicitly opt in
    /// via the `analytics` section in the config.toml file.
    ///
    /// However, for first-party use cases like the VSCode IDE extension, we default analytics
    /// to be enabled by default by setting this flag. Users can still opt out by setting this
    /// in their config.toml:
    ///
    /// ```toml
    /// [analytics]
    /// enabled = false
    /// ```
    ///
    /// See https://developers.openai.com/codex/config-advanced/#metrics for more details.
    #[arg(long = "analytics-default-enabled")]
    pub(crate) analytics_default_enabled: bool,

    #[command(flatten)]
    pub(crate) auth: codex_app_server::AppServerWebsocketAuthArgs,
}

#[derive(Debug, Parser)]
pub(crate) struct ExecServerCommand {
    /// Error out when config.toml contains fields that are not recognized by this version of Codex.
    #[arg(long = "strict-config", default_value_t = false)]
    pub(crate) strict_config: bool,

    /// Transport endpoint URL. Supported values: `ws://IP:PORT` (default), `stdio`, `stdio://`.
    #[arg(long = "listen", value_name = "URL", conflicts_with = "remote")]
    pub(crate) listen: Option<String>,

    /// Register this exec-server as a remote environment using the given base URL.
    #[arg(long = "remote", value_name = "URL", requires = "environment_id")]
    pub(crate) remote: Option<String>,

    /// Environment id to attach to when registering remotely.
    #[arg(long = "environment-id", value_name = "ID")]
    pub(crate) environment_id: Option<String>,

    /// Human-readable environment name.
    #[arg(long = "name", value_name = "NAME")]
    pub(crate) name: Option<String>,

    /// Use Agent Identity auth from CODEX_ACCESS_TOKEN for remote registration.
    #[arg(long = "use-agent-identity-auth", requires = "remote")]
    pub(crate) use_agent_identity_auth: bool,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum AppServerSubcommand {
    /// Manage the local app-server daemon.
    Daemon(AppServerDaemonCommand),

    /// Proxy stdio bytes to the running app-server control socket.
    Proxy(AppServerProxyCommand),

    /// [experimental] Generate TypeScript bindings for the app server protocol.
    GenerateTs(GenerateTsCommand),

    /// [experimental] Generate JSON Schema for the app server protocol.
    GenerateJsonSchema(GenerateJsonSchemaCommand),

    /// [internal] Generate internal JSON Schema artifacts for Codex tooling.
    #[clap(hide = true)]
    GenerateInternalJsonSchema(GenerateInternalJsonSchemaCommand),
}

#[derive(Debug, Args)]
pub(crate) struct AppServerDaemonCommand {
    #[command(subcommand)]
    pub(crate) subcommand: AppServerDaemonSubcommand,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum AppServerDaemonSubcommand {
    /// Install durable local app-server management for SSH-driven use.
    Bootstrap(AppServerBootstrapCommand),

    /// Start the local app server daemon if it is not already running.
    Start,

    /// Restart the local app server daemon.
    Restart,

    /// Enable remote control for future starts and a currently running managed daemon.
    EnableRemoteControl,

    /// Disable remote control for future starts and a currently running managed daemon.
    DisableRemoteControl,

    /// Stop the local app server daemon.
    Stop,

    /// Print local CLI and running app-server versions as JSON.
    Version,

    /// [internal] Run the detached pid-backed standalone updater loop.
    #[clap(hide = true)]
    PidUpdateLoop,
}

#[derive(Debug, Args)]
pub(crate) struct AppServerProxyCommand {
    /// Path to the app-server Unix domain socket to connect to.
    #[arg(long = "sock", value_name = "SOCKET_PATH", value_parser = parse_socket_path)]
    pub(crate) socket_path: Option<AbsolutePathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct AppServerBootstrapCommand {
    /// Launch the managed app-server with remote control enabled.
    #[arg(long = "remote-control")]
    pub(crate) remote_control: bool,
}

#[derive(Debug, Args)]
pub(crate) struct RemoteControlCommand {
    #[command(subcommand)]
    pub(crate) subcommand: Option<RemoteControlSubcommand>,
}

#[derive(Debug, Clone, Copy, clap::Subcommand)]
pub(crate) enum RemoteControlSubcommand {
    /// Start the app-server daemon with remote control enabled.
    Start,

    /// Stop the app-server daemon.
    Stop,
}

#[derive(Debug, Args)]
pub(crate) struct GenerateTsCommand {
    /// Output directory where .ts files will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    pub(crate) out_dir: PathBuf,

    /// Optional path to the Prettier executable to format generated files
    #[arg(short = 'p', long = "prettier", value_name = "PRETTIER_BIN")]
    pub(crate) prettier: Option<PathBuf>,

    /// Include experimental methods and fields in the generated output
    #[arg(long = "experimental", default_value_t = false)]
    pub(crate) experimental: bool,
}

#[derive(Debug, Args)]
pub(crate) struct GenerateJsonSchemaCommand {
    /// Output directory where the schema bundle will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    pub(crate) out_dir: PathBuf,

    /// Include experimental methods and fields in the generated output
    #[arg(long = "experimental", default_value_t = false)]
    pub(crate) experimental: bool,
}

#[derive(Debug, Args)]
pub(crate) struct GenerateInternalJsonSchemaCommand {
    /// Output directory where internal JSON Schema artifacts will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    pub(crate) out_dir: PathBuf,
}

#[derive(Debug, Parser)]
pub(crate) struct StdioToUdsCommand {
    /// Path to the Unix domain socket to connect to.
    #[arg(value_name = "SOCKET_PATH", value_parser = parse_socket_path)]
    pub(crate) socket_path: AbsolutePathBuf,
}

pub(crate) fn parse_socket_path(raw: &str) -> Result<AbsolutePathBuf, String> {
    AbsolutePathBuf::relative_to_current_dir(raw)
        .map_err(|err| format!("failed to resolve socket path `{raw}`: {err}"))
}

pub(crate) fn run_execpolicycheck(cmd: ExecPolicyCheckCommand) -> anyhow::Result<()> {
    cmd.run()
}

#[derive(Debug, Default, Parser, Clone)]
pub(crate) struct FeatureToggles {
    /// Enable a feature (repeatable). Equivalent to `-c features.<name>=true`.
    #[arg(long = "enable", value_name = "FEATURE", action = clap::ArgAction::Append, global = true)]
    pub(crate) enable: Vec<String>,

    /// Disable a feature (repeatable). Equivalent to `-c features.<name>=false`.
    #[arg(long = "disable", value_name = "FEATURE", action = clap::ArgAction::Append, global = true)]
    pub(crate) disable: Vec<String>,
}

#[derive(Debug, Default, Parser, Clone)]
pub(crate) struct InteractiveRemoteOptions {
    /// Connect the TUI to a remote app server endpoint.
    ///
    /// Accepted forms: `ws://host:port`, `wss://host:port`, `unix://`, or `unix://PATH`.
    #[arg(long = "remote", value_name = "ADDR")]
    pub(crate) remote: Option<String>,

    /// Name of the environment variable containing the bearer token to send to
    /// a remote app server websocket.
    #[arg(long = "remote-auth-token-env", value_name = "ENV_VAR")]
    pub(crate) remote_auth_token_env: Option<String>,
}

impl FeatureToggles {
    pub(crate) fn to_overrides(&self) -> anyhow::Result<Vec<String>> {
        let mut v = Vec::new();
        for feature in &self.enable {
            Self::validate_feature(feature)?;
            v.push(format!("features.{feature}=true"));
        }
        for feature in &self.disable {
            Self::validate_feature(feature)?;
            v.push(format!("features.{feature}=false"));
        }
        Ok(v)
    }

    pub(crate) fn validate_feature(feature: &str) -> anyhow::Result<()> {
        if is_known_feature_key(feature) {
            Ok(())
        } else {
            anyhow::bail!("Unknown feature flag: {feature}")
        }
    }
}

#[derive(Debug, Parser)]
pub(crate) struct FeaturesCli {
    #[command(subcommand)]
    pub(crate) sub: FeaturesSubcommand,
}

#[derive(Debug, Parser)]
pub(crate) enum FeaturesSubcommand {
    /// List known features with their stage and effective state.
    List,
    /// Enable a feature in config.toml.
    Enable(FeatureSetArgs),
    /// Disable a feature in config.toml.
    Disable(FeatureSetArgs),
}

#[derive(Debug, Parser)]
pub(crate) struct FeatureSetArgs {
    /// Feature key to update (for example: unified_exec).
    pub(crate) feature: String,
}
