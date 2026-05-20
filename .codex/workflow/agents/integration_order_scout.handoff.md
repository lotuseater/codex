# integration_order_scout Handoff

Status: completed read-only synthesis on 2026-05-20.

## Scope

Built a practical integration order for the completed SOLID refactor worker
slices. No source files, Cargo/Just commands, formatters, staging, commits, or
broad verification were run.

## Sources Inspected

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/canary_observer.handoff.md`
- `.codex/workflow/agents/auth_boundary.handoff.md`
- `.codex/workflow/agents/thread_store_boundary.handoff.md`
- `.codex/workflow/agents/thread_projection_boundary.handoff.md`
- `.codex/workflow/agents/mcp_elicitation_boundary.handoff.md`
- `.codex/workflow/agents/app_catalog_followup.handoff.md`
- `.codex/workflow/agents/dab_availability_worker.handoff.md`

## Recommendation

Integrate the `thread_store_boundary` slice first.

Why: the canary reports 23 current violations: 1 transitive dependency violation
and 22 source-pattern leaks. The thread-store lane accounts for the largest
source-pattern cluster: `LocalThreadStore` (5), `LocalThreadStoreConfig` (4),
`thread_store_from_config` (3), and `InMemoryThreadStore` (2). It also overlaps
the first compile blocker reported by the DAB verification lane:
`undeclared LocalThreadStore`. Fixing this lane first should reduce boundary
canary noise fastest and unblock later protocol work in `thread_manager.rs` and
`session/mod.rs`.

## Recommended Integration Order

1. `thread_store_boundary`
   - Remove concrete thread-store implementation references from `codex-core`.
   - Keep `codex-core` on `codex-thread-store-api` only; do not add a
     `codex-thread-store` implementation dependency to core.
   - Add the needed store API surface for dynamic tools/state access instead of
     downcasting `Arc<dyn ThreadStore>` to `LocalThreadStore`.
   - Move or wrap `thread_store_from_config` so core production code no longer
     owns concrete local/in-memory store construction.

2. `auth_boundary`
   - Root `codex-rs/Cargo.toml` already contains `runtime-domain/auth-api` and
     `codex-auth-api`; the remaining work is per-crate dependency wiring and
     callsite migration.
   - Migrate `AuthMode` usage in core/login away from
     `codex_app_server_protocol::AuthMode`.
   - Keep app-server protocol as the wire DTO boundary, with explicit
     conversion to/from `codex-auth-api::AuthMode`.

3. `mcp_elicitation_boundary`
   - Add the new `codex-rs/mcp/elicitation-api` crate to the workspace and wire
     downstream crates only after the core/store and auth cleanup narrows the
     protocol leak surface.
   - Move core/session MCP elicitation use to the MCP-owned type crate, then
     map at the app-server protocol boundary.
   - Regenerate app-server schema fixtures only after the protocol mapping is
     intentionally changed.

4. `thread_projection_boundary`
   - Wire `codex-rs/thread/thread-projection-api` after `thread_manager.rs` is
     no longer also carrying concrete store construction leaks.
   - Treat `ThreadHistoryBuilder`, `TurnStatus`, `ThreadItem`, and item-builder
     coupling as a root-approved protocol-boundary slice, not a mechanical crate
     import swap.

5. `dab_availability_worker`
   - Preserve the completed DAB registration changes, but do not use this slice
     as the integration wedge. Its focused core test is blocked by unrelated
     dirty-tree compile errors.
   - Re-run the DAB focused release lane after the thread-store/core compile
     blockers are cleared.

6. `app_catalog_followup`
   - No integration action is required for the app catalog data-model boundary.
     The handoff found no remaining app catalog data-model leak in `core`,
     `connectors`, or `tools`, and no root manifest edit is needed.

## Dependency Rationale

- `thread_store_boundary` first reduces the largest canary cluster and addresses
  a live compile blocker (`LocalThreadStore`) seen while verifying DAB.
- `auth_boundary` is lower blast radius than MCP/thread projection and removes
  straightforward `AuthMode` protocol leaks from core/login once crate-level
  dependencies are added.
- `mcp_elicitation_boundary` should wait until auth is no longer mixed into the
  same protocol cleanup. It requires app-server protocol mapping and schema
  fixture work.
- `thread_projection_boundary` should wait because `ThreadHistoryBuilder`
  currently constructs app-server `ThreadItem` variants and uses item-builder
  helpers. Moving it too early would combine store, thread projection, and
  protocol DTO concerns in the same root edit.
- `dab_availability_worker` is already a contained source change. Its remaining
  blocker is not DAB design; it is the current dirty tree failing to compile
  before the DAB canary can run.
- `app_catalog_followup` is a confirmation note, not an implementation slice.

## Exact Blockers By Lane

### Canary Observer

- Static boundary canary exits `1`.
- Current counts:
  - direct forbidden crate dependencies: 0
  - transitive forbidden crate dependencies: 1
  - source-pattern leaks: 22
- Source-pattern counts:
  - `codex_app_server_protocol::`: 8
  - `LocalThreadStore`: 5
  - `LocalThreadStoreConfig`: 4
  - `thread_store_from_config`: 3
  - `InMemoryThreadStore`: 2
- Dependency violation: `codex-core` transitively depends on
  `codex-app-server-protocol`.

### Thread Store

- Core edits were not allowed in the worker lane, so concrete core references
  remain.
- Core test fake files were not granted, so the worker did not move/create test
  fakes.
- Root-owned manifests and lockfiles were forbidden, so dependency wiring was
  only proposed.
- Remaining concrete blockers are in `thread_manager.rs`, `session/mod.rs`,
  core tests, `prompt_debug.rs`, and `multi_agents_tests.rs`.

### Auth

- The worker added `codex-auth-api::AuthMode`, but core/login callsites still
  import or use app-server protocol `AuthMode`.
- Root or a granted lane must add per-crate dependencies where needed and
  migrate callers.
- Compile/test verification is blocked until caller migration is done.

### MCP Elicitation

- New crate files exist under `codex-rs/mcp/elicitation-api`, but the crate is
  not wired as a workspace member.
- Core import rewiring is blocked on root-owned manifest edits.
- App-server protocol conversion/schema fixture updates are blocked until root
  grants/apply protocol edits.

### Thread Projection

- New crate files exist under `codex-rs/thread/thread-projection-api`, but the
  crate is intentionally unwired.
- Workspace membership and downstream crate dependencies are required before
  Cargo/Bazel can validate it.
- Moving `ThreadHistoryBuilder` is blocked by current construction of
  app-server `ThreadItem` variants and use of item-builder helpers.

### App Catalog

- No remaining app catalog data-model leak was found in `core`, `connectors`,
  or `tools`.
- No root manifest edit is needed for the handoff-only slice.
- Future work requires exact root grants only if moving
  `codex_tools::DiscoverableTool`, request-plugin-install structures, or
  conversion helpers.

### DAB Availability

- DAB registration changes were made in core tools/registry/spec planning.
- `codex-tools` focused release test passed per handoff.
- `codex-core` focused release test could not reach the DAB canary because the
  dirty tree fails to compile first.
- First reported compile blockers include unresolved imports for hook runtime,
  permissions, and skills helpers, plus API mismatches such as missing
  `Session::input_queue`, `Op::UserInput.thread_settings`, and undeclared
  `LocalThreadStore`.

## Root-Owned Files Likely Touched

### Cross-cutting manifests and generated metadata

- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- `codex-rs/core/Cargo.toml`
- `codex-rs/login/Cargo.toml`
- `codex-rs/tools/Cargo.toml`
- `codex-rs/app-server-protocol/Cargo.toml`
- `codex-rs/app-server/Cargo.toml`
- Bazel BUILD/package files for any newly wired crates
- `MODULE.bazel.lock` if dependency/lock refresh is required

### Thread store first slice

- `codex-rs/thread/thread-store-api/src/lib.rs`
- `codex-rs/thread/thread-store/src/local/mod.rs`
- `codex-rs/core/src/thread_manager.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/prompt_debug.rs`
- `codex-rs/core/src/agent/control_tests.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/session/tests/guardian_tests.rs`
- `codex-rs/core/src/thread_manager_tests.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`

### Auth slice

- `codex-rs/runtime-domain/auth-api/src/lib.rs`
- `codex-rs/app-server-protocol/src/protocol/common.rs`
- `codex-rs/core/src/client.rs`
- `codex-rs/core/src/client_tests.rs`
- `codex-rs/core/src/compact_remote.rs`
- `codex-rs/core/src/realtime_conversation.rs`
- `codex-rs/login/src/server.rs`
- `codex-rs/login/src/auth/*.rs`

### MCP elicitation slice

- `codex-rs/mcp/elicitation-api/Cargo.toml`
- `codex-rs/mcp/elicitation-api/src/lib.rs`
- `codex-rs/app-server-protocol/src/protocol/common.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/mcp.rs`
- `codex-rs/core/src/mcp_tool_call.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/tools/src/request_plugin_install.rs`

### Thread projection slice

- `codex-rs/thread/thread-projection-api/Cargo.toml`
- `codex-rs/thread/thread-projection-api/src/lib.rs`
- `codex-rs/thread/thread-projection-api/src/page.rs`
- `codex-rs/thread/thread-projection-api/src/turn.rs`
- `codex-rs/app-server-protocol/src/protocol/thread_history.rs`
- `codex-rs/app-server-protocol/src/protocol/item_builders.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/turn.rs`
- `codex-rs/core/src/thread_manager.rs`

### DAB slice

- `codex-rs/core/src/tools/handlers/mod.rs`
- `codex-rs/core/src/tools/handlers/desktop_automation.rs`
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/spec_plan_tests.rs`
- `codex-rs/core/src/tools/registry.rs`

## Slices That Should Wait

- `thread_projection_boundary` should wait until thread-store concrete leaks
  are removed from `thread_manager.rs`.
- `mcp_elicitation_boundary` should wait until auth migration is complete and
  root is ready to update app-server protocol schema fixtures.
- `dab_availability_worker` should wait for dirty-tree compile blockers to be
  cleared before rerunning its core canary.
- `app_catalog_followup` should wait unless root explicitly wants to move
  `DiscoverableTool`, request-plugin-install structures, or conversion helpers.
