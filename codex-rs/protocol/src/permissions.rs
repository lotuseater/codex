// fork-local: runtime-permission types live in the extracted
// `codex_permission_types` crate and are re-exported here so existing
// `codex_protocol::permissions::*` consumers keep their import paths. Upstream
// defines these types inline in this module; the fork intentionally drops the
// inline copy to avoid duplicate definitions while preserving identical
// behavior through the crate (including the richer fork shape: the
// `FileSystemAccessMode::None` access mode and the per-profile writable-root
// modifications feature).
pub use codex_permission_types::FileSystemAccessMode;
pub use codex_permission_types::FileSystemPath;
pub use codex_permission_types::FileSystemPermissions;
pub use codex_permission_types::FileSystemSandboxEntry;
pub use codex_permission_types::FileSystemSandboxKind;
pub use codex_permission_types::FileSystemSandboxPolicy;
pub use codex_permission_types::FileSystemSpecialPath;
pub use codex_permission_types::NetworkSandboxPolicy;
pub use codex_permission_types::PROTECTED_METADATA_PATH_NAMES;
pub use codex_permission_types::ReadDenyMatcher;
pub use codex_permission_types::forbidden_agent_metadata_write;
pub use codex_permission_types::is_protected_metadata_directory_name;
pub use codex_permission_types::is_protected_metadata_name;
// fork-local: re-exported for the merged `protocol.rs`, which (taking upstream's
// monolithic `SandboxPolicy`/`WritableRoot` shape) still calls this helper and
// the upstream-added `project_roots_glob_pattern` through `crate::permissions`.
pub use codex_permission_types::default_read_only_subpaths_for_writable_root;
pub use codex_permission_types::project_roots_glob_pattern;
