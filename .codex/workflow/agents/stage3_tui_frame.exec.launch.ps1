$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: stage3_tui_frame'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Stage 3 TUI frame requester conflict check for slow-context-budget-mode. Focus on codex-rs/tui/src/tui/frame_requester.rs and any directly necessary references from codex-rs/tui/src/app.rs only if needed. HEAD is 14a9f24005; upstream/main is 9f42c89c01. Stage-2 pre-refactor changed frame_requester toward shared upstream scheduling/drop behavior.

DO_NOT_INSPECT: Do not inspect unrelated TUI areas. Do not run cargo, rustc, just, build/test scripts, schema generation, or deploy. Do not mutate root checkout except writing C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage3-20260525-181500\stage3_tui_frame\.handoff\.md. Do not spawn agents.

SCOUT_EVIDENCE: Root first_moves_predict and stage-2 TUI handoff identified frame_requester as the largest conflict surface after upstream/main changed frame scheduling internals.

WHY_AGENT / ROI: The frame requester has the largest local diff and benefits from focused semantic review while other workers scan manifests/config. Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=3, loop_followup_gain=2, risk_penalty=1, net=7.

FIRST_READS: git diff -- codex-rs/tui/src/tui/frame_requester.rs; git show upstream/main:codex-rs/tui/src/tui/frame_requester.rs; optionally git diff upstream/main...HEAD -- the same path.

TASK: Determine whether current frame_requester pre-refactor is enough to make the real merge clean or easy. If dry merge reveals conflicts, explain exact resolution direction by function/struct field. Preserve this branch''s slow/context-budget behavior while keeping upstream/main scheduling/drop-safety improvements. Do not edit.

TOOL_HINTS: Use targeted git commands and small snippets. If you create a temp worktree, put it under C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage3-20260525-181500\stage3_tui_frame\worktree and include the root dirty patch for focus files.

TOKEN_TIP: Do not paste the full file. Report function-level recommendations.

VERIFICATION: Read/dry-merge only. No build/test.

HANDOFF: Write C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage3-20260525-181500\stage3_tui_frame\.handoff\.md with conflict status, semantic risks, and recommended merge resolution.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\stage3_tui_frame.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
