# core_test_split_common_support_scout Handoff

Status: complete read-only scout on 2026-05-20.

Scope honored:
- Inspected the requested support and suite files.
- Did not edit source files.
- Did not run Cargo, Just, formatters, Git staging/commits, or broad build lanes.
- Only this handoff file was updated.

## Sources Inspected

- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/core/tests/common/lib.rs`
- `codex-rs/core/tests/common/Cargo.toml`
- `codex-rs/core/tests/all.rs`
- `codex-rs/core/tests/suite_bootstrap.rs`
- `codex-rs/core/tests/suite/mod.rs`
- `codex-rs/core/tests/suite/compact.rs`
- `codex-rs/core/tests/suite/compact_resume_fork.rs`
- `codex-rs/core/tests/suite/window_headers.rs`
- `codex-rs/core/tests/responses_headers.rs`
- `codex-rs/test-binary-support/lib.rs`
- Targeted `rg` scans for `super::`, `core_test_support::`, `suite_bootstrap`, and test binary dispatch symbols.

## Current Support APIs Already In `core_test_support`

`core_test_support` is the package at `codex-rs/core/tests/common` and is already a dedicated test-support crate, not production `codex-core` API.

Public modules exported from `codex-rs/core/tests/common/lib.rs`:

- `apps_test_server`
- `context_snapshot`
- `hooks`
- `process`
- `responses`
- `streaming_sse`
- `test_codex`
- `test_codex_exec`
- `tracing`
- `zsh_fork`

Top-level support already exported or re-exported:

- Path helpers and traits: `PathBufExt`, `PathExt`, `TempDirExt`, `test_path_buf_with_windows`, `test_path_buf`, `test_absolute_path_with_windows`, `test_absolute_path`, `test_tmp_path`, `test_tmp_path_buf`.
- Assertions/helpers: `assert_regex_match`, `fetch_dotslash_file`, `stdio_server_bin`, `fs_wait`.
- Config/test harness helpers: `load_default_config_for_test`, `load_default_config_for_test_with_cloud_requirements`, `managed_network_requirements_loader`, `submit_thread_settings`.
- Event helpers: `wait_for_event`, `wait_for_event_match`, `wait_for_event_with_timeout`.
- Environment helpers: `sandbox_env_var`, `sandbox_network_env_var`, `remote_env_env_var`, `RemoteEnvConfig`, `get_remote_test_env`.
- Skip/location macros: `skip_if_sandbox!`, `skip_if_no_network!`, `skip_if_remote!`, `codex_linux_sandbox_exe_or_skip!`, `skip_if_windows!`.
- Process-wide test ctors: deterministic unified-exec process IDs, arg0 dispatch setup through `codex_arg0::arg0_dispatch`, and `INSTA_WORKSPACE_ROOT` setup for snapshots.

Observed suite usage of `core_test_support` is concentrated in these APIs:

- `responses` is the dominant dependency, used for SSE fixtures, mock server setup, response mocks, websocket/mock response helpers, and request assertions.
- `test_codex` is the second largest dependency, especially `test_codex`, `TestCodexBuilder`, `TestCodex`, `TestCodexHarness`, `ApplyPatchModelOutput`, `test_env`, and turn permission helpers.
- Other shared surfaces currently used by suite modules include `apps_test_server`, `context_snapshot`, `hooks`, `process`, `streaming_sse`, `tracing::install_test_tracing`, `zsh_fork`, `wait_for_event*`, config loaders, path traits, skip macros, `assert_regex_match`, `stdio_server_bin`, `fs_wait`, and remote/network helpers.

## Current Suite Harness State

`codex-rs/core/tests/all.rs` is the single aggregated integration-test binary root:

- It re-exports `codex_protocol::error`.
- It declares `mod suite_bootstrap;`.
- It declares `mod suite;`.

`codex-rs/core/tests/suite/mod.rs` is currently only module aggregation. The test-binary dispatch ctor is no longer in `suite/mod.rs` in this checkout.

`codex-rs/core/tests/suite_bootstrap.rs` currently owns the suite-level test binary alias/dispatch setup:

- Defines `CODEX_ALIASES_TEMP_DIR`.
- Calls `configure_test_binary_dispatch("codex-core-tests", ...)`.
- Dispatches arg0-only for `CODEX_CORE_APPLY_PATCH_ARG1`, `CODEX_FS_HELPER_ARG1`, and the `CODEX_LINUX_SANDBOX_ARG0` executable name.
- Otherwise installs aliases.

This means future independent test binaries must either include the same bootstrap module or get that bootstrap behavior through `core_test_support`. Because `core_test_support` already has test ctors that run when linked, moving this bootstrap behavior there is the cleaner reusable support-layer shape.

## Exact `super::` Dependencies Blocking Split

The targeted scan found only these `super::` dependencies under `codex-rs/core/tests/suite`:

- `codex-rs/core/tests/suite/compact_resume_fork.rs`
  - `use super::compact::COMPACT_WARNING_MESSAGE;`
  - `use super::compact::FIRST_REPLY;`
  - `use super::compact::SUMMARY_TEXT;`
- `codex-rs/core/tests/suite/window_headers.rs`
  - `use super::compact::COMPACT_WARNING_MESSAGE;`

The source constants are currently in `codex-rs/core/tests/suite/compact.rs`:

- `pub(super) const FIRST_REPLY: &str = "FIRST_REPLY";`
- `pub(super) const SUMMARY_TEXT: &str = "SUMMARY_ONLY_CONTEXT";`
- `pub(super) const COMPACT_WARNING_MESSAGE: &str = "Heads up: Long threads and multiple compactions can cause the model to be less accurate. Start a new thread when possible to keep threads small and targeted.";`

No `crate::error` dependencies were found in suite modules during the same blocker scan.

## Recommended Support Move / Re-export Plan

Minimal reusable support-layer change:

1. Add a small fixture module under `codex-rs/core/tests/common`, for example `compact_fixtures`.
2. Move or duplicate-as-owned-test-fixtures the three compact constants into that support module:
   - `COMPACT_WARNING_MESSAGE`
   - `FIRST_REPLY`
   - `SUMMARY_TEXT`
3. Export the module from `core_test_support`.
4. Update split candidates to import:
   - `use core_test_support::compact_fixtures::{COMPACT_WARNING_MESSAGE, FIRST_REPLY, SUMMARY_TEXT};`
   - or only `COMPACT_WARNING_MESSAGE` where that is the sole dependency.
5. Leave `codex-rs/core/tests/suite/mod.rs` as aggregation only.

Test-binary bootstrap recommendation:

1. Move the reusable alias/dispatch bootstrap from `codex-rs/core/tests/suite_bootstrap.rs` into `core_test_support`, or expose it as a small `core_test_support` bootstrap module/ctor.
2. Keep the current behavior unchanged:
   - arg0-only for apply-patch
   - arg0-only for fs-helper
   - arg0-only for linux sandbox argv0
   - install aliases otherwise
3. After that, each future split integration binary only needs to link/import `core_test_support`; it should not need to remember `mod suite_bootstrap`.
4. If the bootstrap is moved into `core_test_support`, expect dependency ownership to move with it. `core_test_support` currently has `codex-exec-server` and `ctor`, but not all of `codex-apply-patch`, `codex-sandboxing`, or `codex-test-binary-support`.

## Path Ownership Warnings

- Keep this support work in `codex-rs/core/tests/common`; do not grow production `codex-core` for integration-test fixture sharing.
- Root should own manifest and lockfile changes if bootstrap movement adds/removes dependencies.
- Coordinate before touching `codex-rs/core/tests/all.rs`, `codex-rs/core/tests/suite_bootstrap.rs`, `codex-rs/core/tests/suite/mod.rs`, `codex-rs/core/Cargo.toml`, workspace dependency files, Bazel metadata, or lockfiles.
- `codex-rs/core/tests/responses_headers.rs` is already a separate integration test file using `core_test_support`; it currently does not declare `mod suite_bootstrap`. That is a useful canary when deciding whether bootstrap behavior should become automatic from `core_test_support`.
- Do not convert these helpers into compatibility re-exports from `codex-core`; the intended ownership boundary is test support.

## Verification Lane To Run After Implementation

Do not use broad debug-profile Cargo lanes on this checkout.

Recommended sequence after a support-layer implementation:

1. Run formatting after Rust edits from `codex-rs`: `just fmt`.
2. Run the smallest relevant release-profile lane with the repo script, scoped to `codex-core`, for example a focused filter through `scripts\test-local-codex-release.ps1 -Package codex-core -Filter <split-or-affected-test-filter>`.
3. Exercise at least one split candidate that used the former `super::compact` constants.
4. Exercise at least one test path that depends on first-party binary dispatch/aliases if bootstrap was moved.
5. Only after focused lanes pass should root consider broader release-profile `codex-core` coverage.
