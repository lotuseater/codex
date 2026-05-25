$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self_review_sidecar_commit_scope'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Self-review auto-commit behavior and code-like untracked file inclusion.

DO_NOT_INSPECT: Do not read large worker logs except short tails. Do not run cargo, rustc, npm, build scripts, schema generation, deployment, or broad tests. Do not edit source code. Do not spawn more workers.

SCOUT_EVIDENCE: User requested self-review prefer committing by code: (1) all changed files before review, (2) all changed files after review, including untracked code files: Python, Rust, C/C++, bat, ps1, JavaScript, PHP, Java, Kotlin, Scala, Swift, Objective-C, C#, Prolog. Main implementation worker is alive, so this sidecar is read-only research.

WHY_AGENT / ROI: Commit-scope logic is isolated enough for parallel research and reduces root/main-worker context load. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=3, loop_followup_gain=3, risk_penalty=1, net=8.

FIRST_READS: Read self-review feature.md. Targeted rg for git commit helpers, add/stage logic, untracked handling, repo status parsing, session commits, and auto-review in codex-rs/core and codex-rs/agent-policy. Start with: rg -n "git commit|git add|untracked|status --short|commit" codex-rs/core codex-rs/agent-policy -g "*.rs".

TASK: Identify where to implement automatic pre-review/post-review commits by code rather than by LLM prompt. Specify exact code-like untracked file extension list and any path safety exclusions. Clarify how commits should be associated with the current agent/session and how to avoid spending LLM tokens on commit mechanics.

TOOL_HINTS: Use rg and small reads. No builds/tests.

TOKEN_TIP: Keep it implementation-oriented.

VERIFICATION: Source-only reasoning is enough. Mention exact tests/builds intentionally not run.

HANDOFF: Write .codex/workflow/agents/handoffs/self-review-sidecar-commit-scope.md with: relevant files/functions, extension list, proposed algorithm, commit message strategy, failure handling, and risks. Final answer should only say whether the handoff was written and list the top 3 files.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self_review_sidecar_commit_scope.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
