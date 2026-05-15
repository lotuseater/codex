/// The current Codex CLI version as embedded at compile time.
pub const CODEX_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Local fork marker shown in the interactive session header.
pub fn local_fork_version_label() -> String {
    std::env::var("WIZARD_CODEX_LOCAL_BUILD_STAMP")
        .ok()
        .or_else(|| std::env::var("CODEX_LOCAL_BUILD_STAMP").ok())
        .map(|stamp| stamp.trim().to_string())
        .filter(|stamp| !stamp.is_empty())
        .map(|stamp| format!("local build {stamp}"))
        .unwrap_or_else(|| "local source build".to_string())
}
