$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self_review_sidecar_prompt_flow'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Self-review prompt flow, suggestive insertion, pre-review summary/reminder, and act-on-findings prompt.

DO_NOT_INSPECT: Do not read large worker logs except short tails. Do not run cargo, rustc, npm, build scripts, schema generation, deployment, or broad tests. Do not edit source code. Do not spawn more workers.

SCOUT_EVIDENCE: Root confirmed external main implementation worker PID 21776 is alive. Task memo is self-review feature.md. Search hits show agent-policy contains AutoLoopSubmissionContext::AfterSelfReview and diff_reviewer prompting surfaces.

WHY_AGENT / ROI: Prompt-flow research is separable from tracking/commit scope and can produce a compact design handoff while implementation continues. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=7.

FIRST_READS: Read self-review feature.md, codex-rs/agent-policy/src/lib.rs, codex-rs/agent-policy/src/plan_prompt.rs. Then targeted rg for AutoLoopSubmissionContext, diff_reviewer, after_self_review, Review, and prompt insertion/session message paths.

TASK: Design the prompt flow changes: (1) self-review is suggestive and inserted like a user prompt in session, (2) before review insert prompt: "please 1. sum-up your recent changes, 2. write your next plans 3. next actions to do", remember answer by code, (3) after review/actions reinsert reminder prompt: "Please resume your before-review tasks. Here is the reminder about them: <...>", (4) add prompt to fix review findings if Codex does not fix them automatically, (5) extend self-review to ask reflection on optimality, strategy, long-term perspective, delegation/parallelization, automation/scripting, prototyping, architecture/SOLID/decoupling/complexity, and structured planning.

TOOL_HINTS: Prefer exact symbol reads over broad scans. No builds/tests.

TOKEN_TIP: Produce a design that can be implemented directly, not an essay.

VERIFICATION: Source-only reasoning is enough. Mention exact tests/builds intentionally not run.

HANDOFF: Write .codex/workflow/agents/handoffs/self-review-sidecar-prompt-flow.md with: current flow, proposed prompt order, exact strings/where inserted, how to detect/trigger act-on-findings, and risks. Final answer should only say whether the handoff was written and list the top 3 files.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self_review_sidecar_prompt_flow.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
