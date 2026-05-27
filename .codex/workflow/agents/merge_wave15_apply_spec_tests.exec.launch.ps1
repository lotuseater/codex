$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave15_apply_spec_tests'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: upstream/main merge recovery, APPLY conflicts for spec/tool-family/core test-suite slice only.

DO_NOT_INSPECT: Do not run broad searches or builds/tests. Do not inspect config/session/handler files except for direct references. Do not edit or stage files outside OWNERSHIP. Do not kill/interact with other worker processes. You are not alone in the codebase; other workers may be inspecting or editing other conflict slices. Do not revert their changes.

SCOUT_EVIDENCE: Root resumed after reboot and verified the repo is mid-merge on `slow-context-budget-mode` with `MERGE_HEAD=14d80e55cd`, local pre-merge HEAD `74676253d8`, and 21 unresolved paths. Wave14 advisory spec/test worker appears to have finished without writing its requested handoff file; its visible log is `.codex/workflow/agents/merge_wave14_recovery_spec_tests.exec.visible.log` and can be read narrowly for any recommendations.

WHY_AGENT / ROI: This slice is independent from active config/session and tools/handlers advisory workers, so parallel implementation can reduce wall time without racing their files. Highest-capability external non-interactive worker requested by user. ROI: new_agent_cost=3, parallel_gain=3, context_gain=2, repeat_gain=2, loop_followup_gain=2, risk_penalty=1, net=5.

OWNERSHIP: You may edit and `git add` ONLY these paths:
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/spec_plan_tests.rs`
- `codex-rs/core/src/tools/registry_tests.rs`
- `codex-rs/core/src/tools/tool_family/shell.rs`
- `codex-rs/core/tests/suite/client_websockets.rs`
- `codex-rs/core/tests/suite/code_mode.rs`
- `codex-rs/core/tests/suite/compact_remote.rs`

FIRST_READS: Start with `git diff --cc -- <OWNERSHIP paths>` and, where needed, `git show :1:<path>`, `git show :2:<path>`, `git show :3:<path>`. Optionally read only the tail/recommendation lines from `.codex/workflow/agents/merge_wave14_recovery_spec_tests.exec.visible.log`.

TASK: Resolve merge conflicts for the OWNERSHIP paths while preserving branch behavior plus upstream/main updates. For deleted-by-us/deleted-by-them conflicts, make an explicit keep/delete choice based on current repository intent and document it. Stage only the OWNERSHIP paths you resolve with `git add <path>` or `git rm <path>` as appropriate. Do not commit.

TOOL_HINTS: Use targeted per-file conflict inspection. Use `rg -n ''<<<<<<<|=======|>>>>>>>'' <OWNERSHIP paths>` only on assigned files. Use `git diff --name-only --diff-filter=U -- <OWNERSHIP paths>` to verify assigned unmerged paths are cleared. Avoid broad `rg`, cargo, rustc, just, schema generation, deploy/activation, or build/test scripts.

TOKEN_TIP: Keep output concise. Write the handoff file instead of printing long diffs.

VERIFICATION: No builds/tests. Verify only by checking no conflict markers in OWNERSHIP files and no unmerged entries for OWNERSHIP paths. If a conflict is semantically ambiguous, leave it unresolved and document the exact blocker instead of guessing.

HANDOFF: Write `.codex/workflow/agents/merge_wave15_apply_spec_tests.handoff.md` with: (1) files resolved/staged, (2) keep/delete decisions, (3) unresolved blockers if any, (4) verification commands run, (5) whether root can proceed to other slices.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave15_apply_spec_tests.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
