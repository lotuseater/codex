# Slice: appserver-analytics — merge resolution progress

Status: DONE (all 4 files resolved, 0 markers, git diff --check clean)

## Files resolved
- codex-rs/analytics/src/facts.rs — TAKE-UPSTREAM on the one conflicted struct.
  Conflict was `TurnCodexError` struct only. Upstream DELETED the `subreason` field
  (and its INVALID_REQUEST_SUBREASON_* constants + the long from_codex_err body, all
  of which merged cleanly to upstream's deletion OUTSIDE the conflict). Kept upstream's
  field set (kind + http_status_code, NO subreason) but with `pub` visibility to match
  the fork-refactored file (whole file is `pub`, fork's AnalyticsFact AppServer/Custom
  redesign already merged). NOT keeping `subreason`: the constructor below the conflict
  (already merged to upstream short form) never populates it -> would be a compile error,
  and subreason is an upstream-origin field, not a fork feature. fork features
  forked_from_thread_id + subagent_source live in SubAgentThreadStartedInput (untouched,
  preserved). Note: codex-rs/analytics-appserver/src/events.rs still refs
  codex_error_subreason — owned by ANOTHER resolver, their concern.
- codex-rs/app-server/README.md — UNION (trivial). Fork side empty; took upstream's
  runtimeWorkspaceRoots comment + selectedCapabilityRoots example block.
- codex-rs/app-server/src/request_processors/external_agent_config_processor.rs — UNION.
  Kept ALL fork external-agent imports (ImportedExternalAgentSession, PendingSessionImport,
  prepare_validated_session_imports, record_imported_session, ThreadId, InitialHistory,
  ThreadMetadataPatch via codex_thread_store_api) + upstream's new
  `use codex_thread_store::ThreadStore;`. All symbols are used in the file; no dup.
- codex-rs/app-server/src/mcp_refresh.rs — UNION/structural in a test. Kept fork's 3-arg
  thread_store_from_config(config, thread_store_selection(config), state_db) call (matches
  the CURRENT factory.rs signature; upstream's 2-arg form is stale) AND upstream's new
  environment_manager + executor_skill_provider setup (BOTH are referenced by the merged
  ThreadManager::new below the conflict at lines 210/218 -> required).

## Gotchas
- factory.rs thread_store_from_config is now 3-arg (config, selection, state_db); the
  2-arg call form anywhere is wrong/stale.
- analytics/facts.rs whole file is the fork's `pub` redesign; keep `pub` not `pub(crate)`.

MARKERS_REMAINING: 0
