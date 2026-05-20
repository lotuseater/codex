# thread_store_integration_rescue Handoff

Status: completed read-only scout on 2026-05-20.

No source, manifest, lockfile, Bazel, generated, snapshot, Cargo/Just, formatter,
staging, or commit actions were performed. The only write from this scout is this
handoff.

## Files Read

Required handoffs:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/thread_store_boundary.handoff.md`
- `.codex/workflow/agents/compile_session_store_scout.handoff.md`
- `.codex/workflow/agents/integration_order_scout.handoff.md`
- `.codex/workflow/agents/core_dependency_map_scout.handoff.md`

Focused source and manifests:

- `codex-rs/core/Cargo.toml`
- `codex-rs/Cargo.toml`
- `codex-rs/app-server/Cargo.toml`
- `codex-rs/core/src/thread_manager.rs`
- `codex-rs/core/src/prompt_debug.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/session.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/session/tests/guardian_tests.rs`
- `codex-rs/core/tests/common/test_codex.rs`
- `codex-rs/thread/thread-store-api/Cargo.toml`
- `codex-rs/thread/thread-store-api/src/lib.rs`
- `codex-rs/thread/thread-store-api/src/store.rs`
- `codex-rs/thread/thread-store-api/src/live_thread.rs`
- `codex-rs/thread/thread-store-api/src/recording.rs`
- `codex-rs/thread/thread-store/Cargo.toml`
- `codex-rs/thread/thread-store/BUILD.bazel`
- `codex-rs/thread/thread-store/src/lib.rs`
- `codex-rs/thread/thread-store/src/factory.rs`
- `codex-rs/thread/thread-store/src/live_thread.rs`
- `codex-rs/thread/thread-store/src/local/mod.rs`
- `codex-rs/thread/thread-store/src/in_memory.rs`
- `codex-rs/app-server/src/message_processor.rs`
- `codex-rs/app-server/src/mcp_refresh.rs`
- `codex-rs/app-server/src/lib.rs`

Bounded rg also inspected references under `codex-rs/core/src`,
`codex-rs/core/tests`, `codex-rs/core/tests/common`, and `codex-rs/thread`.

## Exact Compile Blockers

### Production blockers in the requested focus

- `codex-rs/core/src/thread_manager.rs:351-357` constructs
  `LocalThreadStore::new(LocalThreadStoreConfig { ... }, state_db.clone())`.
  `thread_manager.rs` imports `codex_thread_store_api::*` at `55-63`, but no
  `LocalThreadStore` or `LocalThreadStoreConfig` is imported or available from
  `codex-core`'s dependencies. Fixing this by adding `codex-thread-store` to
  `codex-core` would violate the boundary.
- `codex-rs/core/src/prompt_debug.rs:22` imports
  `crate::thread_manager::thread_store_from_config`; `:42` calls it. There is
  no current `thread_store_from_config` definition/export in
  `codex-rs/core/src/thread_manager.rs`; the concrete factory now lives at
  `codex-rs/thread/thread-store/src/factory.rs:18-29`.
- `codex-rs/core/src/prompt_debug.rs:44-63` now passes
  `UnsupportedLiveThreadFactory`, but still tries to build a concrete
  `thread_store`. Because `build_prompt_input` sets `config.ephemeral = true`
  at `:32`, this should use `codex_thread_store_api::UnsupportedThreadStore`
  rather than any concrete local store.

### Session/test blockers in the requested focus

- `codex-rs/core/src/session/session.rs:395-418` requires
  `thread_store`, `live_thread_factory`, `state_db`, `parent_rollout_thread_trace`,
  then `attestation_provider`. Direct `Session::new` test callsites in
  `codex-rs/core/src/session/tests.rs` have not been updated:
  `:3718-3742`, `:4145-4170`, and `:4249-4280` pass a concrete store and then
  the trace context, omitting both `live_thread_factory` and `state_db`.
- `codex-rs/core/src/session/tests.rs` still references concrete implementation
  types: `LocalThreadStore`/`LocalThreadStoreConfig` at `:3736-3738`,
  `:3888-3891`, `:4163-4166`, `:4267-4277`, and `:5748-5751`;
  `InMemoryThreadStore` and `InMemoryThreadStoreCalls` at `:5305-5306` and
  `:5337-5342`.
- `codex-rs/core/src/session/tests/guardian_tests.rs:736-739` constructs
  `LocalThreadStore`; `:769-770` builds `CodexSpawnArgs` without the required
  `live_thread_factory` and `state_db` fields present in
  `codex-rs/core/src/session/mod.rs:426-428`.
- `codex-rs/core/tests/common/test_codex.rs:22` imports
  `codex_core::thread_store_from_config`, and `:430` calls it. That export is
  absent. The `ThreadManager::new` call at `:432-443` also omits the
  `live_thread_factory` parameter required by `thread_manager.rs:239-241`.

### Additional core blockers found by bounded rg

These are outside the narrow requested focus but will still block a full
`codex-core` test build if not assigned:

- `codex-rs/core/tests/suite/client.rs:9`, `:1129` use missing
  `codex_core::thread_store_from_config`.
- `codex-rs/core/src/thread_manager_tests.rs:501`, `:611`, `:658`, `:715`,
  `:771`, `:857`, `:972`, `:1190`, `:1298`, `:1395`, `:1538` use missing
  `thread_store_from_config`.
- `codex-rs/core/src/tools/handlers/multi_agents_tests.rs:9`, `:3878` use
  missing `crate::thread_manager::thread_store_from_config`.
- `codex-rs/core/src/agent/control_tests.rs:1973-1974` references
  `LocalThreadStore`/`LocalThreadStoreConfig`.

## Recommended Implementation Order

1. Production core first: in `codex-rs/core/src/thread_manager.rs`, remove the
   concrete local-store fallback from the test constructor and use
   `codex_thread_store_api::UnsupportedThreadStore` plus
   `UnsupportedLiveThreadFactory` for constructors that have no config/store
   input. Do not recreate `thread_store_from_config` in core.
2. Fix `codex-rs/core/src/prompt_debug.rs`: replace the unresolved
   `thread_store_from_config` import/call with `UnsupportedThreadStore` because
   this path forces ephemeral config and only needs an API object to satisfy
   `ThreadManager::new`.
3. Fix direct core session tests in `codex-rs/core/src/session/tests.rs` and
   `codex-rs/core/src/session/tests/guardian_tests.rs` by using API-owned test
   fakes: `RecordingThreadStore`, `RecordingLiveThreadFactory`, and
   `RecordingThreadStoreCalls` where call counters are asserted; use
   `UnsupportedThreadStore`/`UnsupportedLiveThreadFactory` for tests that do not
   exercise persistence. Insert the missing `live_thread_factory` and `state_db`
   arguments at every direct `Session::new`/`CodexSpawnArgs` callsite.
4. Fix `codex-rs/core/tests/common/test_codex.rs` by replacing
   `codex_core::thread_store_from_config` with API fakes and by passing an
   `Arc<dyn LiveThreadFactory>` into `ThreadManager::new`.
5. Only after the focused files compile conceptually, assign a separate worker
   for the broader core references listed above (`thread_manager_tests.rs`,
   `multi_agents_tests.rs`, `core/tests/suite/client.rs`,
   `agent/control_tests.rs`). Use the same API-fake pattern; do not route those
   tests through `codex-thread-store`.

## Path Ownership Warnings

- Current status shows externally modified files in the target area:
  `codex-rs/core/src/thread_manager.rs`, `codex-rs/core/src/prompt_debug.rs`,
  `codex-rs/core/src/session/context_budget.rs`,
  `codex-rs/core/src/session/mod.rs`,
  `codex-rs/core/src/session/session.rs`,
  `codex-rs/core/src/session/tests.rs`,
  `codex-rs/core/src/session/tests/guardian_tests.rs`,
  `codex-rs/core/src/session/turn.rs`, and untracked `codex-rs/thread/`.
  The next worker must read current content before editing and must not revert
  unrelated changes.
- Do not split ownership of `codex-rs/core/src/session/tests.rs`; it contains
  both constructor-argument and concrete-store fixes, and parallel edits will
  collide.
- Suggested non-overlapping worker ownership:
  - Worker A: `codex-rs/core/src/thread_manager.rs` and
    `codex-rs/core/src/prompt_debug.rs`.
  - Worker B: `codex-rs/core/src/session/tests.rs` and
    `codex-rs/core/src/session/tests/guardian_tests.rs`.
  - Worker C: `codex-rs/core/tests/common/test_codex.rs`, then the broader
    core test references only if explicitly granted.
  - Root: manifests, lockfiles, Bazel files, generated files, snapshots,
    staging/commit, and final verification.
- Keep `codex-rs/thread/thread-store/**` owned by the thread-store worker unless
  root explicitly grants follow-up changes. Core integration should consume
  only `codex-thread-store-api`.

## Root-Owned Manifest And Build Actions

- `codex-rs/core/Cargo.toml:74` already depends on
  `codex-thread-store-api`; no `codex-thread-store` dependency should be added
  to `codex-core`, including dev-dependencies.
- `codex-rs/Cargo.toml:159-160` includes both new workspace crates and
  `:322-323` has workspace dependency entries.
- `codex-rs/app-server/Cargo.toml:72-73` owns both `codex-thread-store` and
  `codex-thread-store-api`; app-server is the correct place for concrete store
  selection (`message_processor.rs`, `mcp_refresh.rs`, `lib.rs` already import
  `codex_thread_store`).
- Bazel follow-up appears root-owned: `codex-rs/thread/thread-store/BUILD.bazel`
  exists, but no `BUILD.bazel` was found under
  `codex-rs/thread/thread-store-api/`. If Bazel lanes are expected before CI,
  root should add/check the missing API crate Bazel target and any dependency
  wiring, then run the repo-required Bazel lock/update lane.
- If any manifest or lockfile is changed, run the repo-required
  `just bazel-lock-update` and `just bazel-lock-check` from the repo root.

## Verification Lane And Commit Readiness

- This scout did not run Cargo/Just/formatters by instruction.
- After implementation, run `just fmt` in `codex-rs`.
- For the focused slice, run:
  `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core`
  from the repo root. If root wants faster first feedback, use the same script
  with the most relevant test filters for `prompt_debug`, session tests, and
  `core/tests/common` before the full `codex-core` release package lane.
- Run `just fix -p codex-core` in `codex-rs` before finalizing the large Rust
  slice; per repo guidance, do not rerun tests solely because `fmt`/`fix` ran.
- Commit readiness: not ready. The coherent commit boundary is after the core
  concrete-store leaks are removed, direct constructor callsites are updated,
  no `codex-core -> codex-thread-store` dependency exists, and the focused
  release verification lane is green.
