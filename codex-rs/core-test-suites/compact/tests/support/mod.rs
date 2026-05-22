use codex_apply_patch::CODEX_CORE_APPLY_PATCH_ARG1;
use codex_exec_server::CODEX_FS_HELPER_ARG1;
use codex_sandboxing::landlock::CODEX_LINUX_SANDBOX_ARG0;
use codex_test_binary_support::TestBinaryDispatchGuard;
use codex_test_binary_support::TestBinaryDispatchSpec;
use codex_test_binary_support::configure_test_binary_dispatch_with_classifier;
use ctor::ctor;

const CORE_TEST_ARG1_DISPATCHES: &[&str] = &[CODEX_CORE_APPLY_PATCH_ARG1, CODEX_FS_HELPER_ARG1];
const CORE_TEST_EXE_NAME_DISPATCHES: &[&str] = &[CODEX_LINUX_SANDBOX_ARG0];
const CORE_TEST_BINARY_DISPATCH: TestBinaryDispatchSpec<'static> =
    TestBinaryDispatchSpec::new(CORE_TEST_ARG1_DISPATCHES, CORE_TEST_EXE_NAME_DISPATCHES);

// This code runs before any other tests are run. It allows the test binary to
// behave like codex and dispatch to apply_patch and codex-linux-sandbox based
// on arg0. NOTE: this does not work on ARM.
#[ctor(unsafe)]
pub static CODEX_ALIASES_TEMP_DIR: Option<TestBinaryDispatchGuard> = {
    configure_test_binary_dispatch_with_classifier("codex-core-compact-tests", CORE_TEST_BINARY_DISPATCH)
};
