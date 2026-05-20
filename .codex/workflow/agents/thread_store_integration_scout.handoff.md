# thread_store_integration_scout Handoff

Status: completed read-only scout.

Date: 2026-05-20

## Scope

Read-only inspection of thread-store, app-server thread processors, and core
callers that consume thread metadata, summaries, pagination, or projected turn
data. No source, manifest, lockfile, generated-file, or other handoff edits were
made except this handoff.

Required inputs read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/thread_store_boundary.handoff.md`
- `.codex/workflow/agents/thread_projection_boundary.handoff.md`
- `.codex/workflow/agents/compile_session_store_scout.handoff.md`

The queued `compile_session_store_scout` handoff still only says "queued by
root", so there were no compile-scout findings to integrate here. Existing
subagents `app_server_thread_scout` and `core_thread_consumer_scout` were still
running and did not return results before this handoff was written.

## Exact Callsites That Still Need Integration

### Manifest / Boundary Wiring

- `codex-rs/Cargo.toml` already lists `thread/thread-store-api` in the
  workspace and `codex-thread-store-api` in workspace dependencies, but
  `codex-rs/core/Cargo.toml` and `codex-rs/app-server/Cargo.toml` do not yet
  declare `codex-thread-store-api = { workspace = true }` even though source in
  both crates imports `codex_thread_store_api`.
- `codex-thread-projection-api` is not in the workspace dependency graph yet.
  Add the crate to `codex-rs/Cargo.toml` and wire it only into the crates that
  actually convert stored rollout history into app-server DTOs.

### Thread-Store Data Production

- `codex-rs/thread/thread-store/src/local/read_thread.rs:356` builds a
  `StoredThread` from rollout-file-only metadata with placeholders:
  `preview: String::new()`, `name: None`, default/fallback provider fields,
  read-only sandbox/approval defaults, `token_usage: None`,
  `first_user_message: None`, and `history: None`.
  This path must derive real values from rollout metadata/history when SQLite
  metadata is absent, or app-server/core consumers downstream will faithfully
  carry placeholders.
- `codex-rs/thread/thread-store/src/local/read_thread.rs:267` builds
  `StoredThread` from SQLite `ThreadMetadata` and already carries richer fields
  such as preview/title, cwd, source, thread_source, agent nickname/role,
  git_info, first_user_message, token_usage, and history. Treat this as the
  shape rollout-only reads should converge toward.
- `codex-rs/thread/thread-store-api/src/types.rs` `StoredThread` carries
  metadata required by app-server `Thread`: thread id, rollout path,
  fork source, preview/name, model/provider/reasoning, timestamps, archive
  timestamp, cwd, cli version, source/thread_source, agent nickname/role,
  git_info, base instructions, dynamic tools, token usage, first user message,
  and optional history. Do not reduce this to id/path/history at integration
  callsites.

### App-Server Thread Processor

- `codex-rs/app-server/src/request_processors/thread_processor.rs:1841`
  (`thread_list_response_inner`) calls `list_threads_common`, which forwards
  filters/pagination to `ThreadStore::list_threads` at `:3379` and then maps
  each `StoredThread` through `thread_from_stored_thread` at `:1892`.
  Keep this path store-backed; do not reintroduce `StateDb`-only listing.
  Ensure the mapper consumes every real `StoredThread` field and does not
  replace absent rollout-only fields with local defaults.
- `thread_processor.rs:3790` (`thread_from_stored_thread`) is the central
  `StoredThread -> Thread` adapter. It maps most fields, but synthesizes
  `session_id` from `thread_id` and leaves `turns: Vec::new()` at `:3838`.
  Empty turns are acceptable only for metadata/list views. `thread/read`
  with turns and turn pagination must populate turns through the projection
  boundary.
- `thread_processor.rs:4005` (`build_thread_from_snapshot`) builds live-only
  `Thread` values with placeholders: `forked_from_id: None`, blank preview,
  `git_info: None`, `name: None`, and empty turns at `:4015-4031`.
  `build_thread_from_loaded_snapshot` at `:4035` uses this path. Replace this
  with a store-backed/live-handle `StoredThread` read when possible, or add
  core accessors that return the real live metadata before using placeholders.
- `thread_processor.rs:2103` (`load_live_thread_view`) merges persisted
  metadata when it exists, but if `persisted_thread` is `None` it falls back to
  `build_thread_from_loaded_snapshot`. That keeps the placeholder problem alive
  for live metadata-only reads before persistence materializes.
- `thread_processor.rs:2133` (`apply_thread_read_store_fields`) loads live
  history and then calls `build_api_turns_from_rollout_items` at `:2147`.
  `codex-rs/app-server/src/request_processors.rs:521` filters with
  `EventPersistenceMode::Limited`, so extended event data needed for richer
  projection can be dropped. Move this conversion behind the
  `codex-thread-projection-api` boundary and let the requested `items_view`
  decide how much detail is retained.
- `thread_processor.rs:2153` (`thread_turns_list_response_inner`) loads all
  history, reconstructs turns locally at `:2193`, mutates item detail by
  `TurnItemsView` at `:2201`, and paginates after reconstruction. This should
  become a projection/page adapter over `ThreadStore::list_turns` /
  `list_items` (or an equivalent projection service) so pagination cursors,
  turn status, errors, and item detail are owned by the new boundary.
- `thread_processor.rs:627` returns method-not-found for
  `thread/turns/items/list`. Implement this against `ThreadStore::list_items`
  plus projection DTO conversion; otherwise item-level pagination remains
  absent even though the API surface exists.
- `thread_processor.rs:2249` (`load_thread_turns_list_history`) asks
  `ThreadStore::read_thread(... include_history: true)` and then requires
  `StoredThread.history`. That couples turn listing to full-history reads.
  Future workers should switch this callsite to the page APIs or projection
  service instead of loading the whole rollout for every page.
- `thread_processor.rs:3305` (`get_thread_summary_response_inner`) reads a
  store thread by id/path and maps it with `summary_from_stored_thread` at
  `:3843`. `summary_from_stored_thread` currently uses
  `thread.rollout_path.unwrap_or_default()`. Do not let a missing path become
  an empty path silently; either make summary path optional in the DTO adapter
  or preserve/store the real rollout path before summary mapping.
- `thread_processor.rs:2854` (`read_stored_thread_for_resume`) returns only
  `InitialHistory::Resumed`, so it discards the `StoredThread` metadata it just
  read. `thread_resume_response_inner` then reaches into `StateDb` around
  `:2671` for persisted metadata. Change this callsite to return a small
  resume read result containing both `InitialHistory` and `StoredThread`
  metadata, and feed that into `merge_persisted_resume_metadata`.
- `codex-rs/app-server/src/request_processors/turn_processor.rs:944` and
  `codex-rs/app-server/src/bespoke_event_handling.rs:1530` also call
  `thread_from_stored_thread`. Any mapper fix must cover these review-thread
  and rollback paths, not only `thread_processor.rs`.

### Core Consumers

- `codex-rs/core/src/session/mod.rs:571` reads persisted dynamic tools via
  `ThreadStore::read_thread_dynamic_tools` when a resumed/forked thread starts
  without explicit tools. The current call at `:579` uses `.unwrap_or(None)`,
  which drops store read failures into the same path as "no persisted tools".
  Preserve the distinction where possible, and do not default to an empty tool
  set when the store can provide real thread-start tools.
- `codex-rs/core/src/session/mod.rs:851` (`thread_title_from_thread_store`)
  reads `StoredThread` and derives a title from `name` and `preview` at
  `:876-877`. This callsite depends directly on the store carrying real
  `name` and `preview`; rollout-only placeholder values will suppress titles.
- `codex-rs/core/src/thread_manager.rs:822` reads a thread by rollout path with
  `include_history: true` and immediately calls
  `stored_thread_to_initial_history`.
- `core/src/thread_manager.rs:1308` (`stored_thread_to_initial_history`)
  consumes only `thread_id`, `history.items`, and `rollout_path`. It drops
  metadata, base instructions, dynamic tools, source/thread_source, and config
  fields. If core owns any future store-level resume/fork flow, add a richer
  resume/fork handoff type rather than reducing a `StoredThread` to
  `InitialHistory`.
- `core/src/thread_manager.rs:599-620` subagent fork-by-thread-id also reads a
  `StoredThread` and reduces it through `stored_thread_to_initial_history`.
  Preserve fork source id, rollout path, dynamic tools, base instructions, and
  metadata if subagents must inherit the original thread environment exactly.

## Data Fields That Must Not Be Dropped

Carry these fields through the boundary when the source has them:

- Identity and lineage: `thread_id`, real `session_id` if available,
  `forked_from_id`, `rollout_path`.
- Display metadata: `preview`, `name`, `first_user_message`.
- Model/config metadata: `model_provider`, `model`, `reasoning_effort`,
  `approval_policy` / `sandbox_policy`, memory mode, `base_instructions`,
  `dynamic_tools`.
- Environment metadata: `cwd`, workspace roots if/when exposed, `cli_version`,
  `source`, `thread_source`, `agent_nickname`, `agent_role`.
- Repository metadata: git commit sha, branch, origin URL.
- Time/archive metadata: `created_at`, `updated_at`, `archived_at`.
- Projection data: full persisted `RolloutItem` history when requested,
  turn ids, turn status, turn errors including `CodexErrorInfo`,
  requested `items_view`, item pagination cursors, and backwards cursors.

## Suggested Ownership Split

- **Store worker**: owns `codex-rs/thread/thread-store-api/**` and
  `codex-rs/thread/thread-store/src/local/**`. Fill rollout-only
  `StoredThread` placeholders, verify `list_threads`, `read_thread`,
  `read_thread_by_rollout_path`, `read_thread_dynamic_tools`, metadata update,
  archive/unarchive, and page APIs return real boundary data.
- **App-server adapter worker**: owns
  `codex-rs/app-server/src/request_processors/thread_processor.rs`,
  `request_processors.rs`, `request_processors/thread_lifecycle.rs`,
  `turn_processor.rs`, `bespoke_event_handling.rs`, and affected tests.
  Replace live snapshot placeholders, make resume read return metadata, and
  convert `StoredThread` / projection pages into v2 DTOs without reaching back
  into concrete storage.
- **Projection/protocol worker**: owns
  `codex-rs/thread/thread-projection-api/**`,
  `codex-rs/app-server-protocol/src/protocol/thread_history.rs`,
  `v2/thread.rs`, `v2/thread_data.rs`, `v2/item.rs`, and schema/snapshot tests.
  Wire `codex-thread-projection-api`, move neutral turn/page projection out of
  protocol DTO code, and keep protocol-only JSON/TS/schema policy in
  app-server-protocol.
- **Core consumer worker**: owns `codex-rs/core/src/session/mod.rs` and
  `codex-rs/core/src/thread_manager.rs`. Preserve dynamic tools/title metadata
  and avoid collapsing `StoredThread` to `InitialHistory` where resume/fork
  needs richer data.

## Commit Readiness Notes

- This handoff is read-only research plus one Markdown update; no Cargo, Just,
  fmt, generated schema, staging, commit, or source edit was run.
- Not commit-ready as an implementation slice yet: source currently imports
  `codex_thread_store_api` from crates whose manifests still need direct
  dependencies, and `codex-thread-projection-api` is still unwired.
- A future implementation commit should be split by the ownership groups above
  and verified with the smallest release-profile crate tests for the touched
  crates. If protocol shapes change, regenerate app-server schemas and include
  the generated fixture updates in the same verified slice.
