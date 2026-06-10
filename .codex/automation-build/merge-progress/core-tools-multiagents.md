# core-tools-multiagents merge progress

Guard: toplevel OK, all 5 files unmerged (stages 1/2/3 present).

## DONE
- multi_agents_v2.rs — UNION (kept fork close/compact event imports, CloseAgent/CompactAgent
  handler re-exports, `mod close_agent`/`mod compact_agent`). Upstream had nothing in those slots.
- multi_agents_v2/spawn.rs — TAKE-FORK on the conflict (nested-spawn lineage check +
  CollabAgentSpawnBeginEvent). Restored `let prompt = String::new();` (auto-merge dropped it).
- multi_agents_v2/message_tool.rs — STRUCTURAL: kept fork signature (turn_overrides param),
  fork FollowupTaskArgs model/reasoning fields, fork turn-context override block + receiver
  config snapshot + CollabAgentInteractionBeginEvent + CollabAgentInteractionEndEvent (with
  get_status). Adopted upstream's ensure_agent_known (same AgentMetadata type). Restored
  `let prompt = String::new();`. Kept `result?` AFTER end event per fork semantics.

- multi_agents_spec.rs — TAKE-FORK facade (fork migrated all builders to
  codex_tool_registry_api; thin wrappers + pub use + as_tool_options/into_tool_options +
  #[cfg(test)] mod tests). RESTORED fork-local `create_interrupt_agent_tool_v2` +
  agent_previous_status_output_schema + agent_status_output_schema (upstream's NEW
  interrupt_agent.rs handler imports it from multi_agents_spec; not migrated to registry-api).
  Added codex_tools::{JsonSchema,ResponsesApiTool}+serde_json+BTreeMap imports for it.
  Verified ResponsesApiTool fields + JsonSchema::object/string sigs match. 0 markers.

- multi_agents_tests.rs — TAKE-FORK structural split (fork moved tests into 9 submodules:
  build_config/close_agent/send_input/service_tier/spawn/v2_messaging/v2_spawn/wait/wait_v2,
  all present). Wrote file = ours stage2 (module root: shared helpers + 9 mod decls). Restored
  fork's `CloseAgentHandler as CloseAgentHandlerV2` import (close_agent submodule needs it via
  `use super::*`; auto-merge had dropped it). Excluded upstream-only unused imports
  ApprovalsReviewer + InterruptAgentHandler (no fork submodule uses them).
  KNOWN GAP (flag): upstream added 9 NEW inline tests not in fork submodules — 6 interrupt_agent
  v2 tests + multi_agent_v2_list_agents_keeps_interrupted_resident_agents +
  ..._returns_completed_status_without_encrypted_spawn_preview +
  multi_agent_v2_spawn_agent_ignores_configured_max_depth + install_role_with_model_override
  helper variant. These need re-homing into fork submodules in the test-repair wave.

## DONE — ALL 5 FILES, 0 markers.

## GOTCHAS
- `let prompt = String::new();` gets dropped by auto-merge in spawn.rs + message_tool.rs;
  the fork event blocks need it — restore.
- agent_control methods get_status / get_agent_config_snapshot / override_agent_turn_context
  all still exist (control.rs / control/override_local.rs). ensure_agent_known returns AgentMetadata.
