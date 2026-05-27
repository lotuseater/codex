$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave10_core_tools_state'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: merge_wave10_core_tools_state

You are an external non-interactive Codex worker in C:\Users\Oleh\Documents\GitHub\open_ai\codex. The repo is mid-merge of upstream/main into branch slow-context-budget-mode. Root is only overseeing.

OWNERSHIP: Only modify these remaining core tools/state unmerged files:
- codex-rs/core/src/state/session.rs
- codex-rs/core/src/tools/code_mode/execute_handler.rs
- codex-rs/core/src/tools/handlers/extension_tools.rs
- codex-rs/core/src/tools/handlers/mcp.rs
- codex-rs/core/src/tools/handlers/request_plugin_install.rs
- codex-rs/core/src/tools/registry_tests.rs
- codex-rs/core/src/tools/spec_plan.rs
- codex-rs/core/src/tools/spec_plan_tests.rs
- codex-rs/core/src/tools/tool_family/shell.rs
- codex-rs/core/tests/suite/client_websockets.rs
- codex-rs/core/tests/suite/code_mode.rs
- codex-rs/core/tests/suite/compact_remote.rs
Do not modify config, session/turn/hook files, app-server, protocol, tui, Cargo files, scripts, or unrelated docs.

DO_NOT_INSPECT: Avoid broad repo sweeps. Do not run cargo/rustc/just/build/test scripts/schema generation/deploy activation. Do not run git add, commit, merge, rebase, reset, checkout whole repo, or revert unrelated work. You are not alone in this codebase; other workers may edit disjoint files, so do not undo their work.

SCOUT_EVIDENCE: Root grouped remaining unmerged files after wave9 and found exactly these 12 unowned core tools/state files outside protocol/app-server/config/runtime/TUI ownership.

WHY_AGENT / ROI: These are unowned leftovers that can run in parallel with protocol/app-server/runtime workers. Positive ROI: parallel_gain=3 context_gain=2 repeat_gain=2 loop_followup_gain=3 cost=3 risk=1 net=6.

FIRST_READS: git diff --name-only --diff-filter=U -- the assigned paths. Inspect only assigned files and use git show :1/:2/:3:path if needed.

TASK: Resolve merge conflicts in assigned core tools/state files. Preserve slow-context-budget behavior and upstream additions where compatible. Keep edits minimal and local.

TOOL_HINTS: Use apply_patch for manual edits. Use focused rg only on assigned files for <<<<<<< markers. Avoid expensive commands.

VERIFICATION: Allowed: git diff --check -- assigned paths; focused rg for conflict markers in assigned files. Not allowed: cargo/build/test/schema generation.

HANDOFF: Write .codex/workflow/agents/merge_wave10_core_tools_state.handoff.md with status Done or Blocked, files changed, remaining markers if any, and integration notes. Also write .codex/workflow/agents/merge_wave10_core_tools_state.files.txt listing modified files. Then exit.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave10_core_tools_state.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
