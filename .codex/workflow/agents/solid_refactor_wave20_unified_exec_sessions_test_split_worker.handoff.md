# SOLID Refactor Wave 20 Unified Exec Sessions Test Split Worker

Classification: root-wiring-needed

## Scope

Split the unified-exec session tests into narrower Cargo test binaries while keeping
`suite/unified_exec_support.rs` as the shared support module.

## Changes

- Kept `tests/exec_unified_sessions.rs` as the existing session lifecycle binary.
  - `suite/unified_exec_sessions.rs` now contains lifecycle and interrupt coverage only.
- Added `tests/exec_unified_sessions_terminal.rs`.
  - Covers terminal interaction and `write_stdin` output-token policy behavior.
- Added `tests/exec_unified_sessions_modes.rs`.
  - Covers default pipe behavior and explicit TTY enablement.
- Added `tests/exec_unified_sessions_reuse.rs`.
  - Covers session reuse, lagged output, timeout follow-up polling, and pruning exited sessions.
- Added matching `[[test]]` entries in `codex-rs/core/Cargo.toml`.
- Left `suite/unified_exec_support.rs` unchanged.

## Root Wiring Needed

`codex-rs/core/Cargo.toml` already had broad pre-existing dirty manifest edits before this
worker touched the unified-exec session entries. The new entries are local to this split, but root
should own final manifest staging/commit with the other concurrent wiring.

## Verification

- Passed PowerShell-expanded equivalent of:
  `rg -n "unified_exec" codex-rs/core/Cargo.toml codex-rs/core/tests/exec_unified_sessions*.rs codex-rs/core/tests/suite/unified_exec*.rs`
  - Direct literal `*.rs` path arguments are invalid for Windows `rg`, so the file list was expanded with `Get-ChildItem`.
- Passed:
  `git diff --check -- codex-rs/core/Cargo.toml codex-rs/core/tests/exec_unified_sessions*.rs codex-rs/core/tests/suite/unified_exec*.rs .codex/workflow/agents/solid_refactor_wave20_unified_exec_sessions_test_split_worker.handoff.md`
  - It printed the existing CRLF warning for `codex-rs/core/Cargo.toml`.

## Commit

Not committed by this worker because focused staging is not clean while `Cargo.toml` contains
unrelated concurrent dirty hunks.
