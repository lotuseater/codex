//! Transitional access to core-only embedded app-server types.
//!
//! New TUI behavior should prefer the app-server protocol methods. This
//! module exists so clients can remove a direct `codex-core` dependency
//! while legacy startup/config paths are migrated to RPCs.

pub use codex_core::DEFAULT_AGENTS_MD_FILENAME;
pub use codex_core::LOCAL_AGENTS_MD_FILENAME;
pub use codex_core::McpManager;
pub use codex_core::check_execpolicy_for_warnings;
pub use codex_core::format_exec_policy_error_with_source;
pub use codex_core::grant_read_root_non_elevated;
pub use codex_core::web_search_detail;

pub mod config {
    pub use codex_core::config::*;

    pub mod edit {
        pub use codex_config::edit::*;
    }
}

pub mod connectors {
    pub use codex_core::connectors::*;
}

pub mod otel_init {
    pub use codex_core::otel_init::*;
}

pub mod personality_migration {
    pub use codex_core::personality_migration::*;
}

pub mod review_format {
    pub use codex_core::review_format::*;
}

pub mod review_prompts {
    pub use codex_core::review_prompts::*;
}

pub mod test_support {
    pub use codex_core::test_support::*;
}

pub mod util {
    pub use codex_core::util::*;
}

pub mod windows_sandbox {
    pub use codex_core::windows_sandbox::*;
}
