$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self_review_sidecar_artifacts_journal'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Preserved review artifacts: initial user prompts, accepted initial plan, and activity journal since last review.

DO_NOT_INSPECT: Do not read large worker logs except short tails. Do not run cargo, rustc, npm, build scripts, schema generation, deployment, or broad tests. Do not edit source code. Do not spawn more workers.

SCOUT_EVIDENCE: User requested self-review include useful artifacts preserved by code, not LLM suggestion: initial user prompts, initial accepted plan, and an activity journal since last review. Activity journal may be separate file. Main worker is alive; this sidecar researches design and documentation only.

WHY_AGENT / ROI: Artifact persistence touches different event/session state than commit scoping, so parallel research should save root/main-worker context. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=7.

FIRST_READS: Read self-review feature.md, codex-rs/agent-policy/src/lib.rs, and targeted files that define conversation/session history, plan acceptance, message insertion, and event logging. Use rg for "plan", "accepted", "activity", "journal", "Conversation", "Session", and "Op" in codex-rs/core and codex-rs/tui.

TASK: Design how code should capture and preserve: (a) initial user prompts, (b) initial accepted plan, (c) activity journal since last review. Include how the journal should be reset/rolled after review, how it should be added to the self-review prompt, and where it should live if stored in a separate file.

TOOL_HINTS: Prefer targeted rg and direct reads. No builds/tests.

TOKEN_TIP: Favor concrete data structures and event-hook points.

VERIFICATION: Source-only reasoning is enough. Mention exact tests/builds intentionally not run.

HANDOFF: Write .codex/workflow/agents/handoffs/self-review-sidecar-artifacts-journal.md with: current event sources, proposed storage shape, prompt assembly changes, reset semantics, and risks. Final answer should only say whether the handoff was written and list the top 3 files.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self_review_sidecar_artifacts_journal.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
