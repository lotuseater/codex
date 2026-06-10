# proto-config slice progress

## Guard checks
- toplevel_ok = true (C:/Users/Oleh/Documents/GitHub/open_ai/codex)
- unmerged_seen = true (all 6 files stages 1/2/3 present)

## Files
- [DONE] codex-rs/protocol/src/config_types.rs — take-fork structural. Fork moved whole
  content into `codex_config_types` crate; file is now `pub use codex_config_types::*;`.
  Upstream's only change (guardian_subagent->auto_review rename) lives in config-types
  crate (owned by another resolver). Resolved by taking fork re-export.
- [DONE] codex-rs/app-server-protocol/src/protocol/common.rs — take-fork structural.
  Fork split common.rs into submodules (auth/client_requests/fuzzy_file_search/
  notifications/server_requests), file is now the aggregator. UPSTREAM ADDED
  `AuthMode::PersonalAccessToken` variant + `has_chatgpt_account()` impl — belongs in
  fork's common/auth.rs (clean, outside my slice). FLAGGED under files_uncertain.
- [DONE] codex-rs/app-server-protocol/src/protocol/thread_history.rs — take-fork structural.
  Fork split into thread_history/{builder,event_handlers,tests}. Took fork stage-2 content
  (wrote `git show :2:` to file). UPSTREAM ADDED SubAgentActivity handling (match arm +
  handle_sub_agent_activity method) — belongs in fork's thread_history/event_handlers.rs
  (clean, outside my slice). FLAGGED under files_uncertain.
- [DONE] codex-rs/config/src/config_requirements.rs — union/take-upstream. 3 conflicts:
  new test fns (deserialize_managed_permission_profiles etc.) + 2 struct-literal field adds
  (allowed_permission_profiles/default_permissions). Struct defs already have those fields.
- [DONE] codex-rs/app-server-protocol/src/protocol/v2/tests.rs — structural. Fork split tests
  into v2/tests/{...} submodules. Conflict1: kept fork mod decls + appended the ONE upstream
  inline test not relocated (thread_sources_round_trip_as_scalar_labels) under fork-local
  banner; other 3 upstream inline tests already in submodules (would be dup). Conflict2:
  took fork (empty) — ALL 61 upstream inline tests already exist in submodules (verified by
  fn-name comm). No coverage lost.
- [IN PROGRESS] codex-rs/protocol/src/protocol.rs — structural+union. Fork split into
  protocol/ submodules (op/event_msg/realtime_session/...). Method: per-conflict, take fork
  for relocated content, UNION genuinely-new upstream items. Verified genuinely-new items via
  item-name comm against fork submodules+config-types crate.
  Done conflicts: C1 (kept fork re-exports + NEW upstream TurnEnvironmentSelections plural;
  dropped dup GitSha/singular). C2 (took fork empty - all realtime structs relocated incl
  wire_name in config-types). C3a (union cwd + new environments field). C3b (kept fork side -
  serde-skip attrs over upstream doc comments, same fields).
  [DONE] all 10 conflicts. 0 markers. Resolutions:
  - C4 (inline Op enum): took fork empty - all 25 Op variants already in fork op.rs.
  - C5 (From<Vec<UserInput>> for Op): took fork empty - already in fork op.rs:433.
  - C6 (inline EventMsg variants): kept fork re-export RealtimeConversationVersion. The 74
    upstream EventMsg variants all in fork event_msg.rs EXCEPT SubAgentActivity (see CROSS-SLICE).
  - C7 (TokenUsage/InitialHistory impls block): took fork empty - all relocated EXCEPT
    get_resumed_session_sources method (see CROSS-SLICE).
  - C8 (SessionMeta..SubAgentActivity types big block): took fork empty for relocated content
    but PRESERVED SubAgentActivityKind enum + SubAgentActivityEvent struct as a fork-local
    block in protocol.rs (consumers across core/tui/app-server depend on these types).
  - C9 (test fns user_input_*): kept fork side (fork tests use environments/context_budget_mode/
    collaboration_mode/personality fork features; verified against fork op.rs field shapes).

## CROSS-SLICE GAPS (outside my owned files - orchestrator/other resolver must close)
1. event_msg.rs MISSING `EventMsg::SubAgentActivity(SubAgentActivityEvent)` variant.
   codex-rs/protocol/src/protocol/event_msg.rs is clean/unconflicted but the fork's
   `impl From<SubAgentActivityEvent> for EventMsg` (in protocol.rs, fork-retained) and many
   consumers (app-server bespoke_event_handling, event_mapping, mcp-server, rollout, rollout-trace)
   reference EventMsg::SubAgentActivity. NEEDS: add the variant to event_msg.rs EventMsg enum.
   Types SubAgentActivityKind/SubAgentActivityEvent now live in protocol.rs (fork-local block).
2. rollout.rs MISSING `InitialHistory::get_resumed_session_sources(&self) -> Option<(SessionSource,
   Option<ThreadSource>)>`. codex-rs/protocol/src/protocol/rollout.rs has impl InitialHistory with
   get_resumed_session_meta (private) + get_resumed_thread_source but lacks get_resumed_session_sources.
   Consumers: core/agent/control/spawn.rs, core/thread_manager.rs. NEEDS adding to rollout.rs:
     pub fn get_resumed_session_sources(&self) -> Option<(SessionSource, Option<ThreadSource>)> {
         let meta = self.get_resumed_session_meta()?;
         Some((meta.source.clone(), meta.thread_source.clone()))
     }

## SCHEMA REGEN
protocol.rs / config_requirements.rs touch serde/TS types -> app-server-protocol schema JSON/TS
should be regenerated post-merge (just write-app-server-schema / write-config-schema).
