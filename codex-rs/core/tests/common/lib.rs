#![expect(clippy::expect_used)]

use codex_core::CodexThread;

pub use codex_test_support_lightweight::PathBufExt;
pub use codex_test_support_lightweight::PathExt;
pub use codex_test_support_lightweight::TempDirExt;
pub use codex_test_support_lightweight::fs_wait;
pub use codex_test_support_lightweight::remote_env_env_var;
pub use codex_test_support_lightweight::skip_if_remote;
pub use codex_test_support_lightweight::skip_if_windows;
pub use codex_test_support_lightweight::test_absolute_path;
pub use codex_test_support_lightweight::test_absolute_path_with_windows;
pub use codex_test_support_lightweight::test_path_buf;
pub use codex_test_support_lightweight::test_path_buf_with_windows;
pub use codex_test_support_lightweight::test_tmp_path;
pub use codex_test_support_lightweight::test_tmp_path_buf;
pub use codex_test_support_responses::streaming_sse;

pub mod context_snapshot;
pub mod process;
pub mod protocol_fixtures;
pub mod responses {
    pub use codex_test_support_responses::responses::*;
}
mod runtime_harness;
pub mod tracing;
pub mod zsh_fork;

pub use protocol_fixtures::RemoteEnvConfig;
pub use protocol_fixtures::assert_regex_match;
pub use protocol_fixtures::fetch_dotslash_file;
pub use protocol_fixtures::get_remote_test_env;
pub use protocol_fixtures::load_sse_fixture_with_id_from_str;
pub use protocol_fixtures::stdio_server_bin;
#[cfg(target_os = "linux")]
pub use runtime_harness::find_codex_linux_sandbox_exe;
pub use runtime_harness::format_with_current_shell;
pub use runtime_harness::format_with_current_shell_display_non_login;
pub use runtime_harness::format_with_current_shell_non_login;
pub use runtime_harness::sandbox_env_var;
pub use runtime_harness::sandbox_network_env_var;

/// Waits for a configured MCP server to finish startup and requires it to be ready.
pub async fn wait_for_mcp_server(codex: &CodexThread, server_name: &str) -> anyhow::Result<()> {
    use codex_protocol::protocol::EventMsg;

    // Wait for the startup summary regardless of outcome, then interpret the
    // requested server's ready, failed, or cancelled entry below.
    let summary = loop {
        let event = codex
            .next_event()
            .await
            .expect("stream ended unexpectedly while waiting for MCP startup");
        if let EventMsg::McpStartupComplete(summary) = event.msg {
            break summary;
        }
    };
    if let Some(failure) = summary
        .failed
        .iter()
        .find(|failure| failure.server == server_name)
    {
        let error = &failure.error;
        anyhow::bail!("MCP server {server_name} failed to start: {error}");
    }
    if summary.cancelled.iter().any(|server| server == server_name) {
        anyhow::bail!("MCP server {server_name} startup was cancelled");
    }
    assert!(
        summary.ready.iter().any(|server| server == server_name),
        "expected MCP server {server_name} to be ready; startup summary: {summary:?}"
    );
    Ok(())
}

#[macro_export]
macro_rules! skip_if_sandbox {
    () => {{
        if ::std::env::var($crate::sandbox_env_var())
            == ::core::result::Result::Ok("seatbelt".to_string())
        {
            eprintln!(
                "{} is set to 'seatbelt', skipping test.",
                $crate::sandbox_env_var()
            );
            return;
        }
    }};
    ($return_value:expr $(,)?) => {{
        if ::std::env::var($crate::sandbox_env_var())
            == ::core::result::Result::Ok("seatbelt".to_string())
        {
            eprintln!(
                "{} is set to 'seatbelt', skipping test.",
                $crate::sandbox_env_var()
            );
            return $return_value;
        }
    }};
}

#[macro_export]
macro_rules! skip_if_no_network {
    () => {{
        if ::std::env::var($crate::sandbox_network_env_var()).is_ok() {
            println!(
                "Skipping test because it cannot execute when network is disabled in a Codex sandbox."
            );
            return;
        }
    }};
    ($return_value:expr $(,)?) => {{
        if ::std::env::var($crate::sandbox_network_env_var()).is_ok() {
            println!(
                "Skipping test because it cannot execute when network is disabled in a Codex sandbox."
            );
            return $return_value;
        }
    }};
}

#[macro_export]
macro_rules! codex_linux_sandbox_exe_or_skip {
    () => {{
        #[cfg(target_os = "linux")]
        {
            match $crate::find_codex_linux_sandbox_exe() {
                Ok(path) => Some(path),
                Err(err) => {
                    eprintln!("codex-linux-sandbox binary not available, skipping test: {err}");
                    return;
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }};
    ($return_value:expr $(,)?) => {{
        #[cfg(target_os = "linux")]
        {
            match $crate::find_codex_linux_sandbox_exe() {
                Ok(path) => Some(path),
                Err(err) => {
                    eprintln!("codex-linux-sandbox binary not available, skipping test: {err}");
                    return $return_value;
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }};
}
