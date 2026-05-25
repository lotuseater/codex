$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-gap-artifacts-tests'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Codex Rust self-review artifacts, activity journal, and focused verification plan.
DO_NOT_INSPECT: Do not edit files. Do not run cargo, rustc, build scripts, deploy scripts, broad tests, schema generation, or deploy/activation. Do not duplicate the live main implementation worker; root verified PID 21776 is alive and owns implementation.
SCOUT_EVIDENCE: Root checked existing sidecar handoffs and live main worker. Existing artifacts/journal handoff exists, but focused test/simulation coverage still needs compact mapping.
WHY_AGENT / ROI: A read-only test/artifact mapper can run in parallel and reduce root integration risk. Positive ROI from parallel_gain=2, context_gain=3, loop_followup_gain=3, cost=3, risk=0.
FIRST_READS:
- self-review feature.md
- .codex/workflow/agents/handoffs/self-review-sidecar-artifacts-journal.md
- codex-rs/app-server/tests/suite/v2/review.rs
- codex-rs/app-server/src/bespoke_event_handling.rs
- codex-rs/app-server/src/filters.rs
- codex-rs/core/src/session/tests.rs
- codex-rs/core/src/session/tests
TASK:
- Document how code should preserve initial user prompts, initial accepted plan, and activity journal since last review.
- Document the review reflection prompt content for: optimal actions for current task/user request, best strategy, long-term perspective, delegation/parallelization, automation/scripting, prototyping before broad builds/tests, architecture/SOLID/decoupling/quality/complexity, and whether activity is well planned and structured.
- Identify minimal focused tests or simulations that would cover prompt order, reminder insertion, journal/artifact persistence, and post-review fix prompt.
- Include a no-broad-build verification sequence that root can run later after code edits.
TOOL_HINTS: Start from listed files and existing handoff. Use narrow rg for terms like review, journal, plan, transcript, and session item only after direct reads.
TOKEN_TIP: Produce a concise checklist with exact test names or new test names.
VERIFICATION: No commands beyond read-only inspection.
HANDOFF: Write .codex/workflow/agents/handoffs/self-review-gap-artifacts-tests.md with sections Findings, Artifact model, Reflection prompt requirements, Focused checks, Risks.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-gap-artifacts-tests.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
