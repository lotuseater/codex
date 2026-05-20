# app_server_boundary_rescue Handoff

Status: completed read-only boundary rescue on 2026-05-20.

## Scope

Inspected app-server boundary leaks and app-server owned DTO/domain mapping seams,
with emphasis on `codex-rs/app-server/src/**`,
`codex-rs/app-server-protocol/src/**`, and the existing app/thread domain API
crates. I did not edit Rust source, manifests, lockfiles, Bazel files,
generated schema fixtures, snapshots, or run Cargo/Just/formatters/staging.

One existing helper, `protocol_domain_scout`, completed but returned only reduced
context and reported no repo inspection. `app_server_import_scout` had not
returned before this handoff was written, so this handoff is based on the root
scout evidence below.

## Files Read

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/agents/core_dependency_map_scout.handoff.md`
- `.codex/workflow/agents/protocol_schema_scout.handoff.md`
- `.codex/workflow/agents/integration_order_scout.handoff.md`
- `codex-rs/Cargo.toml`
- `codex-rs/app-server/Cargo.toml`
- `codex-rs/app-server-protocol/Cargo.toml`
- `codex-rs/app-server/src/app_catalog_protocol.rs`
- `codex-rs/app-server/src/request_processors.rs`
- `codex-rs/app-server/src/request_processors/apps_processor.rs`
- `codex-rs/app-server/src/request_processors/thread_lifecycle.rs`
- `codex-rs/app-server/src/request_processors/thread_processor.rs`
- `codex-rs/app-server/src/request_processors/thread_summary.rs`
- `codex-rs/app-server/src/request_processors/turn_processor.rs`
- `codex-rs/app-server-protocol/src/protocol/thread_history.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/apps.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/turn.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/mcp.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/plugin.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/shared.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs`
- `codex-rs/app-server-protocol/src/protocol/common.rs`
- `codex-rs/app-server-protocol/src/protocol/item_builders.rs`
- `codex-rs/app/app-catalog-types/Cargo.toml`
- `codex-rs/app/app-catalog-types/src/lib.rs`
- `codex-rs/app/app-catalog-api/Cargo.toml`
- `codex-rs/app/app-catalog-api/src/lib.rs`
- `codex-rs/thread/thread-api/src/lib.rs`
- `codex-rs/thread/thread-manager-api/src/lib.rs`
- `codex-rs/thread/thread-handle-api/src/lib.rs`
- `codex-rs/thread/thread-store-api/Cargo.toml`
- `codex-rs/thread/thread-store-api/src/lib.rs`
- `codex-rs/thread/thread-store-api/src/store.rs`
- `codex-rs/thread/thread-store-api/src/types.rs`
- `codex-rs/thread/thread-store/Cargo.toml`
- `codex-rs/thread/thread-projection-api/Cargo.toml`
- `codex-rs/thread/thread-projection-api/src/lib.rs`
- `codex-rs/thread/thread-projection-api/src/page.rs`
- `codex-rs/thread/thread-projection-api/src/turn.rs`
- `codex-rs/core/Cargo.toml`
- `codex-rs/core/src/thread_manager.rs`
- `codex-rs/core/src/client.rs`
- `codex-rs/core/src/compact_remote.rs`
- `codex-rs/core/src/mcp_tool_call.rs`
- `codex-rs/core/src/realtime_conversation.rs`
- `codex-rs/core/src/session/mod.rs`

## Exact Searches Run

```powershell
rg -n "app-server|protocol|domain|slice|blocker|verification|commit|Phase|Phase 0|Phase 1|Phase 2|root-owned|owned|app[-_]server" .codex/workflow/solid-refactor-handoff.md .codex/workflow/solid-refactor-delegation-director-plan.md .codex/workflow/worker-delegation-commit-protocol.md .codex/workflow/agents/core_dependency_map_scout.handoff.md .codex/workflow/agents/protocol_schema_scout.handoff.md .codex/workflow/agents/integration_order_scout.handoff.md
rg -n "codex_core::|codex_app_server_protocol::|codex_app_(server|thread)_domain::|RequestPluginInstall|ConversationPathResponseEvent|ConversationHistoryResponseEvent|TurnStatus|ThreadHistory|McpInvocation|EventMsg|SessionConfiguredEvent" codex-rs/app-server/src codex-rs/app-server-protocol/src codex-rs/app-server-domain codex-rs/app-thread-domain
rg --files codex-rs/app-server-domain codex-rs/app-thread-domain codex-rs/app-server/src/request_processors codex-rs/app-server-protocol/src/protocol/v2 codex-rs/app-server-protocol/src/protocol | rg "(app-server-domain|app-thread-domain|request_processors|protocol[/\\]v2|thread_history|item_builders|common)"
rg --files codex-rs | rg "(domain|thread|app).*Cargo\.toml$"
rg -n 'codex-core|codex-protocol|codex-app-catalog|codex-thread|thread-' codex-rs/app-server/Cargo.toml codex-rs/app-server-protocol/Cargo.toml codex-rs/app/app-catalog-types/Cargo.toml codex-rs/app/app-catalog-api/Cargo.toml
rg --files codex-rs/thread codex-rs/app | rg '\.rs$'
rg -n "^use |pub struct|pub enum|pub type|impl From|build_turns_from_rollout_items|ThreadHistoryItem|TurnStatus|ConversationPathResponseEvent|ConversationHistoryResponseEvent" codex-rs/app-server-protocol/src/protocol/thread_history.rs
rg -n "^use |pub struct|pub enum|impl From|Thread|Turn|TurnStatus|Projected|TurnItem" codex-rs/app-server-protocol/src/protocol/v2/thread.rs codex-rs/app-server-protocol/src/protocol/v2/turn.rs
rg -n "^use |pub struct|pub enum|impl From|AppInfo|McpInvocation|RequestPluginInstall|Plugin|AppList" codex-rs/app-server-protocol/src/protocol/v2/apps.rs codex-rs/app-server-protocol/src/protocol/v2/mcp.rs codex-rs/app-server-protocol/src/protocol/v2/plugin.rs
Select-String -Path codex-rs/app-server-protocol/src/protocol/thread_history.rs -Pattern 'pub fn build_turns_from_rollout_items','impl ThreadHistoryBuilder','struct TurnBuilder','enum TurnStatus','pub struct ThreadHistoryFilter','pub enum OrderBy','TurnStatus::','Core'
Select-String -Path codex-rs/app-server-protocol/src/protocol/thread_history.rs -Pattern 'codex_protocol::protocol','Conversation','RolloutItem','ReviewDecision','Op','Event','ResponseItem','RequestPluginInstall','Mcp'
Select-String -Path codex-rs/app-server-protocol/src/protocol/thread_history.rs -Pattern 'pub struct ThreadTurn','pub enum ThreadItem','pub struct ThreadHistory','pub type ThreadHistoryItem','pub struct Turn'
Select-String -Path codex-rs/app-server-protocol/src/protocol/v2/thread.rs -Pattern '^use ','pub struct Thread','pub enum TurnStatus','pub struct Turn','pub type','impl From'
Select-String -Path codex-rs/app-server-protocol/src/protocol/v2/turn.rs -Pattern '^use ','pub struct','pub enum','impl From','CorePlan'
Select-String -Path codex-rs/app-server-protocol/src/protocol/v2/apps.rs -Pattern '^use ','pub struct','pub enum','impl From','AppInfo','AppSummary','McpInvocation','RequestPluginInstall'
Select-String -Path codex-rs/app-server/src/request_processors/thread_processor.rs -Pattern '^use ','ThreadHistory','build_turns','session_configured','rollout','ThreadManager','codex_'
Select-String -Path codex-rs/app-server/src/request_processors/turn_processor.rs -Pattern '^use ','TurnRequest','ThreadManager','ConversationManager','codex_','submit','TurnResponse','legacy'
Select-String -Path codex-rs/app-server/src/request_processors/apps_processor.rs,codex-rs/app-server/src/app_catalog_protocol.rs -Pattern '^use ','app_infos_to_v2','AppInfo','AppCatalog','list_accessible','plugin','AppListUpdated','AppSummary'
rg -n "fn .*thread|build_.*thread|list_threads|thread_history|build_turns|ThreadHistory|load_thread|rollout_path|ThreadItem|TurnItemsView|TurnStatus" codex-rs/app-server/src/request_processors/thread_processor.rs
rg -n "fn .*turn|Turn \{|TurnStatus|submit_core_op|Op::|ThreadItem|build_user_input|TurnResponse|TurnCreate|TurnSubmit|to_core" codex-rs/app-server/src/request_processors/turn_processor.rs
rg -n "fn .*app|app_infos_to_v2|AppsListResponse|AppListUpdated|AppSummary|AppInfo|list_accessible|CatalogProvider|From<" codex-rs/app-server/src/request_processors/apps_processor.rs codex-rs/app-server/src/app_catalog_protocol.rs
rg -n "^use codex_core|^use codex_protocol|codex_core::|codex_protocol::|impl From|Core|protocol::" codex-rs/app-server-protocol/src/protocol/common.rs codex-rs/app-server-protocol/src/protocol/item_builders.rs codex-rs/app-server-protocol/src/protocol/v2/shared.rs codex-rs/app-server-protocol/src/protocol/v2/mcp.rs codex-rs/app-server-protocol/src/protocol/v2/plugin.rs codex-rs/app-server-protocol/src/protocol/v2/permissions.rs codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs
rg -n "codex_core::|codex_protocol::|codex_app_server_protocol::|protocol::v2|From<.*Core|to_core|from_core" codex-rs/app-server/src --glob '*.rs'
rg -n "codex_core::|codex_protocol::|codex_app_server_protocol::" codex-rs/app-server-protocol/src --glob '*.rs'
rg -n "codex_core::|codex-core" codex-rs/app-server-protocol/src codex-rs/app-server-protocol/Cargo.toml
rg -n "codex_core::protocol|codex_protocol::protocol|codex_protocol::models|codex_protocol::user_input|codex_protocol::plan_tool" codex-rs/app-server-protocol/src/protocol/v2/thread.rs codex-rs/app-server-protocol/src/protocol/v2/turn.rs codex-rs/app-server-protocol/src/protocol/thread_history.rs codex-rs/app-server-protocol/src/protocol/item_builders.rs
rg -n "codex_app_catalog_types|codex_thread|thread_projection|ProjectionPage|ProjectedTurn|TurnStatus|AppInfo" codex-rs/app-server/src codex-rs/app-server-protocol/src codex-rs/thread codex-rs/app
rg -n "ThreadHistoryBuilder|TurnStatus|StoredTurn|Thread|ThreadItem|codex_app_server_protocol|codex_thread_store_api|RolloutItem|read_rollout" codex-rs/app-server/src/request_processors.rs codex-rs/app-server/src/request_processors/thread_summary.rs codex-rs/app-server/src/request_processors/thread_processor.rs
rg -n "pub enum StoredTurnStatus|pub struct StoredThread|pub struct StoredTurn|pub struct StoredThreadPage|pub struct ListThreadsParams|pub struct ReadThreadParams|pub struct ThreadStore" codex-rs/thread/thread-store-api/src/types.rs codex-rs/thread/thread-store-api/src/store.rs
rg -n "pub enum TurnStatus|pub struct ProjectedTurn|pub struct ProjectedThread|pub type ThreadHistoryProjection|pub struct ProjectedTurnError|pub enum TurnItemsView|pub struct ProjectionPage|pub enum ProjectionSortDirection" codex-rs/thread/thread-projection-api/src/turn.rs codex-rs/thread/thread-projection-api/src/page.rs codex-rs/thread/thread-projection-api/src/lib.rs
rg -n "thread-projection|thread-store-api|thread-manager-api|thread-api" codex-rs/Cargo.toml
rg -n "codex-thread-projection-api|thread-projection-api" codex-rs --glob Cargo.toml
rg -n 'thread/thread-projection-api|thread/thread-store-api|app/app-catalog' codex-rs/Cargo.toml
rg -n 'codex-thread-projection-api|thread-projection-api' MODULE.bazel MODULE.bazel.lock Cargo.Bazel.lock BUILD.bazel
rg -n 'codex-app-catalog-types|codex-thread-store-api|codex-thread-projection-api' MODULE.bazel MODULE.bazel.lock Cargo.Bazel.lock BUILD.bazel
rg --files | rg '(^|[\\/])(MODULE\.bazel|BUILD\.bazel|.*\.bazel)$'
$files = rg --files -g BUILD.bazel -g MODULE.bazel -g MODULE.bazel.lock; if ($files) { rg -n 'codex-thread-projection-api|thread-projection-api|codex-app-catalog-types|codex-thread-store-api' $files }
rg -n -C 3 "app-server|app_server|protocol|ThreadHistoryBuilder|TurnStatus|MCP elicitation|RequestPluginInstall|core.*protocol|codex_app_server_protocol" .codex/workflow/solid-refactor-handoff.md
rg -n -C 3 "app-server|app_server|protocol|ThreadHistoryBuilder|TurnStatus|MCP elicitation|RequestPluginInstall|core.*protocol|codex_app_server_protocol" .codex/workflow/solid-refactor-delegation-director-plan.md
rg -n -C 3 "app-server|app_server|protocol|ThreadHistoryBuilder|TurnStatus|MCP elicitation|RequestPluginInstall|core.*protocol|codex_app_server_protocol|commit|pathspec" .codex/workflow/worker-delegation-commit-protocol.md .codex/workflow/agents/core_dependency_map_scout.handoff.md .codex/workflow/agents/protocol_schema_scout.handoff.md .codex/workflow/agents/integration_order_scout.handoff.md
rg -n "StoredTurnStatus|StoredThread|thread_from_stored_thread|status:|ThreadStatus|TurnStatus" codex-rs/app-server/src/request_processors/thread_processor.rs codex-rs/app-server/src/request_processors/thread_summary.rs codex-rs/app-server/src/request_processors/thread_lifecycle.rs
rg -n "fn thread_from_stored_thread|pub\(crate\) fn thread_from_stored_thread|turns:" codex-rs/app-server/src/request_processors/thread_processor.rs codex-rs/app-server/src/request_processors/thread_summary.rs
rg -n "codex_app_server_protocol|codex-app-server-protocol" codex-rs/core codex-rs/protocol codex-rs/thread codex-rs/app codex-rs/runtime-domain codex-rs/context-domain --glob '*.rs' --glob Cargo.toml
rg -n "codex_core::|codex-core" codex-rs/app codex-rs/thread codex-rs/app-server-protocol --glob '*.rs' --glob Cargo.toml
rg -n "codex_app_server_protocol|codex-app-server-protocol" codex-rs --glob '*.rs' --glob Cargo.toml
rg -n "ThreadHistoryBuilder|TurnStatus|codex_app_server_protocol|build_turns|turn.status" codex-rs/core/src/thread_manager.rs codex-rs/core/src/mcp_tool_call.rs codex-rs/core/src/session/mod.rs codex-rs/core/src/client.rs codex-rs/core/src/compact_remote.rs codex-rs/core/src/realtime_conversation.rs
rg -n "codex-app-server-protocol|codex-thread-projection-api|codex-thread-store-api" codex-rs/core/Cargo.toml codex-rs/app-server-protocol/Cargo.toml codex-rs/thread/thread-projection-api/Cargo.toml
rg -n "snapshot_turn_state|SnapshotTurnState|ends_mid_turn|active_turn_start_index|ThreadHistoryBuilder" codex-rs/core/src/thread_manager.rs codex-rs/core/src --glob '*.rs'
rg -n "active_turn_snapshot|active_turn_id_if_explicit|has_active_turn|finish_active_turn|start_turn|PendingTurn|TurnComplete|TurnInterrupted|ErrorEvent" codex-rs/app-server-protocol/src/protocol/thread_history.rs
rg -n "ThreadHistoryBuilder|build_turns_from_rollout_items|TurnStatus::InProgress|active_turn" codex-rs/app-server-protocol/src/protocol/thread_history.rs codex-rs/app-server/src/request_processors codex-rs/core/src/thread_manager.rs
```

Notes on failed/negative searches:

- `rg -n "codex_core::|codex-core" codex-rs/app-server-protocol/src codex-rs/app-server-protocol/Cargo.toml` returned no matches. The protocol crate leaks `codex-protocol` types, not `codex-core` directly.
- Searches for `codex-rs/app-server-domain` and `codex-rs/app-thread-domain` failed because those directories do not exist in this worktree. The relevant domain/API crates are under `codex-rs/app/` and `codex-rs/thread/`.
- PowerShell glob searches such as `rg ... codex-rs/thread/*/Cargo.toml` failed with Windows path glob errors; follow-up `rg --files` searches were used instead.

## Current Boundary Findings

1. The direct core leak that best matches this rescue lane is
   `codex-rs/core/src/thread_manager.rs` importing
   `codex_app_server_protocol::ThreadHistoryBuilder` and
   `codex_app_server_protocol::TurnStatus`. The imports are used only by
   `snapshot_turn_state`, where core replays `InitialHistory` rollout items,
   asks the builder for active-turn state, and compares the active turn status
   to `TurnStatus::InProgress`.

2. `ThreadHistoryBuilder` lives in
   `codex-rs/app-server-protocol/src/protocol/thread_history.rs`, but it is not
   just a wire DTO helper. It imports many `codex_protocol::protocol::*` event
   types, consumes `RolloutItem`, tracks the active turn, builds protocol
   `Turn` values, and exposes active-turn inspection methods:
   `active_turn_snapshot`, `has_active_turn`, `active_turn_id_if_explicit`, and
   `active_turn_start_index`.

3. `codex-rs/thread/thread-projection-api` already states the intended
   ownership direction: app-server-neutral thread projection types, with wire
   DTOs converting to/from them instead of app-server protocol types crossing
   ownership boundaries. It defines a matching `TurnStatus` and projected turn
   structs. However, it is not wired as a workspace dependency in
   `codex-rs/Cargo.toml` and no Bazel target/search hit exists for
   `codex-thread-projection-api`.

4. `codex-rs/app-server/src/request_processors.rs` still has an app-server
   owned wrapper, `build_api_turns_from_rollout_items`, that filters persisted
   rollout items and delegates to `ThreadHistoryBuilder`. This is acceptable as
   an edge adapter for now. Moving the full builder immediately would combine
   rollout replay, protocol item construction, and thread projection in one
   larger root edit.

5. The app catalog seam is already heading in the correct direction:
   `codex-rs/app/app-catalog-types` owns app catalog data, and
   `codex-rs/app-server/src/app_catalog_protocol.rs` maps domain catalog data to
   `codex_app_server_protocol` wire DTOs. That path appears to belong to the
   existing app catalog lane, not this rescue slice.

6. Other direct `codex-core` -> `codex-app-server-protocol` leaks remain but are
   separate ownership lanes:
   `AuthMode` in client/realtime/compact paths and MCP elicitation DTOs in
   `mcp_tool_call`/session paths. The required prior handoffs already call out
   auth and MCP elicitation as separate slices; this rescue should not mix them
   with thread projection.

## Recommended Path-Owned Implementation Slice

Slice name: `thread_active_turn_projection_boundary`.

Goal: remove the direct `codex-core` dependency on app-server protocol for
thread active-turn state without changing app-server wire schema or expanding
`codex-core`.

Smallest non-overlapping implementation:

1. Wire `codex-rs/thread/thread-projection-api` as a real workspace crate and
   dependency target owned by root. This is required before core can depend on
   it.
2. Add a small active-turn rollout tracker to `codex-thread-projection-api`
   instead of moving the full `ThreadHistoryBuilder`. The tracker should consume
   `codex_protocol::protocol::RolloutItem` and expose only the state core needs:
   active turn present, explicit active turn id, active turn start index, and
   domain `TurnStatus`.
3. Update `codex-rs/core/src/thread_manager.rs` to use the new
   `codex_thread_projection_api` tracker and `TurnStatus`, deleting the
   `codex_app_server_protocol::{ThreadHistoryBuilder, TurnStatus}` imports.
4. Leave `codex-rs/app-server-protocol/src/protocol/thread_history.rs` and
   app-server `build_api_turns_from_rollout_items` intact in this slice. That
   keeps the wire DTO builder and UI item construction stable while removing
   the highest-value core leak.
5. Add focused tests for the new tracker using the current core
   `snapshot_turn_state` scenarios and the relevant active-turn cases already
   covered by protocol thread-history tests.

This slice is deliberately smaller than moving `ThreadHistoryBuilder`. It
creates the correct domain home and removes core's protocol dependency first;
the later larger cleanup can deduplicate the protocol builder against the
domain tracker after thread-store and app-catalog lanes are green.

## Files That Must Be Root-Owned

Root-owned for the recommended slice:

- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- `MODULE.bazel.lock`
- any `BUILD.bazel` or Bazel crate target files required for
  `codex-thread-projection-api`
- `codex-rs/core/Cargo.toml`
- `codex-rs/core/src/thread_manager.rs`
- `codex-rs/thread/thread-projection-api/Cargo.toml`
- `codex-rs/thread/thread-projection-api/src/lib.rs`
- `codex-rs/thread/thread-projection-api/src/turn.rs`
- new `codex-rs/thread/thread-projection-api/src/rollout_state.rs` or similar

Do not include these unrelated/overlapping lanes in the same commit:

- `codex-rs/app/app-catalog-*`
- `codex-rs/app-server/src/app_catalog_protocol.rs`
- `codex-rs/app-server/src/request_processors/apps_processor.rs`
- auth `AuthMode` cleanup paths
- MCP elicitation DTO cleanup paths
- generated app-server schema fixtures, unless a later slice intentionally
  changes a wire DTO shape

## Blockers

- The worktree is already dirty in the same broad areas, including modified
  app-server files and untracked app/thread domain crates. Do not stage broad
  paths or overwrite those edits.
- `codex-thread-projection-api` exists but is not fully wired in
  `codex-rs/Cargo.toml` workspace dependency lines and had no Bazel search hit.
  Manifest/Bazel/lock work must be owned by root.
- The integration-order handoff recommends landing the thread-store boundary
  before broader thread projection work. This recommended slice is narrow enough
  to be next after root clears the current manifest/thread-store compile
  blockers; it should not be attempted as an uncoordinated worker edit.
- I did not run Cargo, Just, formatters, schema generation, or tests by design.

## Verification Lane

After implementing the recommended slice:

1. Run `just fmt` in `codex-rs`.
2. If manifests/dependencies changed, run `just bazel-lock-update` and
   `just bazel-lock-check` from the repo root.
3. Run the focused release lane:
   `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Filter snapshot_turn_state`
4. Run the new projection crate's focused release tests once it is wired:
   `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-thread-projection-api`
5. Run `just fix -p codex-core` in `codex-rs` before finalizing a large Rust
   change.
6. Verify the specific leak is gone:
   `rg -n "codex_app_server_protocol::(ThreadHistoryBuilder|TurnStatus)|use codex_app_server_protocol::(ThreadHistoryBuilder|TurnStatus)" codex-rs/core/src/thread_manager.rs`

Expected result for step 6: no matches. Broader core searches will still show
`AuthMode` and MCP elicitation app-server protocol imports until their separate
lanes land.

No app-server schema regeneration should be required for this slice if the wire
DTOs remain unchanged.

## Commit Readiness And Pathspec

Implementation commit readiness: not ready from this rescue scout alone. The
recommended implementation needs root-owned manifest/Bazel/lock edits and
focused release verification.

This handoff-only path is ready to stage if root wants to commit scout notes:

```powershell
git add -- .codex/workflow/agents/app_server_boundary_rescue.handoff.md
```

If the recommended implementation is later completed and verified, use explicit
pathspec staging only, along these lines:

```powershell
git add -- `
  codex-rs/Cargo.toml `
  codex-rs/Cargo.lock `
  MODULE.bazel.lock `
  codex-rs/core/Cargo.toml `
  codex-rs/core/src/thread_manager.rs `
  codex-rs/thread/thread-projection-api/Cargo.toml `
  codex-rs/thread/thread-projection-api/src/lib.rs `
  codex-rs/thread/thread-projection-api/src/turn.rs `
  codex-rs/thread/thread-projection-api/src/rollout_state.rs
```

Add any required `BUILD.bazel` files explicitly if Bazel wiring changes. Do not
use `git add .`, and do not include app-catalog, auth, MCP elicitation, schema
fixture, or snapshot paths in this slice unless root deliberately expands the
scope.
