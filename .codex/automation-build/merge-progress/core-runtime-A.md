# core-runtime-A merge progress

Slice files (5):
- [DONE] codex-rs/core/src/agent/control.rs        -- union mod block (compact_local+execution+legacy+override_local+residency+spawn)
- [DONE] codex-rs/core/src/agent/control/spawn.rs  -- adopt upstream V2 residency/execution-capacity reservation; kept fork's fallible effective_agent_max_threads(...)? (returns io::Result)
- [DONE] codex-rs/core/src/client.rs               -- kept fork RuntimeAuthMode branch; added fork-local PersonalAccessToken arm (upstream-added CodexAuth variant)
- [DONE] codex-rs/core/src/codex_thread.rs         -- took fork(empty) for ThreadConfigSnapshot/CodexThreadSettingsOverrides (moved to thread-manager-api crate, re-exported); 3rd conflict unioned upstream ensure_execution_capacity_for_op gate + fork submit_with_id delegation (Codex has NO submit_user_input_with_client_user_message_id, only submit_with_id)
- [DONE] codex-rs/core/src/compact_remote.rs       -- kept fork approx_token_count import (used L653); signature took upstream type _analytics_details: &mut CompactionAnalyticsDetails (CALLER passes &mut analytics_details); body kept fork budget-retry loop + task_memory + process_compacted_history(5-arg). upstream single-shot dropped; trim_function_call_history_to_fit_context_window still defined (unused pub(crate), harmless).

ALL 5 FILES DONE. 0 markers. git diff --check clean.

## Gotchas
- effective_agent_max_threads now returns std::io::Result<Option<usize>> in fork (config_accessors.rs) -> keep the `?`.
- CodexAuth gained PersonalAccessToken variant upstream (login/src/auth/manager.rs); fork match must include it.
- codex_auth_api::AuthMode (RuntimeAuthMode) has NO PersonalAccessToken; only the login AuthMode does.
