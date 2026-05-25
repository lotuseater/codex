$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-research-event-flow-2'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Codex self-review feature in C:\Users\Oleh\Documents\GitHub\open_ai\codex. The root session has a live main implementation worker (PID 21776) owning product-code changes. Your role is read-only research and clear documentation.

DO_NOT_INSPECT:
Do not run cargo, rustc, npm, build scripts, test scripts, schema generation, deploy, or broad repository sweeps. Do not edit product code. Do not spawn subagents.

SCOUT_EVIDENCE:
Root already verified PID 21776 is alive and responding, and existing handoffs exist under .codex/workflow/agents/handoffs. Prior relevant files include codex-rs/core/src/session/review.rs, codex-rs/core/src/session/handlers.rs, codex-rs/core/src/review_prompts.rs, codex-rs/tui/src/app.rs, codex-rs/app-server/src/request_processors/turn_processor.rs, codex-rs/app-server/tests/suite/v2/review.rs, and self-review feature.md.

WHY_AGENT / ROI:
Parallel read-only event-flow research can proceed while the main worker implements. ROI estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=7.

FIRST_READS:
Read self-review feature.md first. Then read only the self-review event-flow files needed to answer the questions: codex-rs/core/src/session/review.rs, codex-rs/core/src/session/handlers.rs, codex-rs/core/src/review_prompts.rs, and codex-rs/tui/src/app.rs. Use rg only for exact symbols found in those files.

TASK:
Document the current self-review event flow and gaps for the requested behavior:
1. Summary prompt before review.
2. Suggestive user-style review prompt inserted into the session.
3. Reminder prompt after review/actions.
4. Reflection on own actions, strategy, delegation, automation, prototyping, architecture, and planning.
5. Follow-up action/fix prompt when findings are not automatically handled.

TOKEN_TIP:
Keep reads focused. Prefer line references and concise bullets over long quotes.

VERIFICATION:
No builds or tests. Verify by reading code and cross-checking symbol names.

HANDOFF:
Write .codex/workflow/agents/handoffs/self-review-research-event-flow-2.md with sections:
- Scope
- Files read
- Current event flow
- Confirmed implemented behavior
- Missing or risky behavior
- Exact integration points
- Suggested minimal code changes
- Residual risks
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-research-event-flow-2.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
