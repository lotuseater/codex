# thread_store_boundary Handoff

Status: inspected and ready for root integration.

Date: 2026-05-20

## Paths Changed

- `.codex/workflow/agents/thread_store_boundary.handoff.md`

## Paths Read

- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/thread/thread-store-api/src/lib.rs`
- `codex-rs/thread/thread-store-api/src/store.rs`
- `codex-rs/thread/thread-store-api/src/live_thread.rs`
- `codex-rs/thread/thread-store-api/src/types.rs`
- `codex-rs/thread/thread-store-api/src/error.rs`
- `codex-rs/thread/thread-store/Cargo.toml`
- `codex-rs/thread/thread-store-api/Cargo.toml`
- `codex-rs/thread/thread-store/src/lib.rs`
- `codex-rs/thread/thread-store/src/factory.rs`
- `codex-rs/thread/thread-store/src/in_memory.rs`
- `codex-rs/thread/thread-store/src/live_thread.rs`
- `codex-rs/thread/thread-store/src/local/mod.rs`
- `codex-rs/thread/thread-store/src/local/create_thread.rs`
- `codex-rs/thread/thread-store/src/local/live_writer.rs`
- `codex-rs/thread/thread-store/src/local/update_thread_metadata.rs`
- `codex-rs/rollout/src/state_db.rs`
- `codex-rs/state/src/runtime/threads.rs`
- `codex-rs/core/src/thread_manager.rs`
- `codex-rs/core/src/thread_manager_tests.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/session/tests/guardian_tests.rs`
- `codex-rs/core/src/agent/control_tests.rs`
- `codex-rs/core/src/prompt_debug.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- `codex-rs/core/tests/common/test_codex.rs`
- `codex-rs/core/tests/suite/client.rs`

## Concrete Store Symbols Blocking Core Decoupling

- `LocalThreadStore` and `LocalThreadStoreConfig`
  - `codex-rs/core/src/session/mod.rs:580-583` downcasts `Arc<dyn ThreadStore>` to `LocalThreadStore` to recover `state_db()` before reading persisted dynamic tools.
  - `codex-rs/core/src/thread_manager.rs:351-357` constructs `LocalThreadStore` directly in `ThreadManager::new_for_testing`.
  - `codex-rs/core/src/agent/control_tests.rs:1973-1975` constructs `LocalThreadStore` directly.
  - `codex-rs/core/src/session/tests.rs:3736-3737`, `3888-3889`, `4163-4164`, `4267-4268`, and `5748-5750` construct `LocalThreadStore` directly.
  - `codex-rs/core/src/session/tests/guardian_tests.rs:736-738` constructs `LocalThreadStore` directly.
- `InMemoryThreadStore` and `InMemoryThreadStoreCalls`
  - `codex-rs/core/src/session/tests.rs:5305-5306` creates `InMemoryThreadStore::default()` and coerces it to `Arc<dyn codex_thread_store_api::ThreadStore>`.
  - `codex-rs/core/src/session/tests.rs:5337-5343` asserts against `InMemoryThreadStoreCalls`.
  - `codex-rs/core/src/thread_manager_tests.rs:858-860` downcasts `ThreadStore` to `InMemoryThreadStore` for call assertions.
- `LiveThread`
  - `codex-rs/core/src/session/tests.rs:2945-2947` and `5308-5310` call `LiveThread::create(...)` directly.
- `thread_store_from_config`
  - `codex-rs/core/src/prompt_debug.rs:22` / `42` import and call `crate::thread_manager::thread_store_from_config`.
  - `codex-rs/core/src/tools/handlers/multi_agents_tests.rs:9` / `3878` import and call it.
  - `codex-rs/core/src/thread_manager_tests.rs:501`, `611`, `658`, `715`, `771`, `857`, `972`, `1190`, `1298`, `1395`, `1538` call it.
  - `codex-rs/core/tests/common/test_codex.rs:22` / `430` and `codex-rs/core/tests/suite/client.rs:9` / `1129` import it from `codex_core`.
  - Current static search found `thread_store_from_config` defined only in `codex-rs/thread/thread-store/src/factory.rs`; no matching definition/export was found in `codex-rs/core/src/thread_manager.rs`. Root should treat the core references as unresolved until replaced or rehomed.
