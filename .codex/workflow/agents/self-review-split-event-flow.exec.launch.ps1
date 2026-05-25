$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-split-event-flow'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: self-review prompt/event flow and suggestive review insertion.
DO_NOT_INSPECT: Do not build, test, generate schemas, deploy, commit, or edit source files. Only write your handoff file. Avoid broad repository searches after FIRST_READS unless a symbol is missing.
SCOUT_EVIDENCE: Root confirmed old external marker PIDs 23384/33488 are not running and will launch this as a fresh non-interactive research lane. Task memo is `self-review feature.md`.
WHY_AGENT / ROI: Independent event-flow analysis can run in parallel with tracking/artifact research. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=7.
FIRST_READS:
- `self-review feature.md`
- `codex-rs/app-server-protocol/src/protocol/v2/review.rs`
- `codex-rs/app-server/src/bespoke_event_handling.rs`
- `codex-rs/app-server/src/request_processors/turn_processor.rs`
- `codex-rs/app-server/tests/suite/v2/review.rs`
- any latest handoff matching `self-review-research-event-flow*.md`
TOOL_HINTS: Use `rg "Review|review|SelfReview|self_review|review_request|turn" codex-rs/app-server codex-rs/app-server-protocol -g ''*.rs''` only if the named reads do not expose the flow.
TOKEN_TIP: Focus on event ordering and where prompts/messages are inserted.
TASK: Document how the self-review request should be made suggestive/user-prompt-like, including the pre-review summary prompt, remembering the answer, reinserting the reminder prompt after review/actions, and adding a fix-review-findings prompt when findings are not automatically fixed. Identify exact functions/types likely needing edits and test assertions to add later.
VERIFICATION: No builds/tests. Reason from source and existing tests only.
HANDOFF: Write `.codex/workflow/agents/handoffs/self-review-split-event-flow.md` with sections: Flow Map, Needed Behavior, Code Edit Targets, Test Assertions, Risks, Commands Run. Keep under 140 lines.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-split-event-flow.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
