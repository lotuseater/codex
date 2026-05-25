$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-research-action-reflection-3'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Self-review feature extension for reviewing the agent''s own actions and strategy. Focus on the new reflective review dimensions: optimality for task/user request, strategy, long-term perspective, delegation/parallelization, automation/scripting, prototyping before broad builds/tests, architecture/SOLID/decoupling/complexity, and planning/structure.

DO_NOT_INSPECT:
Do not run cargo/rustc/npm/build/test/deploy/schema generation. Do not edit product code. Do not duplicate commit-tracking implementation research except where it intersects with prompt/artifact content.

SCOUT_EVIDENCE:
Root verified the existing task memo and handoffs. Existing artifact/test handoff covers artifact persistence broadly; this worker should focus narrowly on how to represent and inject the reflective review content and what artifacts should feed it.

WHY_AGENT / ROI:
This is separable prompt/content architecture research. ROI estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=1, loop_followup_gain=3, risk_penalty=1, net=6.

FIRST_READS:
1. self-review feature.md
2. .codex/workflow/agents/handoffs/self-review-research-artifacts-tests-2.md
3. codex-rs/core/src/review_prompts.rs
4. codex-rs/app-server-protocol/src/protocol/v2.rs
5. codex-rs/app-server/tests/suite/v2/review.rs

TOOL_HINTS:
Search exact names from review_prompts.rs and protocol review types only after reading first files. Keep findings concrete and line-oriented.

TOKEN_TIP:
Write the shortest handoff that lets root implement without rereading every file. Avoid broad summaries.

VERIFICATION:
Use code reading only. Include a proposed final reflection question list and where it should be inserted.

HANDOFF:
Write .codex/workflow/agents/handoffs/self-review-research-action-reflection-3.md with sections:
- Scope
- Existing prompt/artifact model
- Reflection dimensions to add
- Artifact inputs required by code
- Minimal code edit targets
- Suggested prompt wording
- Residual risks
- Commands not run
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-research-action-reflection-3.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
