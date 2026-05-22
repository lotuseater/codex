# SOLID Refactor Wave 20 Realtime/State/Review Split Worker Handoff

Classification: root-wiring-needed

## Summary

- Added the missing `realtime_conversation_startup_context` test binary wiring while keeping the existing websocket/realtime support under `suite/realtime_conversation/`.
- Split the state conversation aggregate wrapper into focused binaries:
  - `state_conversation_request_compression`
  - `state_conversation_turn_state`
  - `state_conversation_user_notification`
- Split review history coverage into focused binaries:
  - `review_history_parent_history_isolation`
  - `review_history_parent_session_surface`
- Converted the review-history suite functions into shared `pub(crate)` async routines called by the focused wrappers.
- Removed the stale aggregate wrapper files `state_conversation.rs` and `review_history.rs`.

## Root Wiring Needed

`codex-rs/core/Cargo.toml` already has a large unrelated working-tree diff from other split work, so I did not stage or commit this worker's changes. Root should reconcile and stage only the intended hunks across the active split workers.

## Files Touched

- `codex-rs/core/Cargo.toml`
- `codex-rs/core/tests/realtime_conversation_startup_context.rs`
- `codex-rs/core/tests/review_history_parent_history_isolation.rs`
- `codex-rs/core/tests/review_history_parent_session_surface.rs`
- `codex-rs/core/tests/state_conversation_request_compression.rs`
- `codex-rs/core/tests/state_conversation_turn_state.rs`
- `codex-rs/core/tests/state_conversation_user_notification.rs`
- `codex-rs/core/tests/suite/review_history.rs`

## Verification

Allowed verification only. No Cargo/Rust builds, formatters, schema generation, Bazel, release builds, deploy, or activation were run.

Passed:

```powershell
git diff --check -- codex-rs/core/Cargo.toml codex-rs/core/tests/realtime_conversation*.rs codex-rs/core/tests/state_conversation*.rs codex-rs/core/tests/review_history*.rs codex-rs/core/tests/suite/realtime_conversation.rs codex-rs/core/tests/suite/state_conversation.rs codex-rs/core/tests/suite/review_history.rs .codex/workflow/agents/solid_refactor_wave20_realtime_state_review_split_worker.handoff.md
```

The literal allowed `rg` command failed on Windows because `rg` received unmatched wildcard path arguments literally. Equivalent verification passed after PowerShell-expanded path lists:

```powershell
$paths = @('codex-rs/core/Cargo.toml') + (Get-ChildItem -Path codex-rs/core/tests -Filter 'realtime_conversation*.rs').FullName + (Get-ChildItem -Path codex-rs/core/tests -Filter 'state_conversation*.rs').FullName + (Get-ChildItem -Path codex-rs/core/tests -Filter 'review_history*.rs').FullName + @('codex-rs/core/tests/suite'); rg -n "realtime_conversation|state_conversation|review_history" @paths
```
