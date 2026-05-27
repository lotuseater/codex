$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-post-fast-audit'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Fast final audit for self-review feature after reboot. Root is only overseer; you are an external noninteractive worker.

DO_NOT_INSPECT: Do not edit or repair `codex-rs/app-server/src/request_processors/turn_processor.rs`; it is an active merge-conflict file owned by another session. Do not run cargo/build/fmt/schema/deploy. Do not recursively delegate. Avoid broad repo scans.

SCOUT_EVIDENCE: Previous worker `self-review-post-final-audit` did not produce a handoff after multiple waits. Existing feature handoffs are in `.codex/workflow/agents/handoffs/self-review-post-core.md` and `.codex/workflow/agents/handoffs/self-review-post-surface.md`. Root also recorded checkpoints in `.codex/workflow/agents/handoffs/self-review-root-overseer.md`.

WHY_AGENT / ROI: User requested external noninteractive delegation. This is a tiny read-only replacement audit with separate output. ROI estimate: new_agent_cost=3, parallel_gain=2, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=6.

FIRST_READS:
1. `.codex/workflow/agents/handoffs/self-review-post-core.md`
2. `.codex/workflow/agents/handoffs/self-review-post-surface.md`
3. Only these implementation/test files if needed:
   - `codex-rs/self-review/src/lib.rs`
   - `codex-rs/self-review/src/git_evidence.rs`
   - `codex-rs/core/src/tasks/review.rs`
   - `codex-rs/app-server-protocol/src/protocol/v2/review.rs`
   - `codex-rs/app-server-protocol/src/protocol/v2/item.rs`
   - `codex-rs/app-server/src/bespoke_event_handling.rs`
   - `codex-rs/app-server/tests/suite/v2/review.rs`

TASK:
In 10-15 minutes maximum, decide whether the feature appears complete enough to stop at the merge blocker, or whether a focused fixer worker is needed. Audit only against the user requirements summarized in prior handoffs: agent-scoped evidence tracking/clear-on-review, suggestive prompt insertion, pre-review summary prompt with remembered answer, resume reminder prompt, review-fix follow-up prompt, extended action/strategy/automation/prototyping/architecture/planning reflection, code-preserved initial user prompts, accepted plan, and activity journal.

VERIFICATION:
Read-only. No full build due merge. You may inspect diffs and run cheap read-only checks only if needed.

HANDOFF:
Write `.codex/workflow/agents/handoffs/self-review-post-fast-audit.md` with status `pass`, `findings`, or `blocked-by-merge`; top 0-3 findings; whether a fixer is needed; percent done and time-to-finish estimate. Keep it concise.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-post-fast-audit.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
