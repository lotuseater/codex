# SOLID Refactor Wave 20 Compact/Resume/Streaming Split Worker Handoff

## Status

Implemented focused wrapper splits for the compact/resume/fork and stream-error suites within this worker's ownership.

## Changed Files

- `codex-rs/core/tests/compact_resume.rs`
  - Narrowed to only `suite/resume.rs`.
- `codex-rs/core/tests/compact_resume_fork_thread.rs`
  - New focused wrapper for `suite/fork_thread.rs`.
- `codex-rs/core/tests/compact_resume_warning.rs`
  - New focused wrapper for `suite/resume_warning.rs`.
- `codex-rs/core/tests/stream_error_allows_next_turn.rs`
  - New focused wrapper for `suite/stream_error_allows_next_turn.rs`.
- `codex-rs/core/Cargo.toml`
  - Added `[[test]]` entries for:
    - `compact_resume_fork_thread`
    - `compact_resume_warning`
    - `stream_error_allows_next_turn`

## Preserved / Not Touched

- Did not edit shared suite harness behavior.
- Did not edit `codex-rs/core/tests/client_stream.rs` because it is outside this worker's ownership.
- Did not edit the existing `window_headers` wrapper; `codex-rs/core/tests/window_headers.rs` already wraps `suite/window_headers.rs`, so `compact_resume.rs` no longer includes that suite.

## root-wiring-needed

- `codex-rs/core/tests/client_stream.rs` still includes `suite/stream_error_allows_next_turn.rs`. Root should remove that module from `client_stream.rs` in the worker or integration pass that owns `client_stream.rs`; otherwise the stream-error continuation tests will be present in both `client_stream` and `stream_error_allows_next_turn` binaries.
- `codex-rs/core/Cargo.toml` was already heavily modified by concurrent split workers before this worker's edit. Keep manifest staging focused during integration.
- No commit was created because `codex-rs/core/Cargo.toml` and multiple owned suite files already had unrelated concurrent edits, and several wrapper files were already untracked before/around this worker's slice.

## Verification

- Passed:
  - `rg -n "compact_|resume|fork_thread|stream_error" ...` using PowerShell-expanded file paths for `codex-rs/core/tests/compact*.rs` and `codex-rs/core/tests/stream_error*.rs` because native `rg` on this Windows shell does not expand those wildcards.
  - `git diff --check -- codex-rs/core/Cargo.toml codex-rs/core/tests/compact*.rs codex-rs/core/tests/stream_error*.rs codex-rs/core/tests/suite/compact*.rs codex-rs/core/tests/suite/fork_thread.rs codex-rs/core/tests/suite/resume.rs codex-rs/core/tests/suite/stream_error*.rs .codex/workflow/agents/solid_refactor_wave20_compact_resume_streaming_split_worker.handoff.md`

No Cargo/Rust builds, formatters, schema generation, Bazel, lock refresh, release builds, deploy, or activation were run by this worker.
