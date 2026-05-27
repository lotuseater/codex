$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave13_core_config_session'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# merge_wave13_core_config_session

CONTEXT_AREA: Resolve the active upstream/main merge conflicts in core config, hook runtime, session, and state files.

DO_NOT_INSPECT: Do not inspect app-server/protocol, core tools/tasks, TUI, or broad repo areas except direct references needed for assigned files. Do not run broad builds/tests, cargo, deploy scripts, schema generation, or git add/commit/merge/rebase.

SCOUT_EVIDENCE: Root recovered `.codex/workflow/ROOT_TASK_HANDOFF.md` after reboot. Current merge is active on branch `slow-context-budget-mode`; `MERGE_HEAD` is upstream/main `14d80e55cd`. Prior `merge_wave12_core_config_finalize.handoff.md` reported config files likely already resolved but still unmerged.

WHY_AGENT / ROI: Independent conflict area with high integration risk; parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=2, cost=3, risk=1, net=6. Root is only overseeing.

FIRST_READS:
- `.codex/workflow/ROOT_TASK_HANDOFF.md`
- `.codex/workflow/agents/root_overseer_handoff.md`
- `.codex/workflow/agents/merge_wave12_core_config_finalize.handoff.md`
- `.codex/workflow/agents/merge_wave10_core_config_src.handoff.md` if present
- Assigned files:
  - `codex-rs/core/src/config/config_tests.rs`
  - `codex-rs/core/src/config/edit.rs`
  - `codex-rs/core/src/config/mod.rs`
  - `codex-rs/core/src/hook_runtime.rs`
  - `codex-rs/core/src/session/mod.rs`
  - `codex-rs/core/src/session/session.rs`
  - `codex-rs/core/src/session/tests.rs`
  - `codex-rs/core/src/session/turn.rs`
  - `codex-rs/core/src/state/session.rs`

TASK:
1. Inspect assigned paths for conflict markers and unmerged state.
2. Preserve branch slow-context-budget behavior while accepting compatible upstream/main changes.
3. Verify the three config files from the prior handoff still look correct; avoid unnecessary rewrites there.
4. Resolve remaining markers in hook/session/state files in the working tree only.
5. Do not stage files. Root will stage after review.

TOOL_HINTS: Use focused `rg -n "^(<<<<<<<|=======|>>>>>>>)"` on assigned files and `git diff -- <assigned paths>`. For tricky conflict chunks, compare stages with `git show :1:path`, `:2:path`, `:3:path` for only that file.

TOKEN_TIP: Start from the prior config handoff and exact files; avoid broad scans.

VERIFICATION: No build/test. Confirm no conflict markers remain in assigned files and summarize diff intent.

HANDOFF: Write `.codex/workflow/agents/merge_wave13_core_config_session.handoff.md` with files changed, unresolved concerns, and root staging/review notes.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave13_core_config_session.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