- `ThreadStore::as_any()` / concrete downcasts
  - The trait still exposes `as_any()` in `codex-rs/thread/thread-store-api/src/store.rs:26-28`.
  - Core uses it for `LocalThreadStore` and `InMemoryThreadStore` downcasts, which preserves a concrete boundary leak even when imports are indirect or currently unresolved.

## API Abstraction Changes Needed

- Add a storage-neutral dynamic-tools read API so `codex-core` does not downcast to `LocalThreadStore` for `state_db()`:
  - Suggested type: `ReadThreadDynamicToolsParams { thread_id: ThreadId }` or `LoadThreadDynamicToolsParams { thread_id: ThreadId }` in `codex-thread-store-api`.
  - Suggested trait method on `ThreadStore`: `async fn read_thread_dynamic_tools(&self, params: ReadThreadDynamicToolsParams) -> ThreadStoreResult<Option<Vec<DynamicToolSpec>>>`.
  - `LocalThreadStore` can implement this using its existing `state_db()` and `codex_rollout::state_db::get_dynamic_tools(..., "codex_spawn")`.
  - `InMemoryThreadStore` can implement this from `CreateThreadParams.dynamic_tools` plus any latest `ThreadMetadataPatch.dynamic_tools` override, which keeps config-driven in-memory tests useful without core downcasts.
  - Default `UnsupportedThreadStore` behavior should be `Ok(None)` or `Unsupported`, depending on whether root wants missing persistence to silently fall back to rollout history. Existing core behavior treats missing state DB as `None`, so `Ok(None)` is the least disruptive default.
- Provide API-only fakes or root-granted core test fakes before removing concrete test store usage:
  - Needed fake store behavior: create/resume/append/shutdown/discard/load-history/read-by-rollout-path plus lightweight call counts for the existing shutdown and rollout-path tests.
  - Needed fake live handle/factory behavior: enough to replace `LiveThread::create(...)` in core tests without importing `codex_thread_store::LiveThread`.
  - The existing `UnsupportedThreadStore` / `UnsupportedLiveThreadFactory` is useful for no-persistence contexts, but it cannot replace tests that expect persistence side effects or call counts.
- Longer-term cleanup: stop using `ThreadStore::as_any()` in core. If downcasting remains necessary in tests, keep it in test-only fake code outside core production paths.

## Root-Owned Manifest Entries Needed

- `codex-core` should depend on `codex-thread-store-api` only.
- `codex-core` should not depend on `codex-thread-store` once the concrete references above are removed.
- Application or integration crates that select local/in-memory persistence should own the `codex-thread-store` dependency and pass `Arc<dyn ThreadStore>` / `Arc<dyn LiveThreadFactory>` into core.
- If root chooses API-exported recording fakes instead of core-local fakes, root should wire any required workspace dependency or feature for those fakes. I did not edit root-owned manifests or lockfiles.

## Verification Performed

- Read the required workflow docs first.
- Ran focused read-only `rg --color never` scans for `codex_thread_store::`, `LocalThreadStore`, `LocalThreadStoreConfig`, `InMemoryThreadStore`, `InMemoryThreadStoreCalls`, `LiveThread`, `thread_store_from_config`, `ThreadStore::as_any`, and related API symbols in the owned thread-store crates plus `codex-core`.
- Inspected the concrete dynamic-tools flow:
  - core downcast at `codex-rs/core/src/session/mod.rs:580-583`;
  - rollout bridge `codex-rs/rollout/src/state_db.rs:458-470`;
  - state runtime `codex-rs/state/src/runtime/threads.rs:78-92`;
  - local store `state_db()` at `codex-rs/thread/thread-store/src/local/mod.rs:99-102`.
- No formatters, broad Cargo builds, Just tasks, Git staging, commits, resets, or checkouts were run.

## Blockers

- Core edits were not allowed in this lane, so concrete core references remain.
- Core test fake files were not granted by root, so I did not create or move test fakes in core.
- Root-owned manifests and lockfiles were forbidden, so dependency wiring is only proposed here.
- No Rust compile/test verification was run because this lane forbids broad Cargo builds and the remaining blockers are in root-owned core/manifest files.
