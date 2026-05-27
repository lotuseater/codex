$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave9_core_session_hooks'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: merge_wave9_core_session_hooks

You are an external non-interactive Codex worker in C:\Users\Oleh\Documents\GitHub\open_ai\codex. The repo is already mid-merge of upstream/main into branch slow-context-budget-mode. Root is only overseeing.

OWNERSHIP: Only modify non-config core runtime conflicts: codex-rs/core/src/session/**, codex-rs/core/src/hook_runtime.rs, codex-rs/core-test-suites/** conflicts that touch agents/hooks/session behavior, and codex-rs/codex-mcp/src/mcp_connection_manager.rs if still unmerged. Do not modify core/src/config, app-server, protocol, tui, Cargo files, scripts, or unrelated docs.

DO_NOT_INSPECT: Avoid broad repo sweeps. Do not run cargo/rustc/just/build/test scripts/schema generation/deploy activation. Do not run git add, commit, merge, rebase, reset, checkout whole repo, or revert unrelated work. You are not alone in this codebase; other workers may edit disjoint files, so do not undo their work.

SCOUT_EVIDENCE: Root grouped current unmerged paths and previous handoffs flagged core session/turn/hook runtime as pending. Current inventory includes hook_runtime.rs, session/mod.rs, session/session.rs, session/tests.rs, session/turn.rs and core test-suite files.

WHY_AGENT / ROI: Runtime conflicts are high-context but separable from config/TUI/app-server. Positive ROI: parallel_gain=3 context_gain=3 repeat_gain=2 loop_followup_gain=3 cost=3 risk=1 net=7.

FIRST_READS: git diff --name-only --diff-filter=U; inspect assigned files only. Use git show :1/:2/:3:path for conflicted files and nearby exact tests if needed. Read previous .codex/workflow/agents/merge_wave7_core_runtime_triage.handoff.md if present for context, but do not trust it over current files.

TASK: Resolve non-config core runtime merge conflicts. Preserve slow-context-budget behavior and upstream session/hook improvements where compatible. Keep edits minimal and local.

TOOL_HINTS: Use apply_patch for manual edits. Use focused rg only inside assigned paths for <<<<<<< markers. Avoid expensive commands.

VERIFICATION: Allowed: git diff --check -- assigned paths; focused rg for conflict markers inside assigned paths. Not allowed: cargo/build/test/schema generation.

HANDOFF: Write .codex/workflow/agents/merge_wave9_core_session_hooks.handoff.md with status Done or Blocked, files changed, remaining conflict markers if any, and integration notes. Also write .codex/workflow/agents/merge_wave9_core_session_hooks.files.txt listing modified files. Then exit.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave9_core_session_hooks.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
