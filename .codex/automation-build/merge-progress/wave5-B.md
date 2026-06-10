# Wave-5 build-fix Worker B — progress  [DONE: cargo check -p codex-core --release EXITCODE=0]

Repo root asserted = C:/Users/Oleh/Documents/GitHub/open_ai/codex (OK).
Theme: merge dropped free fn/const/type the fork still references; locate real current home, fix reference; no invented symbols.

## ROOT CAUSE (single, covers most of my errors)
Upstream PR #27106 commit `08cb633c06` "[codex] Remove remote compaction failure log" DELETED:
- `log_remote_compact_failure`, `build_compact_request_log_data`, `CompactRequestLogData`
- `TotalTokenUsageBreakdown` type + `ContextManager::get_total_token_usage_breakdown` +
  `Session::get_total_token_usage_breakdown` (replaced by `Session::estimated_tokens_after_last_model_generated_item`)
- made `estimate_response_item_model_visible_bytes` PRIVATE again (no re-export from context_manager)

The merge applied that removal to `context_manager/history.rs`, `context_manager/mod.rs`,
and `session/mod.rs` (NOT my files — already correct in tree) but LEFT STALE pre-removal callers in:
- compact_remote.rs (call sites of build_compact_request_log_data / log_remote_compact_failure / estimate_..bytes / get_total_token_usage_breakdown)
- compact_remote_v2.rs (calls Session::estimated_tokens_after_last_model_generated_item — which is NOT yet defined on Session because fork relocated that method module)
- task_memory.rs (import of estimate_response_item_model_visible_bytes — now private)
- session/context_budget.rs (fork's RELOCATED copy of get_total_token_usage_breakdown + TotalTokenUsageBreakdown — stale duplicate)

=> FIX = adopt upstream removal in MY owned files (union-safe: this is generic codex
   plumbing, NOT a fork feature; fork added no custom logic here).

## Separate, unrelated error
config_loaders.rs BUILT_IN_WORKSPACE_PROFILE: const still exists (permissions.rs:43, pub(crate));
merge dropped the `use` line. FIX = add `use crate::config::permissions::BUILT_IN_WORKSPACE_PROFILE;`.

## EDITS (status)
- [x] config_loaders.rs: added `use crate::config::permissions::BUILT_IN_WORKSPACE_PROFILE;` (const still exists, pub(crate), at permissions.rs:43)
- [x] context_budget.rs: replaced Session::get_total_token_usage_breakdown -> Session::estimated_tokens_after_last_model_generated_item (matches upstream session/mod.rs; ContextManager method exists at history.rs:317). Fixes context_budget:206/208 AND compact_remote_v2:214 (the Session method it calls now exists).
- [x] compact_remote_v2.rs: NO EDIT NEEDED — merge already removed its stale imports + log fn; only issue was the missing Session method (now added in context_budget.rs).
- [x] compact_remote.rs: removed the 3 stale-symbol lines in the Err arm (get_total_token_usage_breakdown / build_compact_request_log_data / log_remote_compact_failure) — PRESERVED the fork's budget-retry continue/return (ContextWindowExceeded && attempt_index+1<budgets.len()). The CompactRequestLogData/build_../log_.. DEFINITIONS were already merge-dropped.
- [x] compact_remote.rs: line 644 caller of private estimate_response_item_model_visible_bytes (inside FORK fn estimate_remote_compaction_prompt_tokens, live at :474) -> added a LOCAL `fn estimate_response_item_model_visible_bytes` using serde_json::to_string(item).len().
- [x] task_memory.rs: dropped `use crate::context_manager::estimate_response_item_model_visible_bytes;` (now private upstream) -> added LOCAL serde_json-based helper of the same name. Used by fork estimated_prompt_tokens, called from context_budget.rs:282.

## DECISION FOR ORCHESTRATOR SANITY-CHECK
estimate_response_item_model_visible_bytes was made PRIVATE to context_manager by upstream #27106.
Two LIVE fork callers (compact_remote.rs:474 budget path, task_memory.rs estimated_prompt_tokens) still
need a per-item model-visible byte estimate. I CANNOT re-export it (history.rs/context_manager/mod.rs are
NOT my owned files). The original estimator's default arm IS `serde_json::to_string(item).len()`; it only
diverges for image data-URLs (discounted to ~7373 B) and encrypted reasoning. Both fork uses feed
approximate token budgets/thresholds, so the serde_json proxy is faithful for text and CONSERVATIVE
(over-estimates) for images -> only tightens budget / triggers injection slightly earlier. If exact parity
is required, the alternative is to re-export the private fn from context_manager (needs the owner of those
files to add `pub(crate) use history::estimate_response_item_model_visible_bytes;` back).
