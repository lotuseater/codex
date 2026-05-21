status: finding

# solid_refactor_readonly_dependency_lock_review_worker handoff

## Scope and boundary

Read-only dependency/lock review only. I did not edit source, stage, commit, run Cargo/Bazel/just/tests/formatters/schema generation, or overwrite other workers' changes. The only write is this handoff.

## Current manifest and lock groups

### Replacement-shadow cleanup

- Current dependency diff does not contain a `codex-replacement-shadow` hunk in `codex-rs/core/Cargo.toml` or `codex-rs/Cargo.lock`.
- `rg` found no `codex_replacement_shadow` / `codex-replacement-shadow` references in `codex-rs/core/Cargo.toml`, `codex-rs/core/src`, or `codex-rs/Cargo.lock`.
- Treat the replacement-shadow core dependency finding as already resolved at the current source/lock state; it should not be bundled with the remaining dirty lock hunks unless root is committing a larger verified integration slice.

### Agent-depth policy/graph

- Source changes are present in `codex-rs/agent-policy/src/lib.rs`, `codex-rs/agent-graph-store/src/lib.rs`, and `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`.
- No agent crate manifest diff is present for `codex-rs/agent-policy/Cargo.toml`, `codex-rs/agent-graph-store/Cargo.toml`, or `codex-rs/agent-identity/Cargo.toml`.
- The current `codex-rs/Cargo.lock` has no agent-policy/graph-specific dependency hunk.
- The agent-depth worker's `codex-agent-policy` verification passed, but its `codex-core` `multi_agent_v2` verification is still blocked by the core test-support dependency issue below.

### Core / thread-store / test-support

- `codex-rs/core/Cargo.toml` adds four `[[test]]` targets: `compact_remote_parity`, `prompt_debug_tests`, `quota_exceeded`, and `rollout_list_find`.
- `codex-rs/core/Cargo.toml` also adds `codex-thread-store = { workspace = true }`.
- `codex-rs/Cargo.lock` reflects that by adding `"codex-thread-store"` to the `codex-core` package dependency list.
- Current source imports `codex_thread_store` and/or `codex_thread_store_api` from:
  - `codex-rs/core/src/lib.rs`
  - `codex-rs/core/src/prompt_debug.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
  - `codex-rs/core/tests/suite/client.rs`
  - `codex-rs/core/tests/common/test_codex.rs`
- `codex-rs/core/tests/common/Cargo.toml` still does not declare `codex-thread-store` or `codex-thread-store-api`.
- `codex-rs/Cargo.lock` still does not add `codex-thread-store` or `codex-thread-store-api` to the `core_test_support` package block.

Conclusion: the reported `core_test_support` blocker is a dependency declaration issue, not a stale import issue. It is also not fixed by the current `codex-rs/core/Cargo.toml` hunk, because `core_test_support` is its own crate under `codex-rs/core/tests/common/Cargo.toml`.

### Core-api / core-domain identifiers

- `codex-rs/core-api/Cargo.toml` adds `codex-core-domain-types = { workspace = true }`.
- `codex-rs/core-domain/types/Cargo.toml` adds `serde = { workspace = true, features = ["derive"] }`.
- `codex-rs/core-api/src/identifiers.rs` is an untracked new file that re-exports `SessionId`, `ThreadId`, `ToolCallId`, and `TurnId` from `codex_core_domain_types`.
- `codex-rs/core-domain/types/src/lib.rs` owns the moved string identifier types and derives serde support.
- `codex-rs/Cargo.lock` currently reflects this by adding `"codex-core-domain-types"` to `codex-core-api` and `"serde"` to `codex-core-domain-types`.

Conclusion: the earlier core-api worker's stale-lock finding appears resolved in the current `Cargo.lock`, but the lockfile hunk is mixed with the core/thread-store hunk, so it is not safe as a file-level commit boundary.

### App-server permissions/schema

- App-server source changes are present in `codex-rs/app-server-protocol/src/protocol/thread_history.rs`, `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs`, and `codex-rs/app-server-protocol/src/protocol/v2/tests.rs`.
- App-server generated schema JSON has a broad dirty set under `codex-rs/app-server-protocol/schema/json`, including permission-related files and many config/thread/review notification/response files.
- No app-server dependency manifest or `Cargo.lock` hunk is present for this slice.

Conclusion: this is a schema/source boundary, not a Cargo dependency boundary. The schema JSON is too broad to treat as a dependency-lock slice.

## Concrete blockers

1. `core_test_support` still lacks direct dependency declarations for `codex-thread-store` and `codex-thread-store-api`.
2. `codex-rs/Cargo.lock` is mixed across at least two owner slices:
   - `codex-core` -> `codex-thread-store`
   - `codex-core-api` -> `codex-core-domain-types`
   - `codex-core-domain-types` -> `serde`
3. `codex-rs/core/Cargo.toml` is also mixed:
   - test target registration
   - core crate `codex-thread-store` dependency
4. The app-server schema JSON dirty set is broad and should be resnapshotted after source/schema owners settle before staging.

## Commit-boundary advice

No safe file-level `git add -- ...` list exists right now.

Root should not commit `codex-rs/Cargo.lock` as-is for any single owner slice. A file-level add would mix core thread-store/test-support dependency work with core-api/domain dependency work. Patch staging is possible later, but only after the missing `core_test_support` manifest dependency declarations are added and lock regeneration confirms the exact package-block changes.

## Exact next action for root

1. Add `codex-thread-store` and `codex-thread-store-api` to `codex-rs/core/tests/common/Cargo.toml`.
2. Regenerate/refresh the lock state so the `core_test_support` package block gains those two dependencies.
3. Rerun the agent-depth worker's blocked verification lane:
   `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter multi_agent_v2 -AllowBroadCoreLibUnitTests`
4. After that passes, resnapshot the dependency diff and split staging by owner. Until then, keep `codex-rs/Cargo.lock`, `codex-rs/core/Cargo.toml`, `codex-rs/core-api/Cargo.toml`, and `codex-rs/core-domain/types/Cargo.toml` out of a file-level mixed commit.
