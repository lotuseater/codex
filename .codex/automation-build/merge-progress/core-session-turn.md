# Slice: core-session-turn

## Files owned
- codex-rs/core/src/session/turn.rs  -> DONE (0 markers)

## Resolution
Single conflict block at old lines 1790-2325.
- HEAD (fork) side was EMPTY.
- upstream/main side added a large inline plan-mode streaming block
  (ProposedPlanItemState, PlanModeStreamState, AssistantMessageStreamParsers,
   realtime_text_for_event, handle_plan_segments, emit_streamed_assistant_text_delta,
   flush_assistant_text_segments_*, maybe_complete_plan_item_from_message,
   emit_agent_message_in_plan_mode, emit_turn_item_in_plan_mode,
   handle_assistant_item_done_in_plan_mode, etc).

The fork EXTRACTED all of these into session/turn/plan_mode.rs (already resolved,
in working tree; NOT my slice). turn.rs imports them via `use plan_mode::*` (line 145)
+ re-exports AssistantMessageStreamParsers / realtime_text_for_event (lines 149-150).

=> Resolution = take-fork / STRUCTURAL: keep the empty HEAD side, DELETE upstream's
inline duplicate block (re-inlining it = duplicate-definition compile errors vs
`use plan_mode::*`). Added a `// fork-local:` note at the seam.

Verified the in-file call-site (handle_assistant_item_done_in_plan_mode, ~line now)
uses the 6-arg fork signature (no turn_store), matching plan_mode.rs — consistent.

## GOTCHA for orchestrator / plan_mode.rs owner
Upstream's inline copy was NEWER than the fork's plan_mode.rs snapshot. Divergences
upstream has that plan_mode.rs lacks:
  1. handle_assistant_item_done_in_plan_mode upstream takes
     `turn_store: &codex_extension_api::ExtensionData` and uses
     finalize_non_tool_response_item + record_completed_response_item_with_finalized_facts
     + TurnItemContributorPolicy::Run(turn_store).
     Fork plan_mode.rs uses handle_non_tool_response_item + record_completed_response_item
     + TurnItemContributorPolicy::Skip (older shape).
  2. realtime_text_for_event match: upstream ends Collab arms at CollabResume*/then
     `SubAgentActivity(_)`; fork plan_mode.rs has CollabCompact*/CollabRestart* +
     ThreadSettingsApplied, NO SubAgentActivity (EventMsg enum divergence -> protocol slice).
These deltas must be ported into session/turn/plan_mode.rs by that file's owner; they
are OUT OF MY SLICE. Flagged under FILES_UNCERTAIN-adjacent in handoff.

## Status: success, markers_remaining = 0
