$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave6_router'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Read-only next-wave merge routing for the active upstream/main merge in C:/Users/Oleh/Documents/GitHub/open_ai/codex.

DO_NOT_INSPECT: Do not run builds, tests, cargo commands, npm commands, or broad generated/target directory scans. Do not edit or stage files. Do not touch .git state. Do not modify worker-owned source files.

SCOUT_EVIDENCE: Root overseer already merged upstream/main and integrated the completed merge_wave5_config_cli_tools worker by staging its clean owned file list. Current reliable counts after that integration: 64 unmerged paths. Remaining unmerged area counts observed by root: app_protocol 17, core_runtime 30, tui 17. Active editing workers still running: merge_wave5_app_protocol, merge_wave5_core_runtime, merge_wave5_tui.

WHY_AGENT / ROI: Root must remain an overseer and sleep between worker checks. This read-only task prepares the next split without colliding with active editing workers. ROI estimate: new_agent_cost=3, parallel_gain=2, context_gain=2, repeat_gain=3, loop_followup_gain=3, risk_penalty=1, net=6.

FIRST_READS:
- .codex/workflow/agents/root_overseer_handoff.md
- .codex/workflow/agents/merge_wave5_common.md
- .codex/workflow/agents/merge_wave5_app_protocol.exec.visible.log
- .codex/workflow/agents/merge_wave5_core_runtime.exec.visible.log
- .codex/workflow/agents/merge_wave5_tui.exec.visible.log
- Use `git diff --name-only --diff-filter=U` for the current unmerged path list.
- Use narrow `Select-String`/`rg -l "^(<<<<<<<|=======|>>>>>>>)"` on the unmerged paths only if needed.

TOOL_HINTS: Use PowerShell list processing to group current unmerged paths by ownership and produce a compact assignment proposal. Stay read-only. If you need marker counts, inspect only the unmerged path list, not the whole repo.

TOKEN_TIP: Keep output compact. Do not paste large diffs. Report only areas, path lists, likely dependencies, and recommended next worker prompts.

VERIFICATION: Confirm in your handoff that you made no edits, staged nothing, and ran no builds/tests. Include exact current counts you observed.

HANDOFF: Write .codex/workflow/agents/merge_wave6_router.handoff.md with:
- HANDOFF_STATUS: success|partial|blocked
- COUNTS_OBSERVED:
- ACTIVE_WORKERS_OBSERVED:
- NEXT_SPLIT_RECOMMENDATION:
- HIGH_RISK_OVERLAPS:
- SUGGESTED_WORKER_PROMPTS:
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave6_router.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
