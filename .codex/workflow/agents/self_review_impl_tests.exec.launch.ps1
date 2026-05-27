$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self_review_impl_tests'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Self-review protocol/app-server tests and review-plan wording.

DO_NOT_INSPECT: Do not inspect unrelated handoffs or broad unrelated modules. Read only self-review handoffs under .codex/workflow/agents/handoffs/self-review*.md as needed.

SCOUT_EVIDENCE: Root checked first_moves and self-review handoffs. High-value files include codex-rs/app-server-protocol/src/protocol/v2/review.rs, codex-rs/app-server/src/request_processors/turn_processor.rs, codex-rs/app-server/tests/suite/v2/review.rs, and codex-rs/collaboration-mode-templates/templates/plan.md.

WHY_AGENT / ROI: Protocol/test wording is independent from core event wiring. ROI estimate: cost=3, parallel_gain=3, context_gain=2, repeat_gain=1, loop_followup_gain=2, risk_penalty=1, net=4.

FIRST_READS:
- .codex/workflow/agents/handoffs/self-review-gap-artifacts-tests.md
- .codex/workflow/agents/handoffs/self-review-research-artifacts-tests-2.md
- .codex/workflow/agents/handoffs/self-review-split-reflection-artifacts.md
- codex-rs/app-server-protocol/src/protocol/v2/review.rs
- codex-rs/app-server/src/request_processors/turn_processor.rs
- codex-rs/app-server/tests/suite/v2/review.rs
- codex-rs/collaboration-mode-templates/templates/plan.md

OWNERSHIP:
- You may edit app-server review protocol/tests and collaboration-mode plan template.
- You may add/adjust tests around review request conversion/prompt fields.
- Do not edit core event-wiring files unless a compile error from your owned tests requires a narrow import/schema fix.
- You are not alone in the codebase. Do not revert or overwrite edits made by others; if a nearby change appears, adapt and leave it intact.

TASK:
Add/adjust tests and plan wording for the improved self-review feature:
1. Extend plan/self-review wording so review of own actions reflects on current-task fit, strategy, long-term perspective, delegation/parallelization, automation/scripting, prototyping before broad builds/tests, architecture/SOLID/decoupling/quality/complexity, and planning structure.
2. Ensure protocol/app-server surfaces can carry any new self-review prompt/context fields without including unrelated working tree changes.
3. Add tests that verify self-review context is agent-scoped/suggestive where the existing protocol permits it, or add TODO-free test scaffolding that root can wire once core fields exist.

TOOL_HINTS: Use targeted cargo tests for app-server suite if practical. Keep plan-template edits surgical.

VERIFICATION:
- Run cargo fmt if Rust changed.
- Run focused app-server/protocol tests if practical.

HANDOFF:
Write .codex/workflow/agents/handoffs/self-review-impl-tests.md with:
- changed files
- protocol/test assertions added
- plan-template wording changes
- tests run and results
- integration notes for root
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self_review_impl_tests.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
