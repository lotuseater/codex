# solid_refactor_area_review_retry_tests_schema_worker Handoff

Status: retry read-only review complete. I only wrote this handoff file. I did not run cargo, rustc, just, Bazel, schema generation, build/test scripts, staging, commits, or pushes.

## Findings

### P1 - `ConfigToml` schema fixture is stale after adding `apps_mcp_product_sku`

Evidence:
- `codex-rs/config/src/config_toml.rs:326` adds `pub apps_mcp_product_sku: Option<String>,` to `ConfigToml`.
- `codex-rs/config/src/profile_toml.rs:49` adds the same profile-level field.
- `git diff --name-status -- codex-rs\config\src\config_toml.rs codex-rs\config\src\profile_toml.rs codex-rs\core\config.schema.json` reports only the two Rust files as modified; `codex-rs/core/config.schema.json` is not dirty.
- `rg -n apps_mcp_product_sku codex-rs\core\config.schema.json` returns no matches.

Why this blocks commit grouping: repo guidance requires `just write-config-schema` after changing `ConfigToml` or nested config types. Committing the config source fields without the regenerated `codex-rs/core/config.schema.json` leaves the generated fixture stale.

Root-owned next action: after the config source shape is final, run `just write-config-schema` from `codex-rs`, include the resulting `codex-rs/core/config.schema.json` diff in the same config/API commit, then verify `rg -n apps_mcp_product_sku codex-rs\core\config.schema.json` finds the generated field.

### P1 - `codex-thread-store` is a concrete store dependency in normal `codex-core` dependencies even though current uses are test-only

Evidence:
- `codex-rs/core/Cargo.toml:71` starts `[dependencies]`; `codex-rs/core/Cargo.toml:132` adds `codex-thread-store = { workspace = true }` there.
- `codex-rs/core/Cargo.toml:206` starts `[dev-dependencies]`, where test-only support dependencies belong; `codex-rs/core/Cargo.toml:212` already keeps `core_test_support` there.
- Current `codex_thread_store::` uses found in this review scope are test-only paths: `codex-rs/core/tests/common/test_codex.rs:44-45`, `codex-rs/core/tests/suite/client.rs:50-51`, and `codex-rs/core/src/tools/handlers/multi_agents_tests.rs:55-56`.
- `codex-rs/Cargo.lock:2706` reflects `codex-thread-store` under the dirty `codex-core` lock entry.

Why this blocks commit grouping: the SOLID/refactor goal is to prevent concrete low-level thread-store dependencies from leaking back into `codex-core`. The observed use sites are tests/test helpers, so adding the concrete store to normal crate dependencies expands production dependency fan-in unnecessarily.

Root-owned next action: move `codex-thread-store` out of `codex-rs/core/Cargo.toml` `[dependencies]` and into `[dev-dependencies]` if these test-only references still need it, or route the remaining test construction through `core_test_support`; then refresh `Cargo.lock` and any Bazel lock follow-up from that corrected manifest state.

## Grouping Checks

- `git diff --cached --name-status` was empty, so I did not see app-server schema JSON or `Cargo.lock` staged/committed prematurely.
- App-server protocol schema JSON is dirty alongside source DTO/API edits in `codex-rs/app-server-protocol/src/protocol/thread_history.rs`, `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs`, and `codex-rs/app-server-protocol/src/protocol/v2/tests.rs`; keep those generated JSON files with the source/API change that caused them, not as a standalone commit.
- The stale test API repairs I inspected look structurally aligned with the newer settings shape: `codex-rs/core/tests/suite/mcp_turn_metadata.rs:80-86` now supplies `cwd`, approval policy, sandbox policy, permission profile, and collaboration mode; `codex-rs/core/src/tools/handlers/multi_agents_tests.rs:4216-4222` now asserts approval and permission profile through the current permission model.

## Verification Not Run

Per worker restrictions, I did not run cargo, just, Bazel, schema generation, or scripts. Root owns the corrected narrow verification after applying the two fixes above.
