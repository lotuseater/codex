# core_compile_session_thread_worker Handoff

## Status

Refactor slice complete, but not committed. The owned session/thread changes are
formatted and `git diff --check` is clean. Commit is blocked because the focused
release check fails in an upstream dependency before `codex-core` is checked.

## Files Changed

- `codex-rs/core/src/thread_manager.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/session.rs`
- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/session/context_budget.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/session/tests/guardian_tests.rs`
- `codex-rs/core/src/codex_delegate.rs`
- `codex-rs/core/src/state/service.rs`

`codex_delegate.rs` and `state/service.rs` were touched only to carry the
session/thread persistence API through the existing core spawn path without
dropping the injected live-thread factory or state DB handle.

## Compile Blockers Fixed

- Removed remaining concrete `LocalThreadStore` / `LocalThreadStoreConfig`
  references from the owned core session/thread path. Core now uses
  `codex_thread_store_api` recording fakes in session/thread tests instead of
  depending on the concrete local store crate.
- Completed `ThreadManagerState` construction in
  `ThreadManager::with_models_provider_for_tests` by supplying a matching
  `RecordingLiveThreadFactory`.
- Added `live_thread_factory` to `SessionServices` and populated it from
  `Session::new`, so delegated `CodexSpawnArgs` can preserve the parent
  persistence factory.
- Updated delegated and guardian spawn paths to pass both
  `live_thread_factory` and `state_db`.
- Updated direct `Session::new` test callsites for the new
  `(thread_store, live_thread_factory, state_db, trace, attestation)` argument
  sequence.

## Cross-Lane Blockers Left For Root

- Verification is blocked before `codex-core` by `codex-otel`:
  `error[E0004]: non-exhaustive patterns: &ResponseEvent::Incomplete { .. } not covered`.
  Log: `logs/codex-core-lib-release-check-20260521-021630.log`.
- The broader worktree still has many unrelated dirty files from other lanes.
  This worker did not revert or stage them.

## Verification

- Ran `just fmt` from `codex-rs`.
- Ran `git diff --check -- codex-rs/core/src/codex_delegate.rs codex-rs/core/src/session codex-rs/core/src/state/service.rs codex-rs/core/src/thread_manager.rs`: passed.
- Attempted the allowed focused release check:
  `cargo check -p codex-core --release --lib`
  from `codex-rs`, logged to
  `logs/codex-core-lib-release-check-20260521-021630.log`. It failed in
  `codex-otel` before reaching this lane's `codex-core` compile path.

## Commit

No commit. Exact blocker: focused verification did not reach `codex-core`
because of the out-of-lane `codex-otel` non-exhaustive `ResponseEvent::Incomplete`
compile error.
