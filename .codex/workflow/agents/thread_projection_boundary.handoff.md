# thread_projection_boundary Handoff

## Status

Implemented a new, unwired `codex-thread-projection-api` crate under the owned
`codex-rs/thread/**` lane. The crate owns app-server-neutral turn projection
types and cursor page shapes; it does not import `codex-app-server-protocol`,
`codex-core`, or thread store implementation crates.

## Paths Changed

- `codex-rs/thread/thread-projection-api/Cargo.toml`
- `codex-rs/thread/thread-projection-api/src/lib.rs`
- `codex-rs/thread/thread-projection-api/src/page.rs`
- `codex-rs/thread/thread-projection-api/src/turn.rs`
- `.codex/workflow/agents/thread_projection_boundary.handoff.md`

## Paths Read

- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/app-server-protocol/src/protocol/thread_history.rs`
- `codex-rs/app-server-protocol/src/protocol/item_builders.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/thread.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/turn.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/item.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/shared.rs`
- `codex-rs/app-server/src/thread_state.rs`
- `codex-rs/app-server/src/request_processors.rs`
- `codex-rs/app-server/src/bespoke_event_handling.rs`
- `codex-rs/thread/thread-api/Cargo.toml`
- `codex-rs/thread/thread-api/src/lib.rs`
- `codex-rs/thread/thread-store-api/Cargo.toml`
- `codex-rs/thread/thread-store-api/src/lib.rs`
- `codex-rs/thread/thread-store-api/src/types.rs`

## Exact App-Server-Protocol Types Involved

- `ThreadHistoryBuilder`
- `build_turns_from_rollout_items`
- `Thread`
- `GitInfo`
- `Turn`
- `TurnStatus`
- `TurnItemsView`
- `TurnError`
- `ThreadItem`
- `ThreadTurnsListParams`
- `ThreadTurnsListResponse`
- `ThreadTurnsItemsListParams`
- `ThreadTurnsItemsListResponse`
- `SortDirection`
- `CodexErrorInfo`
- `ThreadStatus`, `SessionSource`, and `ThreadSource` are part of the wider
  `Thread` DTO shape, but this slice intentionally kept ownership to turn
  history projection instead of moving the whole thread metadata wire object.

## Crate Ownership Recommendation

- `codex-thread-projection-api` should own neutral thread-history projection
  concepts: `ProjectedTurn<TurnItem>`, `TurnStatus`, `TurnItemsView`,
  `ProjectedTurnError`, `ProjectedThread<TurnItem>`, turn listing params, and
  cursor page shapes.
- Keep item payloads generic at this boundary. The current app-server
  `ThreadItem` enum is a large wire/presentation DTO and should remain at the
  app-server protocol edge until root decides whether to create a separate
  neutral item-projection type or inject item projection into the history
  reducer.
- `ThreadHistoryBuilder` can move out of `codex-app-server-protocol` only after
  its direct dependencies on app-server `ThreadItem`, app-server item builders,
  and app-server `CodexErrorInfo` conversion are split or adapted. A good next
  integration shape is `ThreadHistoryBuilder<ProjectedItem>` plus an injected
  event-to-item projector owned outside app-server protocol.
- App-server protocol should remain responsible for `JsonSchema`, `TS`, v2 wire
  field policy, and conversion between v2 DTOs and projection API types.

## Root-Owned Manifest Entries Needed

- Add workspace member in `codex-rs/Cargo.toml`:
  `thread/thread-projection-api`.
- Add workspace dependency in `codex-rs/Cargo.toml`:
  `codex-thread-projection-api = { path = "thread/thread-projection-api" }`.
- Add `codex-thread-projection-api = { workspace = true }` to root-selected
  downstream crate manifests when integrating, likely starting with
  `codex-rs/app-server-protocol/Cargo.toml` and/or
  `codex-rs/app-server/Cargo.toml`.
- Add or regenerate Bazel targets/lockfiles if this repo's Bazel lane requires
  them for the new crate.
- No root manifest, lockfile, or Bazel file was edited in this lane.

## Verification Performed

- `rg --files codex-rs/thread/thread-projection-api`
  - Confirmed only `Cargo.toml`, `src/lib.rs`, `src/page.rs`, and `src/turn.rs`
    were added under the new crate.
- `rg -n "codex_app_server_protocol|codex-app-server-protocol|codex_core|codex-core|codex_thread_store|codex-thread-store|codex_app_server|codex-app-server" codex-rs/thread/thread-projection-api`
  - No matches. `rg` exited `1`, which is expected for no matches.
- `rg -n "codex-thread-projection-api|codex-protocol|serde" codex-rs/thread/thread-projection-api/Cargo.toml codex-rs/thread/thread-projection-api/src`
  - Confirmed the new crate depends only on workspace `codex-protocol` and
    `serde`.
- No Cargo, Just, formatter, broad build, Git, staging, reset, checkout, root
  manifest, lockfile, Bazel, or app-server protocol edits were performed.

## Blockers

- The new crate is intentionally unwired. Root must add workspace membership and
  downstream dependencies before Cargo/Bazel compilation can validate it.
- Moving `ThreadHistoryBuilder` itself is blocked by its current construction of
  app-server `ThreadItem` variants and use of app-server protocol item-builder
  helpers. That should be handled by a root-approved integration slice with
  exact file grants.
- No commit was made because this lane explicitly forbids Git operations and the
  useful slice requires root-owned manifest wiring before normal verification.
