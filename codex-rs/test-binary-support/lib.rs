use std::path::Path;
use std::path::PathBuf;

use codex_arg0::Arg0DispatchPaths;
use codex_arg0::Arg0PathEntryGuard;
use codex_arg0::arg0_dispatch;
use tempfile::TempDir;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestBinaryInvocation {
    exe_name: String,
    argv1: Option<String>,
}

impl TestBinaryInvocation {
    pub fn new(exe_name: impl Into<String>) -> Self {
        Self {
            exe_name: exe_name.into(),
            argv1: None,
        }
    }

    pub fn with_argv1(mut self, argv1: impl Into<String>) -> Self {
        self.argv1 = Some(argv1.into());
        self
    }

    pub fn exe_name(&self) -> &str {
        &self.exe_name
    }

    pub fn argv1(&self) -> Option<&str> {
        self.argv1.as_deref()
    }

    fn current() -> Self {
        let mut args = std::env::args_os();
        let argv0 = args.next().unwrap_or_default();
        let exe_name = Path::new(&argv0)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_owned();
        let argv1 = args
            .next()
            .and_then(|arg| arg.to_str().map(ToOwned::to_owned));

        Self { exe_name, argv1 }
    }
}

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

/// Classifies how a test binary invocation should be handled before tests run.
///
/// Implementations should only inspect the invocation shape and return the
/// desired dispatch mode. They should not construct runtime sessions or depend
/// on `codex-core`, so small owner-crate test binaries can reuse the same
/// startup policy without pulling in core runtime behavior.
pub trait TestBinaryDispatchClassifier {
    fn classify(&self, invocation: &TestBinaryInvocation) -> TestBinaryDispatchMode;
}

impl<F> TestBinaryDispatchClassifier for F
where
    F: Fn(&TestBinaryInvocation) -> TestBinaryDispatchMode,
{
    fn classify(&self, invocation: &TestBinaryInvocation) -> TestBinaryDispatchMode {
        self(invocation)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestBinaryDispatchSpec<'a> {
    arg1_dispatches: &'a [&'a str],
    exe_name_dispatches: &'a [&'a str],
}

impl<'a> TestBinaryDispatchSpec<'a> {
    pub const fn new(
        arg1_dispatches: &'a [&'a str],
        exe_name_dispatches: &'a [&'a str],
    ) -> Self {
        Self {
            arg1_dispatches,
            exe_name_dispatches,
        }
    }

    pub fn dispatches_arg0(&self, invocation: &TestBinaryInvocation) -> bool {
        if invocation
            .argv1()
            .is_some_and(|argv1| self.arg1_dispatches.contains(&argv1))
        {
            return true;
        }

        self.exe_name_dispatches.contains(&invocation.exe_name())
    }
}

impl TestBinaryDispatchClassifier for TestBinaryDispatchSpec<'_> {
    fn classify(&self, invocation: &TestBinaryInvocation) -> TestBinaryDispatchMode {
        if self.dispatches_arg0(invocation) {
            TestBinaryDispatchMode::DispatchArg0Only
        } else {
            TestBinaryDispatchMode::InstallAliases
        }
    }
}

pub fn configure_test_binary_dispatch<F>(
    codex_home_prefix: &str,
    classify: F,
) -> Option<TestBinaryDispatchGuard>
where
    F: FnOnce(&str, Option<&str>) -> TestBinaryDispatchMode,
{
    let invocation = TestBinaryInvocation::current();
    configure_test_binary_dispatch_for_mode(
        codex_home_prefix,
        classify(invocation.exe_name(), invocation.argv1()),
    )
}

pub fn configure_test_binary_dispatch_with_classifier<C>(
    codex_home_prefix: &str,
    classifier: C,
) -> Option<TestBinaryDispatchGuard>
where
    C: TestBinaryDispatchClassifier,
{
    let invocation = TestBinaryInvocation::current();
    configure_test_binary_dispatch_for_mode(codex_home_prefix, classifier.classify(&invocation))
}

fn configure_test_binary_dispatch_for_mode(
    codex_home_prefix: &str,
    mode: TestBinaryDispatchMode,
) -> Option<TestBinaryDispatchGuard> {
    match mode {
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
