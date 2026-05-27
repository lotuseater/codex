$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave13_core_tools_tasks'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# merge_wave13_core_tools_tasks

CONTEXT_AREA: Resolve the active upstream/main merge conflicts in core tasks/tools files.

DO_NOT_INSPECT: Do not inspect app-server/protocol, core config/session, TUI, or broad repo areas except direct references needed for assigned files. Do not run broad builds/tests, cargo, deploy scripts, schema generation, or git add/commit/merge/rebase.

SCOUT_EVIDENCE: Root recovered `.codex/workflow/ROOT_TASK_HANDOFF.md` after reboot. Current merge is active on branch `slow-context-budget-mode`; `MERGE_HEAD` is upstream/main `14d80e55cd`. Previous handoffs mention core tools/state conflict work (`merge_wave11_core_tools_state_exact.handoff.md`, `merge_wave10_core_tools_state.handoff.md`).

WHY_AGENT / ROI: Independent conflict area; parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=2, cost=3, risk=1, net=6. Root is only overseeing.

FIRST_READS:
- `.codex/workflow/ROOT_TASK_HANDOFF.md`
- `.codex/workflow/agents/root_overseer_handoff.md`
- `.codex/workflow/agents/merge_wave11_core_tools_state_exact.handoff.md` if present
- `.codex/workflow/agents/merge_wave10_core_tools_state.handoff.md` if present
- Assigned files:
  - `codex-rs/core/src/tasks/mod.rs`
  - `codex-rs/core/src/tools/code_mode/execute_handler.rs`
  - `codex-rs/core/src/tools/handlers/extension_tools.rs`
  - `codex-rs/core/src/tools/handlers/mcp.rs`
  - `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
  - `codex-rs/core/src/tools/registry_tests.rs`
  - `codex-rs/core/src/tools/spec_plan_tests.rs`
  - `codex-rs/core/src/tools/spec_plan.rs`
  - `codex-rs/core/src/tools/tool_family/shell.rs`
  - `codex-rs/core/tests/suite/client_websockets.rs`
  - `codex-rs/core/tests/suite/code_mode.rs`
  - `codex-rs/core/tests/suite/compact_remote.rs`

TASK:
1. Inspect assigned paths for conflict markers and unmerged state.
2. Resolve conflicts in the working tree only, preserving current branch slow-context-budget/tool behavior plus upstream/main changes.
3. Treat assigned integration tests as content files only; do not run them.
4. Do not stage files. Root will stage after review.

TOOL_HINTS: Use focused `rg -n "^(<<<<<<<|=======|>>>>>>>)"` on assigned files and `git diff -- <assigned paths>`. Use `git show :1/:2/:3` only for files with non-obvious conflict chunks.

TOKEN_TIP: Use prior handoffs for intent, then direct assigned file reads; avoid repo-wide symbol searches unless blocked.

VERIFICATION: No build/test. Confirm no conflict markers remain in assigned files and summarize diff intent.

HANDOFF: Write `.codex/workflow/agents/merge_wave13_core_tools_tasks.handoff.md` with files changed, unresolved concerns, and root staging/review notes.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave13_core_tools_tasks.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
