#![expect(clippy::expect_used)]

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

pub mod protocol_fixtures {
    pub use codex_test_support_context_fixtures::protocol_fixtures::*;
}

pub mod responses {
    pub use codex_test_support_responses::responses::*;
}

pub use codex_test_support_responses::streaming_sse;

pub mod compact_fixtures;
pub mod process {
    pub use codex_test_support_lightweight::process::*;
}
pub mod tracing {
    pub use codex_test_support_lightweight::tracing::*;
}
#[path = "../../../core/tests/common/apps_test_server.rs"]
pub mod apps_test_server;
#[path = "../../../core/tests/common/runtime_harness.rs"]
pub mod runtime_harness;
#[path = "../../../core/tests/common/test_codex.rs"]
pub mod test_codex;
#[path = "../../../core/tests/common/test_codex_exec.rs"]
pub mod test_codex_exec;
#[path = "../../../core/tests/common/zsh_fork.rs"]
pub mod zsh_fork;
pub mod hooks;

pub use protocol_fixtures::RemoteEnvConfig;
pub use protocol_fixtures::assert_regex_match;
pub use protocol_fixtures::fetch_dotslash_file;
pub use protocol_fixtures::get_remote_test_env;
pub use protocol_fixtures::load_sse_fixture_with_id_from_str;
pub use protocol_fixtures::stdio_server_bin;
#[cfg(target_os = "linux")]
pub use runtime_harness::find_codex_linux_sandbox_exe;
pub use runtime_harness::format_with_current_shell;
pub use runtime_harness::format_with_current_shell_display;
pub use runtime_harness::format_with_current_shell_display_non_login;
pub use runtime_harness::format_with_current_shell_non_login;
pub use runtime_harness::load_default_config_for_test;
pub use runtime_harness::load_default_config_for_test_with_cloud_requirements;
pub use runtime_harness::managed_network_requirements_loader;
pub use runtime_harness::sandbox_env_var;
pub use runtime_harness::sandbox_network_env_var;
pub use runtime_harness::submit_thread_settings;
pub use runtime_harness::wait_for_event;
pub use runtime_harness::wait_for_event_match;
pub use runtime_harness::wait_for_event_with_timeout;

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
            eprintln!(
                "Skipping test because it cannot execute when network is disabled in a Codex sandbox."
            );
            return;
        }
    }};
    ($return_value:expr $(,)?) => {{
        if ::std::env::var($crate::sandbox_network_env_var()).is_ok() {
            eprintln!(
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
