$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-research-event-flow-3'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Self-review feature event/prompt flow in the Codex repo. Focus on how the TUI/app-server/core pipeline should sequence the new self-review prompts: pre-review summary/plans/actions prompt, suggestive review prompt, optional fix prompt, then reminder prompt with the remembered pre-review answer.

DO_NOT_INSPECT:
Do not run cargo/rustc/npm/build/test/deploy/schema generation. Do not edit product code. Do not duplicate the main implementation worker''s broad work. Do not scan the whole repo unless the named first reads are insufficient.

SCOUT_EVIDENCE:
Root already verified live external workers and existing handoffs under .codex/workflow/agents/handoffs. Existing handoffs cover tracking/commit and artifacts/tests, but the newer event-flow handoff is missing.

WHY_AGENT / ROI:
Bounded read-only event-flow research can run in parallel while the main/root integration work proceeds. ROI estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=1, loop_followup_gain=3, risk_penalty=1, net=6.

FIRST_READS:
1. self-review feature.md
2. .codex/workflow/agents/handoffs/self-review-gap-event-flow.md
3. codex-rs/core/src/review_prompts.rs
4. codex-rs/core/src/session/handlers.rs
5. codex-rs/app-server/src/bespoke_event_handling.rs
6. codex-rs/app-server/src/request_processors/turn_processor.rs
7. codex-rs/app-server/tests/suite/v2/review.rs

TOOL_HINTS:
Use rg only for exact symbols found in first reads, such as review prompt names and self_review identifiers. Keep the output small. Prefer line-specific findings and minimal implementation suggestions.

TOKEN_TIP:
Stop once you can write a useful handoff. Do not narrate exploration. Do not run build/test.

VERIFICATION:
Verify only by code reading and, if useful, a tiny text-only simulation of the intended prompt order in the handoff. Do not execute product tests.

HANDOFF:
Write .codex/workflow/agents/handoffs/self-review-research-event-flow-3.md with sections:
- Scope
- Current event flow
- Required prompt sequence
- Missing or risky behavior
- Minimal code edit targets
- Suggested prompt text/ordering
- Residual risks
- Commands not run
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-research-event-flow-3.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
