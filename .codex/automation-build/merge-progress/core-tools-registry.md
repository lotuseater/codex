# core-tools-registry slice progress

## Guards
- toplevel_ok: true (resolves to repo root)
- unmerged_seen: true (all 3 files stages 1/2/3)

## Files
- registry.rs: DONE (union). 2 conflicts:
  - imports: kept fork's *_api crate imports (codex_tool_execution_api::{FunctionCallError,ToolName,ToolOutputPayload}, codex_tool_registry_api::ToolSpec) + ADDED upstream's `use codex_rollout::state_db;`. Dropped upstream's old `codex_tools::ToolName` (fork migrated).
  - handle_any_tool: fork refactored sig to `&dyn RegisteredTool` so used `tool.post_tool_use_payload(...)` (RegisteredTool method, NOT CoreToolRuntime::). Kept upstream's NEW memories `state_db::mark_thread_memory_mode_polluted` block (disable_on_external_context). Verified state_db/services.state_db/contains_external_context all exist in tree.
  - 0 markers, git diff --check clean.
- spec_plan.rs: DONE (take-fork). STRUCTURAL divergence: fork uses ToolsConfig/executors/ToolRegistryBuildParams shape; upstream rewrote to turn_context/PlannedTools/CoreToolPlanContext planner that the fork did NOT adopt. Confirmed callers (router.rs, spec.rs, spec_plan_types.rs, spec_tests.rs — all NOT conflicted) expect fork shape. Resolved all 6 conflicts take-fork. ALSO removed 2 orphan upstream lines that auto-merged outside markers and would break fork build: (a) `standalone_web_search_enabled(turn_context)`/`web_search_mode_on` lines in append_extension_tool_executors, (b) unused `use ...InterruptAgentHandler;` import. Result byte-identical to fork stage2 (CRLF-normalized). 0 markers, git diff --check clean.
- spec_plan_tests.rs: DONE (take-fork, structural). Fork uses build_specs/ToolsConfig harness (59 refs); upstream rewrote to probe()/turn_context harness (0 in fork) — same divergence as spec_plan.rs. Wrote fork stage2 wholesale (CRLF). Byte-identical to fork stage2 (LF-normalized, equal=True). 0 markers, git diff --check clean. NOTE: upstream's NEW probe-based test cases (multi_agent_v2 namespace visibility, agents-namespace, etc.) were DROPPED because they reference the upstream probe harness absent in the fork; flagged FILES_UNCERTAIN for the later test-repair wave to re-add against build_specs harness.

## ALL DONE — 3/3 files, 0 markers total.
