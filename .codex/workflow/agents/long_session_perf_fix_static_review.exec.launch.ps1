$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: long_session_perf_fix_static_review'
Set-Location -LiteralPath 'long_session_perf_merge_static_review'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'long_session_perf_merge_static_review', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: open_ai/codex long-session degradation investigation/fix integration. Current branch is slow-context-budget-mode and the tree is mid-merge; do not resolve conflicts or edit source.
DO_NOT_INSPECT: Do not run cargo, rustc, just, build scripts, tests, schema generation, deploy scripts, or broad history scans. Do not modify any file except your own handoff.
SCOUT_EVIDENCE: Existing handoffs in .codex/workflow/agents include long_session_perf_tui_impl.handoff.md, long_session_perf_tui_review.handoff.md, long_session_perf_tui_inspection.handoff.md, long_session_perf_app_protocol_impl.handoff.md, long_session_perf_core_app_scope.handoff.md, long_session_perf_core_review.handoff.md, and long_session_perf_app_mcp_review.handoff.md. Root observed the TUI accepted fix is in codex-rs/tui/src/tui/frame_requester.rs and the merge has many conflicts.
WHY_AGENT / ROI: Independent static review reduces risk before root integrates a large change. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=2, risk_penalty=1, net=6.
FIRST_READS: Read the handoff files above first, then inspect the exact source files they mention. Pay special attention to codex-rs/tui/src/tui/frame_requester.rs and app-server/app-server-protocol/MCP files named by the handoffs. If a file has conflict markers, report what can and cannot be reviewed.
TOOL_HINTS: Use rg and git diff focused on named files. No build/test commands. Keep output compact.
TOKEN_TIP: Spend tokens on concrete findings with file/line references and integration recommendations, not broad summaries.
VERIFICATION: Static code inspection only. Check for obvious logic races, lifetime leaks, unbounded per-frame work, compile-time type mismatches visible by reading, and missing docs/tests.
HANDOFF: Write .codex/workflow/agents/long_session_perf_fix_static_review.handoff.md with sections: Verdict, Findings, Required Before Commit, Optional Followups, Files Inspected. Do not wait for root.')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\long_session_perf_fix_static_review.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
