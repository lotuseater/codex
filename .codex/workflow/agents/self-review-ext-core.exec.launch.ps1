$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-ext-core'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Codex Rust self-review feature core implementation.
DO_NOT_INSPECT: Do not inspect unrelated merge work, unrelated UI/TUI files, or broad history. Do not resolve merge conflicts. Do not edit codex-rs/app-server/src/request_processors/turn_processor.rs because merge from main is in progress and it is currently unmerged.
SCOUT_EVIDENCE: Existing handoffs in .codex/workflow/agents/handoffs/self-review*.md, especially retry/reflection/tracking/event-flow files, plus current targeted status shows self-review feature edits and an unmerged turn_processor.rs.
WHY_AGENT / ROI: User explicitly requested root only oversee external noninteractive sessions. This worker owns core feature code so root can stay out of implementation. Positive ROI: parallel_gain=3, context_gain=2, repeat_gain=3, loop_followup_gain=3, cost=3, risk=1, net=7.
FIRST_READS: Read .codex/workflow/agents/handoffs/self-review-retry-reflection-artifacts.md, self-review-retry-current-diff.md, self-review-gap-tracking-commit.md, self-review-gap-event-flow.md if present. Then read codex-rs/core/src/session/review.rs, codex-rs/core/src/review_prompts.rs, codex-rs/core/src/state/turn.rs, and relevant codex-rs/core/tests/suite/review*.rs.
TASK: Implement only the core-side missing pieces for improved self-review: code-tracked current-agent changed-file and commit scope since last review with reset after review, suggestive review prompt flow, pre-review summary/plans/actions prompt capture, post-review resume reminder prompt, fallback prompt to act on findings when needed, expanded action/strategy/delegation/automation/prototyping/architecture/planning reflection, and code-preserved artifacts for initial user prompts, accepted plan, and activity journal since last review. Stay within owned core files/tests. If app-server/protocol or turn_processor changes are required, write exact recommendations to your handoff rather than editing outside scope.
TOOL_HINTS: Use rg and targeted reads. Prefer small scripted inspections for repeated checks. Use apply_patch for code edits. Avoid broad build/test loops; run the narrowest cargo tests for core review behavior if feasible.
TOKEN_TIP: Keep notes compact. Do not paste large diffs into handoff.
VERIFICATION: Run focused tests you change or explain why blocked by merge state.
HANDOFF: Write .codex/workflow/agents/handoffs/self-review-ext-core.md with summary, files touched, tests run/results, blockers, percent estimate, and exact follow-up needed.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-ext-core.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
