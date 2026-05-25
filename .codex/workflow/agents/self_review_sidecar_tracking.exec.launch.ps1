$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self_review_sidecar_tracking'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Self-review feature, current-agent change/commit tracking and review scoping.

DO_NOT_INSPECT: Do not read the 46MB self-review-feature-main-worker.exec.log except with a short tail if absolutely needed. Do not run cargo, rustc, npm, build scripts, schema generation, deployment, or broad tests. Do not edit source code. Do not spawn more workers.

SCOUT_EVIDENCE: Root confirmed no internal agents are live and external main worker PID 21776 is alive with self-review-feature-main-worker.exec.log recently updated. Root rg found self-review surfaces mainly under codex-rs/agent-policy plus likely core/tui event paths. Main task memo is self-review feature.md.

WHY_AGENT / ROI: Independent read-only research can run while the live main worker owns implementation. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=7. Your work should return a compact handoff for root/main worker.

FIRST_READS: Read self-review feature.md, AGENTS.md, codex-rs/agent-policy/src/lib.rs, codex-rs/agent-policy/src/plan_prompt.rs. Then use targeted rg for symbols related to session/user changes, git commits, modified files, review invocation, and auto-loop review scope. If the first reads point elsewhere, inspect only the minimal exact files.

TASK: Map how to implement code-owned tracking of files and commits changed by the current session since the last review. Include how to include files modified by shell commands in the current session, including files dirty before the command, and how to clear remembered paths/commits after review. Identify where current review incorrectly uses broad working-tree/older changes.

TOOL_HINTS: Prefer rg and direct file reads. Use workflow_batch for bounded file reads if helpful. No broad builds/tests.

TOKEN_TIP: Stop after enough detail for root to act. Avoid pasting code unless a tiny snippet is essential.

VERIFICATION: Source-only reasoning is enough. Mention exact tests/builds intentionally not run.

HANDOFF: Write .codex/workflow/agents/handoffs/self-review-sidecar-tracking.md with: summary, exact relevant files/functions, proposed implementation steps, edge cases, and residual questions. Final answer should only say whether the handoff was written and list the top 3 files.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self_review_sidecar_tracking.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
