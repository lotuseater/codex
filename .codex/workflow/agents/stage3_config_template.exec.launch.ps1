$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: stage3_config_template'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Stage 3 config/template conflict check for slow-context-budget-mode. Focus on codex-rs/config/src/config_toml.rs, codex-rs/config/src/profile_toml.rs, and codex-rs/collaboration-mode-templates/templates/plan.md. HEAD is 14a9f24005; upstream/main is 9f42c89c01. Stage-2 pre-refactor renamed some local context-budget field reads toward upstream-compatible naming and adjusted prompt wording.

DO_NOT_INSPECT: Do not inspect unrelated files. Do not run cargo, rustc, just, build/test scripts, schema generation, or deploy. Do not mutate root checkout except writing C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage3-20260525-181500\stage3_config_template\.handoff\.md. Do not spawn agents.

SCOUT_EVIDENCE: Root first_moves_predict plus stage-2 protocol/template handoffs identified these config and plan-template files as future conflict points.

WHY_AGENT / ROI: Independent semantic review of config and prompt template conflicts can run in parallel and prevents subtle merge resolution errors. Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=2, risk_penalty=1, net=6.

FIRST_READS: git diff -- the three focus files; git show upstream/main:codex-rs/config/src/config_toml.rs for nearby fields if needed; git show upstream/main:codex-rs/collaboration-mode-templates/templates/plan.md for prompt text.

TASK: Check whether current pre-refactors reduce future conflicts and whether more small pre-merge edits are needed before the real merge. If a dry merge is useful, create a temp worktree under the stage3 directory and include the root dirty patch only for the seven codex-rs files listed in stage context. Do not edit. Recommend exact conflict resolution if merge still conflicts.

TOOL_HINTS: Use targeted git diff/show. Avoid broad repo scans.

TOKEN_TIP: Keep handoff to field names, prompt paragraphs, and exact keep/replace guidance.

VERIFICATION: Read/dry-merge only. No build/test.

HANDOFF: Write C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage3-20260525-181500\stage3_config_template\.handoff\.md with findings, recommended resolutions, and any pre-merge edit request.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\stage3_config_template.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
