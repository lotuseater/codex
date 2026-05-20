# thread_store_api_recording_repair_worker Handoff

Status: completed and verified.

Date: 2026-05-20

## Owned Paths

- `codex-rs/thread/thread-store-api/src/recording.rs`
- `.codex/workflow/agents/thread_store_api_recording_repair_worker.handoff.md`

Inspected but did not edit:

- `codex-rs/thread/thread-store-api/src/store.rs`
- `codex-rs/thread/thread-store-api/src/live_thread.rs`

## Changes

- Updated `RecordingThreadStore` for the current `ThreadStore` trait by adding
  `persist_thread` and `flush_thread` implementations plus matching call
  counters.
- Updated `RecordingLiveThread` for the current `LiveThreadHandle` trait:
  `append_items` now accepts a `&[RolloutItem]`, `persist` delegates to
  `ThreadStore::persist_thread`, and `flush` delegates to
  `ThreadStore::flush_thread`.
- Fixed stale local API mismatches:
  - renamed the recording call counter from `list_turn_items` to `list_items`;
  - removed invalid `Option::flatten` calls from `model` and
    `reasoning_effort`;
  - cloned `SessionSource` when materializing `StoredThread`;
  - coerced factory results to `Arc<dyn LiveThreadHandle>`;
  - removed the private live handle's invalid `Debug` derive.

## Verification

- `rustfmt codex-rs/thread/thread-store-api/src/recording.rs`
- `cargo check --release -p codex-thread-store-api`

The first focused Cargo check exposed the local API issues listed above. After
the follow-up fixes, the same package-only release check passed.

Passing log:

- `codex-rs/logs/thread-store-api-recording-check-20260521-001708.log`

Failed diagnostic log kept for traceability:

- `codex-rs/logs/thread-store-api-recording-check-20260521-001524.log`

No broad `codex-core` or workspace build/test lane was run.

## Commit Scope

Stage only:

- `codex-rs/thread/thread-store-api/src/recording.rs`
- `.codex/workflow/agents/thread_store_api_recording_repair_worker.handoff.md`

Do not stage generated logs unless root explicitly wants them.
