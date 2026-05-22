//! Helpers that intentionally depend on `codex-core` because they build real
//! Codex configs, mutate `CodexThread`, or wait on runtime events.

use codex_arg0::Arg0PathEntryGuard;
use codex_config::CloudRequirementsLoader;
use codex_config::ConfigRequirementsToml;
use codex_config::LoaderOverrides;
use codex_config::NetworkRequirementsToml;
use codex_core::CodexThread;
use codex_core::CodexThreadSettingsOverrides;
use codex_core::config::Config;
use codex_core::config::ConfigBuilder;
use codex_core::config::ConfigOverrides;
#[cfg(target_os = "linux")]
use codex_utils_cargo_bin::CargoBinError;
use ctor::ctor;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use std::sync::OnceLock;
use tempfile::TempDir;

static TEST_ARG0_PATH_ENTRY: OnceLock<Option<Arg0PathEntryGuard>> = OnceLock::new();

#[ctor(unsafe)]
fn enable_deterministic_unified_exec_process_ids_for_tests() {
    configure_codex_core_runtime_test_mode();
}

#[ctor(unsafe)]
fn configure_arg0_dispatch_for_test_binaries() {
    let _ = TEST_ARG0_PATH_ENTRY.get_or_init(codex_arg0::arg0_dispatch);
}

#[ctor(unsafe)]
fn configure_insta_workspace_root_for_snapshot_tests() {
    if std::env::var_os("INSTA_WORKSPACE_ROOT").is_some() {
        return;
    }

    let workspace_root = codex_utils_cargo_bin::repo_root()
        .ok()
        .map(|root| root.join("codex-rs"));

    if let Some(workspace_root) = workspace_root
        && let Ok(workspace_root) = workspace_root.canonicalize()
    {
        // Safety: this ctor runs at process startup before test threads begin.
        unsafe {
            std::env::set_var("INSTA_WORKSPACE_ROOT", workspace_root);
        }
    }
}

/// Runtime harnesses instantiate `CodexThread`; these toggles keep process-id
/// behavior deterministic across those integration tests.
fn configure_codex_core_runtime_test_mode() {
    codex_core::test_support::set_thread_manager_test_mode(/*enabled*/ true);
    codex_core::test_support::set_deterministic_process_ids(/*enabled*/ true);
}

pub fn sandbox_env_var() -> &'static str {
    codex_core::spawn::CODEX_SANDBOX_ENV_VAR
}

pub fn sandbox_network_env_var() -> &'static str {
    codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR
}

/// Returns a default `Config` whose on-disk state is confined to the provided
/// temporary directory. Using a per-test directory keeps tests hermetic and
/// avoids clobbering a developer's real `~/.codex`.
pub async fn load_default_config_for_test(codex_home: &TempDir) -> Config {
    load_default_config_for_test_with_cloud_requirements(
        codex_home,
        CloudRequirementsLoader::default(),
    )
    .await
}

/// Returns a default `Config` with test-provided cloud requirements applied
/// during config construction.
pub async fn load_default_config_for_test_with_cloud_requirements(
    codex_home: &TempDir,
    cloud_requirements: CloudRequirementsLoader,
) -> Config {
    ConfigBuilder::default()
        .loader_overrides(LoaderOverrides::without_managed_config_for_tests())
        .codex_home(codex_home.path().to_path_buf())
        .harness_overrides(default_test_overrides())
        .cloud_requirements(cloud_requirements)
        .build()
        .await
        .expect("defaults for test should always succeed")
}

pub fn managed_network_requirements_loader() -> CloudRequirementsLoader {
    CloudRequirementsLoader::new(async {
        Ok(Some(ConfigRequirementsToml {
            network: Some(NetworkRequirementsToml {
                enabled: Some(true),
                allow_local_binding: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }))
    })
}

#[cfg(target_os = "linux")]
fn default_test_overrides() -> ConfigOverrides {
    ConfigOverrides {
        codex_linux_sandbox_exe: Some(
            find_codex_linux_sandbox_exe().expect("should find binary for codex-linux-sandbox"),
        ),
        ..ConfigOverrides::default()
    }
}

#[cfg(not(target_os = "linux"))]
fn default_test_overrides() -> ConfigOverrides {
    ConfigOverrides::default()
}

#[cfg(target_os = "linux")]
pub fn find_codex_linux_sandbox_exe() -> Result<PathBuf, CargoBinError> {
    if let Some(path) = TEST_ARG0_PATH_ENTRY
        .get()
        .and_then(Option::as_ref)
        .and_then(|path_entry| path_entry.paths().codex_linux_sandbox_exe.clone())
    {
        return Ok(path);
    }

    if let Ok(path) = std::env::current_exe() {
        return Ok(path);
    }

    codex_utils_cargo_bin::cargo_bin("codex-linux-sandbox")
}

pub async fn wait_for_event<F>(
    codex: &CodexThread,
    predicate: F,
) -> codex_protocol::protocol::EventMsg
where
    F: FnMut(&codex_protocol::protocol::EventMsg) -> bool,
{
    use tokio::time::Duration;
    wait_for_event_with_timeout(codex, predicate, Duration::from_secs(1)).await
}

pub async fn submit_thread_settings(
    codex: &CodexThread,
    thread_settings: CodexThreadSettingsOverrides,
) -> anyhow::Result<()> {
    codex
        .apply_thread_settings_overrides(thread_settings)
        .await?;
    Ok(())
}

pub async fn wait_for_event_match<T, F>(codex: &CodexThread, matcher: F) -> T
where
    F: Fn(&codex_protocol::protocol::EventMsg) -> Option<T>,
{
    let ev = wait_for_event(codex, |ev| matcher(ev).is_some()).await;
    matcher(&ev).expect("EventMsg should match matcher predicate")
}

pub async fn wait_for_event_with_timeout<F>(
    codex: &CodexThread,
    mut predicate: F,
    wait_time: tokio::time::Duration,
) -> codex_protocol::protocol::EventMsg
where
    F: FnMut(&codex_protocol::protocol::EventMsg) -> bool,
{
    use tokio::time::Duration;
    use tokio::time::timeout;
    loop {
        // Allow a bit more time to accommodate async startup work (e.g. config IO, tool discovery)
        let ev = timeout(wait_time.max(Duration::from_secs(10)), codex.next_event())
            .await
            .expect("timeout waiting for event")
            .expect("stream ended unexpectedly");
        if predicate(&ev.msg) {
            return ev.msg;
        }
    }
}

pub fn format_with_current_shell(command: &str) -> Vec<String> {
    codex_core::shell::default_user_shell().derive_exec_args(command, /*use_login_shell*/ true)
}

pub fn format_with_current_shell_display(command: &str) -> String {
    let args = format_with_current_shell(command);
    shlex::try_join(args.iter().map(String::as_str)).expect("serialize current shell command")
}

pub fn format_with_current_shell_non_login(command: &str) -> Vec<String> {
    codex_core::shell::default_user_shell()
        .derive_exec_args(command, /*use_login_shell*/ false)
}

pub fn format_with_current_shell_display_non_login(command: &str) -> String {
    let args = format_with_current_shell_non_login(command);
    shlex::try_join(args.iter().map(String::as_str))
        .expect("serialize current shell command without login")
}
