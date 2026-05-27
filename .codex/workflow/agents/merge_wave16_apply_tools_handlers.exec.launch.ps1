$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave16_apply_tools_handlers'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: upstream/main merge recovery, APPLY conflicts for core tools/handlers/tasks slice only.

DO_NOT_INSPECT: Do not run broad searches or builds/tests. Do not inspect config/session/test-suite files except for direct references. Do not edit or stage files outside OWNERSHIP. Do not edit or stage `codex-rs/core/src/tools/registry_tests.rs`; it is owned by another active worker. Do not kill/interact with other worker processes. You are not alone in the codebase; avoid reverting or overwriting external edits.

SCOUT_EVIDENCE: Root verified repo is mid-merge on `slow-context-budget-mode` with `MERGE_HEAD=14d80e55cd`. Wave14 advisory tools/handlers worker wrote `.codex/workflow/agents/merge_wave14_recovery_tools_handlers.handoff.md`; use it as the primary resolution guide. Wave15 spec/test implementation worker is active and owns `registry_tests.rs`, so exclude it from this slice.

WHY_AGENT / ROI: Advisory recommendations are available and this file slice is disjoint from active spec/test and config/session work. Parallel implementation reduces wall time. Highest-capability external non-interactive worker requested by user. ROI: new_agent_cost=3, parallel_gain=3, context_gain=2, repeat_gain=2, loop_followup_gain=2, risk_penalty=1, net=5.

OWNERSHIP: You may edit and `git add` ONLY these paths:
- `codex-rs/core/src/tasks/mod.rs`
- `codex-rs/core/src/tools/code_mode/execute_handler.rs`
- `codex-rs/core/src/tools/handlers/extension_tools.rs`
- `codex-rs/core/src/tools/handlers/mcp.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`

FIRST_READS: Read `.codex/workflow/agents/merge_wave14_recovery_tools_handlers.handoff.md` first. Then read `git diff --cc -- <OWNERSHIP paths>` and, where needed, `git show :1:<path>`, `git show :2:<path>`, `git show :3:<path>`.

TASK: Resolve merge conflicts for OWNERSHIP paths, preserving local split-crate/tool-execution refactor behavior plus upstream/main changes. If a file has no working-tree markers but is still unmerged, inspect stages and stage the correct resolved working-tree content if appropriate. Stage only OWNERSHIP paths you resolve. Do not commit.

TOOL_HINTS: Use targeted per-file inspection. Use `rg -n ''<<<<<<<|=======|>>>>>>>'' <OWNERSHIP paths>` only on assigned files. Use `git diff --name-only --diff-filter=U -- <OWNERSHIP paths>` to verify assigned unmerged paths are cleared. Avoid broad `rg`, cargo, rustc, just, schema generation, deploy/activation, or build/test scripts.

TOKEN_TIP: Keep output concise. Write the handoff file instead of printing long diffs.

VERIFICATION: No builds/tests. Verify only by checking no conflict markers in OWNERSHIP files and no unmerged entries for OWNERSHIP paths. If semantically ambiguous, leave it unresolved and document exact blocker.

HANDOFF: Write `.codex/workflow/agents/merge_wave16_apply_tools_handlers.handoff.md` with: (1) files resolved/staged, (2) key stage/snippet choices, (3) unresolved blockers if any, (4) verification commands run, (5) whether root can proceed.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave16_apply_tools_handlers.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
