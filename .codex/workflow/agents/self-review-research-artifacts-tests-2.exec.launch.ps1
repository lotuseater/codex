$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-research-artifacts-tests-2'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Codex self-review feature in C:\Users\Oleh\Documents\GitHub\open_ai\codex. The root session has a live main implementation worker (PID 21776) owning product-code changes. Your role is read-only research and clear documentation.

DO_NOT_INSPECT:
Do not run cargo, rustc, npm, build scripts, test scripts, schema generation, deploy, or broad repository sweeps. Do not edit product code. Do not spawn subagents.

SCOUT_EVIDENCE:
Root already verified PID 21776 is alive and responding. The user asked that review artifacts be preserved by code, not LLM suggestion: initial user prompts, initial accepted plan, and activity journal since last review.

WHY_AGENT / ROI:
Artifact/test mapping is independent from event-flow and tracking research, and can run in parallel while implementation continues. ROI estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=7.

FIRST_READS:
Read self-review feature.md first. Then read codex-rs/app-server/tests/suite/v2/review.rs, codex-rs/core/src/session/review.rs, codex-rs/app-server/src/request_processors/turn_processor.rs, and codex-rs/app-server/src/bespoke_event_handling.rs only as needed.

TASK:
Document the artifact preservation and focused test gaps for:
1. Initial user prompts preserved by code.
2. Initial accepted plan preserved by code.
3. Activity journal since last review preserved by code.
4. Self-review prompt includes those artifacts.
5. Tests or simulations that can verify prompt sequence and scope without broad builds.

TOKEN_TIP:
Focus on existing review tests and event types. Propose the smallest test updates; do not implement them.

VERIFICATION:
No builds or tests. Verify by reading test code and existing review event assertions.

HANDOFF:
Write .codex/workflow/agents/handoffs/self-review-research-artifacts-tests-2.md with sections:
- Scope
- Files read
- Current artifact model
- Existing tests
- Missing tests or simulations
- Suggested minimal test changes
- Residual risks
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-research-artifacts-tests-2.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
