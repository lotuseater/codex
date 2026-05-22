pub mod env;
pub mod fs_wait;
pub mod paths;
pub mod process;
#[cfg(feature = "tracing")]
pub mod tracing;

pub use env::remote_env_env_var;
pub use paths::TempDirExt;
pub use paths::test_absolute_path;
pub use paths::test_absolute_path_with_windows;
pub use paths::test_path_buf;
pub use paths::test_path_buf_with_windows;
pub use paths::test_tmp_path;
pub use paths::test_tmp_path_buf;

pub use codex_utils_absolute_path::test_support::PathBufExt;
pub use codex_utils_absolute_path::test_support::PathExt;

#[macro_export]
macro_rules! skip_if_remote {
    ($reason:expr $(,)?) => {{
        if ::std::env::var_os($crate::remote_env_env_var()).is_some() {
            eprintln!(
                "Skipping test under {}: {}",
                $crate::remote_env_env_var(),
                $reason
            );
            return;
        }
    }};
    ($return_value:expr, $reason:expr $(,)?) => {{
        if ::std::env::var_os($crate::remote_env_env_var()).is_some() {
            eprintln!(
                "Skipping test under {}: {}",
                $crate::remote_env_env_var(),
                $reason
            );
            return $return_value;
        }
    }};
}

#[macro_export]
macro_rules! skip_if_windows {
    ($return_value:expr $(,)?) => {{
        if cfg!(target_os = "windows") {
            println!("Skipping test because it cannot execute on Windows.");
            return $return_value;
        }
    }};
}
