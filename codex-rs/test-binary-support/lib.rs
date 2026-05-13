use std::path::Path;
use std::path::PathBuf;

use codex_arg0::Arg0DispatchPaths;
use codex_arg0::Arg0PathEntryGuard;
use codex_arg0::arg0_dispatch;
use tempfile::TempDir;

pub struct TestBinaryDispatchGuard {
    _codex_home: TempDir,
    arg0: Arg0PathEntryGuard,
    _previous_codex_home: Option<std::ffi::OsString>,
}

impl TestBinaryDispatchGuard {
    pub fn paths(&self) -> &Arg0DispatchPaths {
        self.arg0.paths()
    }
}

pub enum TestBinaryDispatchMode {
    DispatchArg0Only,
    Skip,
    InstallAliases,
}

pub fn configure_test_binary_dispatch<F>(
    codex_home_prefix: &str,
    classify: F,
) -> Option<TestBinaryDispatchGuard>
where
    F: FnOnce(&str, Option<&str>) -> TestBinaryDispatchMode,
{
    let mut args = std::env::args_os();
    let argv0 = args.next().unwrap_or_default();
    let exe_name = Path::new(&argv0)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let argv1 = args.next();
    match classify(exe_name, argv1.as_deref().and_then(|arg| arg.to_str())) {
        TestBinaryDispatchMode::DispatchArg0Only => {
            let _ = arg0_dispatch();
            None
        }
        TestBinaryDispatchMode::Skip => None,
        TestBinaryDispatchMode::InstallAliases => {
            let codex_home_parent = match test_codex_home_parent() {
                Ok(path) => path,
                Err(error) => panic!("failed to resolve test CODEX_HOME parent: {error}"),
            };
            let codex_home = match tempfile::Builder::new()
                .prefix(codex_home_prefix)
                .tempdir_in(codex_home_parent)
            {
                Ok(codex_home) => codex_home,
                Err(error) => panic!("failed to create test CODEX_HOME: {error}"),
            };
            let previous_codex_home = std::env::var_os("CODEX_HOME");
            // Safety: this runs from a test ctor before test threads begin.
            unsafe {
                std::env::set_var("CODEX_HOME", codex_home.path());
            }

            let arg0 = match arg0_dispatch() {
                Some(arg0) => arg0,
                None => panic!("failed to configure arg0 dispatch aliases for test binary"),
            };
            match previous_codex_home.as_ref() {
                Some(value) => unsafe {
                    std::env::set_var("CODEX_HOME", value);
                },
                None => unsafe {
                    std::env::remove_var("CODEX_HOME");
                },
            }

            Some(TestBinaryDispatchGuard {
                _codex_home: codex_home,
                arg0,
                _previous_codex_home: previous_codex_home,
            })
        }
    }
}

fn test_codex_home_parent() -> std::io::Result<PathBuf> {
    if let Some(target_tmpdir) = std::env::var_os("CARGO_TARGET_TMPDIR") {
        let path = PathBuf::from(target_tmpdir);
        std::fs::create_dir_all(&path)?;
        return Ok(path);
    }

    let exe = std::env::current_exe()?;
    let exe_dir = exe.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "test executable path has no parent directory",
        )
    })?;
    let path = exe_dir.join("test-codex-home");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}
