$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave13_index_audit'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# merge_wave13_index_audit

CONTEXT_AREA: Lightweight read-only index/content audit for the active upstream/main merge after reboot.

DO_NOT_INSPECT: Do not make code edits. Do not inspect broad source areas except current unmerged paths. Do not run broad builds/tests, cargo, deploy scripts, schema generation, or git add/commit/merge/rebase.

SCOUT_EVIDENCE: Root recovered `.codex/workflow/ROOT_TASK_HANDOFF.md` after reboot. Current merge is active on branch `slow-context-budget-mode`; `MERGE_HEAD` is upstream/main `14d80e55cd`. Some prior workers reported files resolved in the worktree but still unmerged in the index.

WHY_AGENT / ROI: Cheap parallel audit reduces root coordination risk; parallel_gain=2, context_gain=2, repeat_gain=2, loop_followup_gain=2, cost=3, risk=0, net=5.

FIRST_READS:
- `.codex/workflow/ROOT_TASK_HANDOFF.md`
- `.codex/workflow/agents/root_overseer_handoff.md`
- `.codex/workflow/agents/merge_wave12_core_config_finalize.handoff.md`
- `.codex/workflow/agents/merge_wave12_core_tests.handoff.md`

TASK:
1. Read-only audit of current unmerged index: use `git ls-files -u` and group by path/status stages.
2. Check only current unmerged paths for conflict markers with `rg -n "^(<<<<<<<|=======|>>>>>>>)"`.
3. Identify paths that appear marker-free and probably only need root `git add`, versus paths that still need content edits.
4. Do not edit files and do not stage files.

TOOL_HINTS: A small PowerShell grouping command is fine. Keep output compact.

TOKEN_TIP: Do not read full source files unless needed to classify a marker-free unmerged path.

VERIFICATION: No build/test. Read-only commands only.

HANDOFF: Write `.codex/workflow/agents/merge_wave13_index_audit.handoff.md` with counts, marker list, and recommended next root actions.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave13_index_audit.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
