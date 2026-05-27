$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: long_session_perf_final_static_review'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Final static review for long-running TUI session performance fix before root builds/deploys.
DO_NOT_INSPECT: Do not inspect unrelated dirty SOLID refactor worker files beyond confirming they are unrelated. Do not edit files. Do not build or run tests.
SCOUT_EVIDENCE: Root already inspected existing handoffs `.codex/workflow/agents/long_session_perf_merge_static_review.handoff.md` and `.codex/workflow/agents/long_session_perf_tui_inspection.handoff.md`; prior reviews supported the TUI frame requester fix. Root found and removed an inconsistent partial app-server slice before launching you.
WHY_AGENT / ROI: User explicitly requested delegated inspection. External non-interactive review gives independent code inspection before build/deploy. ROI: new_agent_cost=3, parallel_gain=2, context_gain=2, repeat_gain=1, loop_followup_gain=2, risk_penalty=1, net=3.
FIRST_READS: `git diff origin/slow-context-budget-mode..HEAD -- codex-rs/tui/src/tui/frame_requester.rs`; `git show --stat --oneline HEAD`; `docs/long-running-session-performance-investigation.md`; existing handoffs if needed. Also run `git status --short --untracked-files=no` and identify whether dirty tracked files are unrelated.
TOOL_HINTS: Use focused `git diff`, `git show`, `rg`, and direct file reads. Keep it read-only. No `cargo`, `rustc`, build scripts, test scripts, formatters, or deployment commands.
TOKEN_TIP: Spend most attention on correctness risks in `frame_requester.rs`: deadlocks, missed wakeups, coalescing semantics, shutdown behavior, timeout math, and tests. Do not restate the whole diff.
VERIFICATION: Static review only. Report concrete findings with file/line references if any; otherwise say no blocking findings found and list residual risks.
HANDOFF: Write `.codex/workflow/agents/long_session_perf_final_static_review.handoff.md` with: verdict, blocking findings, non-blocking risks, dirty-tree separation notes, and whether root should proceed to build/deploy.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\long_session_perf_final_static_review.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
