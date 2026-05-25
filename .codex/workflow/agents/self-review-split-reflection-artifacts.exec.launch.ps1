$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-split-reflection-artifacts'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: reflective self-review prompt content plus preserved artifacts: initial user prompts, accepted plan, activity journal.
DO_NOT_INSPECT: Do not build, test, generate schemas, deploy, commit, or edit source files. Only write your handoff file. Avoid broad repository searches.
SCOUT_EVIDENCE: Root confirmed old external workers are not alive and is splitting research lanes. Task memo includes a requested reflection checklist and says artifacts must be preserved by code, not merely suggested by the LLM.
WHY_AGENT / ROI: Artifact/reflection requirements are separable from commit tracking and event flow. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=7.
FIRST_READS:
- `self-review feature.md`
- latest handoffs matching `self-review-research-action-reflection*.md`, `self-review-research-artifacts-tests*.md`, `self-review-sidecar-artifacts-journal.md`
- `codex-rs/app-server-protocol/src/protocol/v2/review.rs`
- `codex-rs/app-server/src/bespoke_event_handling.rs`
- `codex-rs/app-server/src/request_processors/turn_processor.rs`
- `codex-rs/app-server/tests/suite/v2/review.rs`
TOOL_HINTS: Use `rg "plan|journal|prompt|review|activity|initial" codex-rs/app-server codex-rs/app-server-protocol -g ''*.rs''` only after named reads.
TOKEN_TIP: The output should help root write code/tests, not repeat the user prompt.
TASK: Map how to add the expanded own-actions reflection checklist: optimal for current task/request, best strategy, long-term perspective, delegation/parallelization, automation/scripting, prototyping before broad build/tests, architecture/SOLID/decoupling/complexity, and planned/structured activity. Also identify how code should preserve initial user prompts, initial accepted plan, and an activity journal since last review, and how these artifacts should be supplied to the review.
VERIFICATION: No builds/tests. Reason from source and test patterns only.
HANDOFF: Write `.codex/workflow/agents/handoffs/self-review-split-reflection-artifacts.md` with sections: Reflection Prompt Content, Artifact State Needed, Event Capture Points, Code Edit Targets, Tests Later, Risks, Commands Run. Keep under 140 lines.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-split-reflection-artifacts.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
