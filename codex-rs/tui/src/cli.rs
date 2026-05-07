use clap::Args;
use clap::FromArgMatches;
use clap::Parser;
use codex_utils_cli::ApprovalModeCliArg;
use codex_utils_cli::CliConfigOverrides;
use codex_utils_cli::SharedCliOptions;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Cli {
    /// Optional user prompt to start the session.
    #[arg(value_name = "PROMPT", value_hint = clap::ValueHint::Other)]
    pub prompt: Option<String>,

    /// Resume a previous interactive session using the picker.
    #[arg(long = "resume", default_value_t = false)]
    pub resume: bool,

    // Internal controls set by the top-level `codex resume` subcommand.
    // These are not exposed as user flags on the base `codex` command.
    #[clap(skip)]
    pub resume_picker: bool,

    #[clap(skip)]
    pub resume_last: bool,

    /// Internal: resume a specific recorded session by id (UUID). Set by the
    /// top-level `codex resume <SESSION_ID>` wrapper; not exposed as a public flag.
    #[clap(skip)]
    pub resume_session_id: Option<String>,

    /// Internal: show all sessions (disables cwd filtering and shows CWD column).
    #[clap(skip)]
    pub resume_show_all: bool,

    /// Internal: include non-interactive sessions in resume listings.
    #[clap(skip)]
    pub resume_include_non_interactive: bool,

    // Internal controls set by the top-level `codex fork` subcommand.
    // These are not exposed as user flags on the base `codex` command.
    #[clap(skip)]
    pub fork_picker: bool,

    #[clap(skip)]
    pub fork_last: bool,

    /// Internal: fork a specific recorded session by id (UUID). Set by the
    /// top-level `codex fork <SESSION_ID>` wrapper; not exposed as a public flag.
    #[clap(skip)]
    pub fork_session_id: Option<String>,

    /// Internal: show all sessions (disables cwd filtering and shows CWD column).
    #[clap(skip)]
    pub fork_show_all: bool,

    #[clap(flatten)]
    pub shared: TuiSharedCliOptions,

    /// Configure when the model requires human approval before executing a command.
    #[arg(long = "ask-for-approval", short = 'a')]
    pub approval_policy: Option<ApprovalModeCliArg>,

    /// Enable live web search. When enabled, the native Responses `web_search` tool is available to the model (no per‑call approval).
    #[arg(long = "search", default_value_t = false)]
    pub web_search: bool,

    /// Disable alternate screen mode
    ///
    /// Runs the TUI in inline mode, preserving terminal scrollback history. This is useful
    /// in terminal multiplexers like Zellij that follow the xterm spec strictly and disable
    /// scrollback in alternate screen buffers.
    #[arg(long = "no-alt-screen", default_value_t = false)]
    pub no_alt_screen: bool,

    /// Keep the session moving by submitting a message after the TUI is idle.
    #[arg(long = "loop", default_value_t = false)]
    pub auto_loop: bool,

    /// Idle period before --loop submits its message.
    #[arg(long = "loop-period", value_parser = parse_loop_period, default_value = "5m")]
    pub auto_loop_period: Duration,

    /// Message submitted by --loop after the session is idle.
    #[arg(long = "loop-message", default_value = "go on")]
    pub auto_loop_message: String,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,
}

impl std::ops::Deref for Cli {
    type Target = SharedCliOptions;

    fn deref(&self) -> &Self::Target {
        &self.shared.0
    }
}

impl std::ops::DerefMut for Cli {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.shared.0
    }
}

#[derive(Debug, Default)]
pub struct TuiSharedCliOptions(SharedCliOptions);

impl TuiSharedCliOptions {
    pub fn into_inner(self) -> SharedCliOptions {
        self.0
    }
}

impl std::ops::Deref for TuiSharedCliOptions {
    type Target = SharedCliOptions;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for TuiSharedCliOptions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Args for TuiSharedCliOptions {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        mark_tui_args(SharedCliOptions::augment_args(cmd))
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        mark_tui_args(SharedCliOptions::augment_args_for_update(cmd))
    }
}

impl FromArgMatches for TuiSharedCliOptions {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        SharedCliOptions::from_arg_matches(matches).map(Self)
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        self.0.update_from_arg_matches(matches)
    }
}

fn mark_tui_args(cmd: clap::Command) -> clap::Command {
    cmd.mut_arg("dangerously_bypass_approvals_and_sandbox", |arg| {
        arg.conflicts_with("approval_policy")
    })
}

pub(crate) fn parse_loop_period(value: &str) -> Result<Duration, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("loop period must not be empty".to_string());
    }
    let (digits, unit) = value.split_at(
        value
            .find(|ch: char| !ch.is_ascii_digit())
            .unwrap_or(value.len()),
    );
    if digits.is_empty() {
        return Err("loop period must start with a number".to_string());
    }
    let amount = digits
        .parse::<u64>()
        .map_err(|_| "loop period number is invalid".to_string())?;
    if amount == 0 {
        return Err("loop period must be at least 1 second".to_string());
    }
    let seconds = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "s" | "sec" | "secs" | "second" | "seconds" => amount,
        "m" | "min" | "mins" | "minute" | "minutes" => amount
            .checked_mul(60)
            .ok_or_else(|| "loop period is too large".to_string())?,
        "h" | "hr" | "hrs" | "hour" | "hours" => amount
            .checked_mul(60 * 60)
            .ok_or_else(|| "loop period is too large".to_string())?,
        _ => {
            return Err("loop period unit must be seconds, minutes, or hours".to_string());
        }
    };
    Ok(Duration::from_secs(seconds))
}

pub(crate) fn format_loop_period(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds.is_multiple_of(3600) {
        format!("{}h", seconds / 3600)
    } else if seconds.is_multiple_of(60) {
        format!("{}m", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn loop_period_parses_seconds_minutes_and_hours() {
        assert_eq!(parse_loop_period("30s"), Ok(Duration::from_secs(30)));
        assert_eq!(parse_loop_period("5m"), Ok(Duration::from_secs(300)));
        assert_eq!(parse_loop_period("1h"), Ok(Duration::from_secs(3600)));
    }

    #[test]
    fn loop_period_rejects_zero_and_unknown_units() {
        assert!(parse_loop_period("0s").is_err());
        assert!(parse_loop_period("5d").is_err());
    }
}
