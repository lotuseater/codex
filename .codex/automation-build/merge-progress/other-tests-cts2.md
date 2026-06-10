# other-tests-cts2 merge progress

Guards: toplevel_ok=true, unmerged_seen=true.

## Key pattern (confirmed by sibling core-tests-3.md)
- Fork renamed `core_test_support` crate -> `codex_core_test_runtime`; fork already imports
  everything. => DROP upstream's duplicate `use core_test_support::*;` import blocks.
- Protocol `Op::UserTurn` kept the FORK FLAT shape (cwd/approval_policy/sandbox_policy/
  permission_profile/model/effort/summary/service_tier/context_budget_mode/collaboration_mode/
  personality). Upstream's nested `thread_settings: ThreadSettingsOverrides` +
  `responsesapi_client_metadata`/`additional_context` was NOT adopted. => For UserTurn conflicts
  TAKE HEAD (fork flat). `local_selections`/`TempDirExt`/`core_test_support` end up 0 refs => drop.
- Upstream sometimes adds `use pretty_assertions::assert_eq;` only on its side — keep that one line.

## DONE
- cli_stream.rs (1 conflict) — dropped upstream core_test_support imports, kept fork
  codex_core_test_runtime imports (already present) + kept `use pretty_assertions::assert_eq;`.
- websocket_fallback_switch.rs (2 conflicts) — dropped upstream core_test_support import block;
  UserTurn took HEAD (fork flat shape). markers=0.

- client.rs (3 conflicts) — kept fork codex_thread_store imports; dropped upstream core_test_support
  block; ADDED `use codex_core_test_runtime::responses::ev_completed_with_tokens;` + `sse_failed`
  (used in body, not previously imported). Both UserTurn conflicts took HEAD (fork flat). markers=0.

- compact_resume_fork.rs (2 conflicts) — fork feature test KEPT. Import: kept fork
  codex_test_support_responses::context_snapshot, dropped upstream core_test_support block.
  2nd conflict: kept fork Op::OverrideTurnContext (fork-owned op), dropped upstream
  submit_thread_settings/ThreadSettingsOverrides path. markers=0.
- exec_policy.rs (3 conflicts) — dropped upstream import block; 2x UserTurn took HEAD (fork flat).
- shell_snapshot.rs (5 conflicts) — take-HEAD script: import block (HEAD empty) + 4 UserTurn flat.
- user_shell_cmd.rs (2 conflicts) — take-HEAD script: import block + 1 UserTurn flat.

- model_visible_layout.rs (6 conflicts) — take-HEAD: 5 UserTurn flat (kept fork
  personality: Some(Personality::Friendly)) + 1 Op::OverrideTurnContext (vs upstream
  submit_thread_settings). Also REMOVED stray auto-merged
  `use core_test_support::test_codex::local_selections;` (line 3) — core_test_support is NOT a
  dependency of realtime-code-mode crate; local_selections had 0 body refs after take-HEAD.

## COMPLETE — markers=0 across all 8 files (verified git diff --check, only LF/CRLF warnings).
No orphaned core_test_support/local_selections/submit_thread_settings refs remain anywhere.
