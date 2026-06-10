# TUI slice merge progress — COMPLETE

## Guards
- toplevel_ok: true
- unmerged_seen: true (all 5 files stages 1/2/3)

## DONE (all 5, 0 markers each; git diff --check clean)
- snapshot (review_submission_warning) -> take-upstream. interaction.rs (not in slice, clean) holds upstream warning text "Press Ctrl+C now to cancel the review." so snapshot must match upstream's shorter 2-line message.
- markdown_render.rs -> STRUCTURAL take-fork: fork extracted module to codex_tui_render crate; file is now a re-export shim (pub use codex_tui_render::markdown_render::*;). Dropped upstream's inline 2770-line body. FLAG: upstream's only change vs base was a DecodedTextMerge wrapper (from markdown_text_merge) that landed in tui/src/markdown_text_merge.rs but is NOT in codex-rs/tui-render/src/markdown_render.rs — build-fix wave must port it into the tui-render crate copy.
- external_agent_config_migration_startup.rs -> UNION imports: kept upstream CloudConfigBundleLoader + fork ConfigEdit/ConfigEditsBuilder (all used).
- app.rs -> UNION two free fns: upstream sub_agent_activity_item + fork collab_agent_item_is_not_found (both used).
- app/agent_navigation.rs -> STRUCTURAL UNION. Final AgentPickerThreadEntry struct (in multi_agents.rs, already merged clean) = union of all 8 fields (agent_nickname, agent_role, agent_path, is_running, is_closed, model, reasoning_effort, token_context_percent_used).
  - upsert: merged both prev-state reads (agent_path/is_running from upstream + model/reasoning_effort/token_context_percent_used from fork) into pre-insert locals; insert literal uses all 8.
  - Unioned 5 methods: upstream record_sub_agent_activity/set_running/set_agent_path + fork update_runtime_details/update_token_context_percent_used. All entry literals expanded to all 8 fields.
  - CORRECTNESS FIX: fork methods originally had `self.order.push(thread_id)` INSIDE the or_insert_with closure (borrow-check error: borrows self.order while self.threads is borrowed by entry()). Moved push out to a preceding `if !contains_key` guard (upstream's pattern). Preserves fork intent (append on first insert).
  - ReasoningEffortConfig == codex_protocol::openai_models::ReasoningEffort (alias) so fork param type matches struct field type.
  - All 5 methods confirmed called in session_lifecycle.rs / thread_routing.rs / tests.rs.

## files_needing_regen
- none (snapshot resolved deterministically to match code; not flagged for regen)

## files_uncertain
- markdown_render.rs (med) — DecodedTextMerge port to tui-render crate is a cross-crate task outside this slice.
