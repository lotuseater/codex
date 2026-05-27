$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self_review_impl_core'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Self-review core session logic for the Codex Rust repo.

DO_NOT_INSPECT: Do not inspect unrelated handoffs or broad unrelated modules. Read only self-review handoffs under .codex/workflow/agents/handoffs/self-review*.md as needed.

SCOUT_EVIDENCE: Root checked first_moves for the self-review task and existing self-review handoffs. High-value files include codex-rs/core/src/guardian/review_session.rs, codex-rs/core/src/guardian/review.rs, codex-rs/protocol/src/protocol.rs, and app-server review protocol/tests.

WHY_AGENT / ROI: This slice is independent enough to reduce root context and can run while other workers handle event wiring/tests. ROI estimate: cost=3, parallel_gain=3, context_gain=3, repeat_gain=1, loop_followup_gain=2, risk_penalty=2, net=4.

FIRST_READS:
- .codex/workflow/agents/handoffs/self-review-sidecar-prompt-flow.md
- .codex/workflow/agents/handoffs/self-review-split-event-flow.md
- .codex/workflow/agents/handoffs/self-review-research-event-flow-3.md
- codex-rs/core/src/guardian/review_session.rs
- codex-rs/core/src/guardian/review.rs
- codex-rs/protocol/src/protocol.rs

OWNERSHIP:
- You may edit codex-rs/core/src/guardian/review_session.rs.
- You may add a small adjacent module under codex-rs/core/src/guardian/ only if it materially improves separation.
- Do not edit app-server files, Cargo files, or unrelated guardian review logic.
- You are not alone in the codebase. Do not revert or overwrite edits made by others; if a nearby change appears, adapt and leave it intact.

TASK:
Implement the core self-review flow so it is suggestive and task-preserving:
1. Add code-owned per-agent review tracking state in the review session area for changed file paths and git commits since last review. It must expose narrow methods for recording changed files/commits and for taking/clearing the pending review context after a review cycle.
2. Build prompt text for the three inserted user-style prompts:
   - pre-review summary prompt: "please 1. sum-up your recent changes, 2. write your next plans 3. next actions to do"
   - self-review prompt that scopes review to current-agent changed files/commits and asks to review own actions/strategy/delegation/automation/prototyping/architecture/planning.
   - resume reminder prompt: "Please resume your before-review tasks. Here is the reminder about them: <...>"
3. Ensure review findings are acted upon by adding a follow-up fix prompt after findings when the agent did not automatically fix review findings.
4. Preserve APIs so event wiring can call record_changed_file/record_git_commit without depending on prompt internals.

TOOL_HINTS: Keep changes local and small. Use rg and targeted reads. If adding prompt builders, unit-test them in the same file or nearby module if feasible.

VERIFICATION:
- Run cargo fmt only if you edited Rust.
- Run the narrowest relevant unit test(s) if they already exist or you add them.
- Do not run broad debug Cargo builds.

HANDOFF:
Write .codex/workflow/agents/handoffs/self-review-impl-core.md with:
- changed files
- APIs added/changed
- exact prompt ordering implemented
- tests run and results
- integration notes for root
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self_review_impl_core.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
