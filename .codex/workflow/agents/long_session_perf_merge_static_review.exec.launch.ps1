$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: long_session_perf_merge_static_review'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: open_ai/codex branch slow-context-budget-mode is mid-merge against MERGE_HEAD. Root must integrate long-session performance fixes, then build/deploy/commit/push. You are a read-only merge-risk reviewer.
DO_NOT_INSPECT: Do not run cargo, rustc, just, build scripts, tests, schema generation, deploy scripts, or broad repo sweeps. Do not edit source or stage/unstage. Only write your own handoff.
SCOUT_EVIDENCE: Root counted 90 unmerged paths. Recent local commit is 1df5afe4be Refactor merge conflict hotspots before upstream merge; MERGE_HEAD is 9f42c89c01. Handoffs under .codex/workflow/agents describe TUI frame requester and app protocol/MCP fixes.
WHY_AGENT / ROI: Parallel merge-risk inspection lets root decide whether to continue current merge, abort/restart, or integrate selected files. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=2, risk_penalty=1, net=6.
FIRST_READS: Start with git status --short --branch, git diff --name-only --diff-filter=U, .git/MERGE_MSG, and the recent handoff md files. Then inspect only conflict hotspots that affect the proposed fixes: frame requester/TUI, app-server protocol, core session/MCP scope/config/schema files.
TOOL_HINTS: Use rg conflict-marker searches and focused git diff/status. No build/test commands. If command output is huge, reduce to counts and key paths.
TOKEN_TIP: Prioritize actionable merge guidance and list blockers; do not catalog every unrelated conflict.
VERIFICATION: Static inspection only. Determine if current merge state appears safe to resolve manually, if a clean restart/cherry-pick approach is safer, and what exact files should be protected.
HANDOFF: Write .codex/workflow/agents/long_session_perf_merge_static_review.handoff.md with sections: Verdict, Conflict Hotspots, Safe Integration Route, Blockers, Files Inspected. Do not wait for root.')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\long_session_perf_merge_static_review.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
