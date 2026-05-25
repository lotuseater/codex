$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-research-tracking-commit-2'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Codex self-review feature in C:\Users\Oleh\Documents\GitHub\open_ai\codex. The root session has a live main implementation worker (PID 21776) owning product-code changes. Your role is read-only research and clear documentation.

DO_NOT_INSPECT:
Do not run cargo, rustc, npm, build scripts, test scripts, schema generation, deploy, or broad repository sweeps. Do not edit product code. Do not spawn subagents.

SCOUT_EVIDENCE:
Root already verified PID 21776 is alive and responding. The task requires code-owned tracking of changed files and commits since last review, including files modified by the current session even when they were dirty before, plus code-owned auto-commits before/after review that include tracked changes and untracked code files.

WHY_AGENT / ROI:
Tracking and commit scope is independent from prompt/event-flow research and can be documented in parallel. ROI estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=7.

FIRST_READS:
Read self-review feature.md first. Then read codex-rs/core/src/session/review.rs and codex-rs/core/src/session/handlers.rs. If needed, use exact rg searches for git/status/commit tracking symbols in codex-rs/core/src only.

TASK:
Document how current code tracks self-review scope and what remains for:
1. File paths changed by the current agent since last review, code-owned and then forgotten after review.
2. Git commits made by the current agent since last review, code-owned and then forgotten after review.
3. Including all files modified by the current session, even if they were already dirty before the command.
4. Auto-committing all changed files before review.
5. Auto-committing all changed files after review.
6. Including untracked code files in those commits for Python, Rust, C/C++, batch, PowerShell, JavaScript, PHP, Java, Kotlin, Scala, Swift, Objective-C, C#, and Prolog.

TOKEN_TIP:
Avoid broad git archaeology. Use git status only if it helps identify current touched paths; do not attribute authorship manually.

VERIFICATION:
No builds or tests. Verify by reading code paths and exact extension filters.

HANDOFF:
Write .codex/workflow/agents/handoffs/self-review-research-tracking-commit-2.md with sections:
- Scope
- Files read
- Current tracking model
- Current auto-commit model
- Extension coverage
- Confirmed implemented behavior
- Missing or risky behavior
- Suggested minimal code changes
- Residual risks
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-research-tracking-commit-2.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
