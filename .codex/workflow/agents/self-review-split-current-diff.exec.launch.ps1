$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-split-current-diff'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: self-review feature implementation, current worktree/diff scope only.
DO_NOT_INSPECT: Do not perform broad repository sweeps, builds, tests, schema generation, deployment, or commits. Do not edit source files. Only write your handoff file.
SCOUT_EVIDENCE: Root checked AGENTS.md worker conventions, task memo `self-review feature.md`, current handoff directory, and marker PIDs 23384/33488; both marker PIDs are not running. Existing handoffs include multiple `self-review-research-*` docs and a large `self-review-main.exec.log`.
WHY_AGENT / ROI: Independent research lane; helps root avoid duplicating old worker findings while keeping integration context small. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=7.
FIRST_READS:
- `self-review feature.md`
- latest 8 files in `.codex/workflow/agents/handoffs` matching `self-review-*.md`
- `git diff --name-only`
- `git diff -- codex-rs/app-server-protocol/src/protocol/v2/review.rs codex-rs/app-server/src/bespoke_event_handling.rs codex-rs/app-server/src/request_processors/turn_processor.rs codex-rs/app-server/tests/suite/v2/review.rs`
TOOL_HINTS: Use `rg`/focused `git diff -- <path>` only. Use small PowerShell snippets for compact file lists. Avoid broad `git diff` dumps.
TOKEN_TIP: Stop after you have enough to answer. Prefer a concise handoff over exhaustive notes.
TASK: Determine what the current worktree already changed for the self-review feature, what appears incomplete or risky, and which files root should inspect next. Include whether any older/other-agent changes appear present and how to avoid pulling them into review scope.
VERIFICATION: No tests/builds. Validate only by matching file paths/functions in diffs.
HANDOFF: Write `.codex/workflow/agents/handoffs/self-review-split-current-diff.md` with sections: Summary, Changed Files, Likely Complete, Gaps/Risks, Recommended Root Reads, Commands Run. Keep under 120 lines.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-split-current-diff.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
