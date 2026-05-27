$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave4_crosscheck'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: read-only cross-check of merge conflict resolution risk
DO_NOT_INSPECT: Do not edit any files. Do not run build/tests/deploy/format/generation. Do not stage, commit, checkout, reset, or alter the merge.
SCOUT_EVIDENCE: Root observed 112 unmerged paths and is launching editing workers by area. A read-only reviewer can catch ownership gaps and risky conflict choices while edits proceed. Current file list: `.codex/workflow/agents/current-unmerged-files.txt`.
WHY_AGENT / ROI: This is parallel review without write risk. Agent ROI Estimate: new_agent_cost=3, parallel_gain=2, context_gain=3, repeat_gain=2, loop_followup_gain=2, risk_penalty=0, net=6.
FIRST_READS: Read `.codex/workflow/agents/merge_wave4_common.md`, `.codex/workflow/agents/current-unmerged-files.txt`, `.codex/workflow/agents/current-marker-counts.tsv`, and any existing `merge_stage1_*_worker.handoff.md` files.
TOOL_HINTS: Use `git diff --name-only --diff-filter=U`, `rg "^(<<<<<<<|=======|>>>>>>>)"`, and focused `git show :2/:3` reads only. Do not modify files.
TOKEN_TIP: Produce a concise checklist of path groups, unassigned paths, high-risk conflicts, and recommended follow-up owners.
VERIFICATION: Read-only only.
HANDOFF: Write `.codex/workflow/agents/merge_wave4_crosscheck.handoff.md` with ownership gaps, likely risky files, and `HANDOFF_STATUS`.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave4_crosscheck.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
