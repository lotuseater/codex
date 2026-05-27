$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_stage1_tui_config_worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Pre-merge static review of uncommitted refactors in TUI/session-thread/config-template areas before upstream/main is merged.

DO_NOT_INSPECT: Do not run builds, tests, cargo check, cargo test, or formatters. Do not inspect protocol/core API areas except for names needed by touched TUI/config files. Do not modify .codex/workflow/agents except your handoff file.

SCOUT_EVIDENCE: Root ran first_moves_predict for the upstream/main merge plan and existing wave3 worker handoffs are present. Relevant prior handoffs include solid_refactor_wave3_session_thread_boundary_worker.handoff.md and solid_refactor_wave3_manifest_planner_worker.handoff.md.

WHY_AGENT / ROI: Independent static review can run while other areas are reviewed. new_agent_cost=3, parallel_gain=3, context_gain=2, repeat_gain=1, loop_followup_gain=2, risk_penalty=1, net=4. User requested external non-interactive top-model/high-effort workers.

FIRST_READS: Start with `git status --short --branch --untracked-files=no`, then `git diff -- codex-rs/tui codex-rs/config codex-rs/core/config.schema.json codex-rs/collaboration-mode-templates`. Read solid_refactor_wave3_session_thread_boundary_worker.handoff.md and solid_refactor_wave3_manifest_planner_worker.handoff.md before broad search. Then inspect only touched files in those areas, especially codex-rs/tui/src/app.rs, codex-rs/tui/src/chatwidget.rs, codex-rs/tui/src/bottom_pane/mod.rs, codex-rs/tui/src/bottom_pane/chat_composer.rs, codex-rs/tui/src/bottom_pane/footer.rs, codex-rs/tui/styles.md, codex-rs/config/src/config_toml.rs, codex-rs/config/src/profile_toml.rs, codex-rs/core/config.schema.json, and codex-rs/collaboration-mode-templates/templates/plan.md.

TOOL_HINTS: Use targeted `rg` for symbol references after reading the diff. No build/test commands. If you patch, keep it within the files above and explain why the static evidence is enough.

TOKEN_TIP: Stay on changed lines, nearby structs/enums, imports, and config schema/template consistency.

VERIFICATION: Static review only. Identify likely compile conflicts, stale imports, schema/template mismatch, duplicated plan/delegation text, or merge-sensitive large blocks. If you find a critical small fix in your owned scope, patch it and record the exact file/line. Otherwise do not edit.

HANDOFF: Write .codex/workflow/agents/merge_stage1_tui_config_worker.handoff.md with sections: Scope, Files inspected, Changes made, Findings/blockers, Merge-risk notes, Recommended staging. Keep it concise and explicit whether you edited files.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_stage1_tui_config_worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
