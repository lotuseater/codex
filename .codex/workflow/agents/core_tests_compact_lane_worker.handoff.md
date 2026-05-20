# core_tests_compact_lane_worker Handoff

Date: 2026-05-20
Status: compact/context/resume lane split implemented; release compile blocked by unrelated broader `codex-core` source errors before this test target compiles.

## Scope

Owned lane:

- `codex-rs/core/tests/compact.rs`
- `codex-rs/core/tests/common/compact_fixtures.rs`
- `codex-rs/core/tests/common/lib.rs`
- Compact/context/resume modules under `codex-rs/core/tests/suite/`
- Compact snapshots under `codex-rs/core/tests/suite/snapshots/`

## Changes

- Added `codex-rs/core/tests/compact.rs` as the standalone integration-test entry point for the compact/context/resume lane.
- Reused the existing suite files through `#[path = "suite/..."]` module declarations:
  - `compact`
  - `compact_remote`
  - `compact_remote_parity`
  - `compact_resume_fork`
  - `fork_thread`
  - `resume`
  - `resume_warning`
  - `window_headers`
- Moved the compact constants used across multiple modules into `core_test_support::compact_fixtures`:
  - `FIRST_REPLY`
  - `SUMMARY_TEXT`
  - `COMPACT_WARNING_MESSAGE`
- Updated compact modules to import those shared constants from `core_test_support::compact_fixtures` instead of depending on another suite module.
- Renamed compact snapshot files from the old aggregate test-binary prefix `all__suite__...` to the new direct compact integration-test prefix:
  - `compact__compact__...`
  - `compact__compact_remote__...`
  - `compact__compact_resume_fork__...`

## Verification

- `rg -n "super::|crate::suite" codex-rs/core/tests/suite/{compact.rs,compact_remote.rs,compact_remote_parity.rs,compact_resume_fork.rs,fork_thread.rs,resume.rs,resume_warning.rs,window_headers.rs}`
  - No matches in the compact lane.
- `just fmt`
  - Passed.
  - Output: `64 files left unchanged`, then Python SDK ruff fix/format completed.
- `cargo test -p codex-core --test compact --release --no-run`
  - Failed before compiling the compact integration target because the current broader working tree has unrelated `codex-core` library compile errors.
  - Fresh log: `logs/compact-test-no-run-20260521-002555.log`
  - Primary unrelated errors include:
    - `core/src/tools/handlers/view_image.rs`: `ToolExecutor<ToolInvocation>` associated type `Output` is not specified.
    - `core/src/tasks/regular.rs`: stale calls to `turn_extension_data`, `run_turn`, and `input_queue`.
    - `core/src/codex_delegate.rs`: `Op::UserInput` is constructed with a removed `thread_settings` field.
    - `core/src/thread_manager.rs`: `LocalThreadStore` is not in scope.

## Notes

- The compile blocker is outside this worker's owned compact test lane and overlaps the broader in-progress core refactor.
- `cargo test --release -p codex-core --test compact` should be the focused release verification command once `codex-thread-store-api` is green again.
