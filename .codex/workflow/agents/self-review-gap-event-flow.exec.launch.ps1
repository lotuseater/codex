$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-gap-event-flow'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Codex Rust self-review workflow prompt/event flow.
DO_NOT_INSPECT: Do not edit files. Do not run cargo, rustc, build scripts, deploy scripts, broad tests, schema generation, or generated-code steps. Do not duplicate the live main implementation worker; root verified PID 21776 is alive and owns implementation.
SCOUT_EVIDENCE: Root checked AGENTS/task memo, existing sidecar handoffs, and live worker state. Existing handoffs include self-review-sidecar-prompt-flow.md, self-review-sidecar-tracking.md, self-review-sidecar-commit-scope.md, and self-review-sidecar-artifacts-journal.md. Main handoff self-review-feature-main-worker.md did not exist yet at launch time.
WHY_AGENT / ROI: Independent read-only mapping can run while the main worker continues. Positive ROI from parallel_gain=3, context_gain=2, loop_followup_gain=3, cost=3, risk=0.
FIRST_READS:
- self-review feature.md
- .codex/workflow/agents/handoffs/self-review-sidecar-prompt-flow.md
- codex-rs/app-server/src/bespoke_event_handling.rs
- codex-rs/app-server/src/filters.rs
- codex-rs/app-server/tests/suite/v2/review.rs
- codex-rs/app-server-protocol/src/protocol/item_builders.rs
- codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs
TASK:
- Document the current review event flow and where automatic review prompts are inserted.
- Identify the smallest code touch points to make review suggestive, user-prompt-like, and acted on.
- Include how to insert the pre-review "sum-up recent changes / next plans / next actions" prompt, remember the answer, then reinsert the reminder after review/action.
- Include how to add a follow-up fix prompt after review findings when the model did not act automatically.
- Include how filters should keep review subagent/session items from polluting normal transcript views without hiding necessary user-prompt reminders.
TOOL_HINTS: Use narrow rg patterns and direct file reads. If a file is large, inspect only nearby functions around review/filter symbols.
TOKEN_TIP: Produce a compact handoff with exact file/function names and no long diffs.
VERIFICATION: No build/test. Include suggested focused assertions or simulations only.
HANDOFF: Write .codex/workflow/agents/handoffs/self-review-gap-event-flow.md with sections Findings, Recommended touch points, Suggested focused checks, Risks.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-gap-event-flow.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
