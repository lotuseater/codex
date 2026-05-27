$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave9_tui'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: merge_wave9_tui

You are an external non-interactive Codex worker in C:\Users\Oleh\Documents\GitHub\open_ai\codex. The repo is already mid-merge of upstream/main into branch slow-context-budget-mode. Root is only overseeing.

OWNERSHIP: Only modify TUI conflicts: codex-rs/tui/** and top-level tui/**. Do not modify core, app-server, protocol, Cargo files, scripts, or unrelated docs.

DO_NOT_INSPECT: Avoid broad repo sweeps. Do not run cargo/rustc/just/build/test scripts/schema generation/deploy activation. Do not run git add, commit, merge, rebase, reset, checkout whole repo, or revert unrelated work. You are not alone in this codebase; other workers may edit disjoint files, so do not undo their work.

SCOUT_EVIDENCE: Root grouped current unmerged paths and earlier handoffs named TUI closeout as pending. Prior inventories included codex-rs/tui/src/app.rs, bottom_pane files, chatwidget, styles, app/event_dispatch.rs, chatwidget/tests/helpers.rs, tui/frame_requester.rs, plus top-level tui/src files.

WHY_AGENT / ROI: TUI conflicts are broad but independent from protocol/core/app-server. Positive ROI: parallel_gain=3 context_gain=3 repeat_gain=2 loop_followup_gain=3 cost=3 risk=1 net=7.

FIRST_READS: git diff --name-only --diff-filter=U; inspect only assigned TUI files. Use git show :1/:2/:3:path for conflicted files. Read .codex/workflow/agents/merge_wave8_tui_closeout.handoff.md if present; if missing, proceed from current files.

TASK: Resolve TUI merge conflicts. Preserve slow-context-budget UI behavior and upstream TUI changes where compatible. Keep edits minimal and local.

TOOL_HINTS: Use apply_patch for manual edits. Use focused rg only inside assigned paths for <<<<<<< markers. Avoid expensive commands.

VERIFICATION: Allowed: git diff --check -- assigned TUI paths; focused rg for conflict markers inside assigned paths. Not allowed: cargo/build/test/schema generation.

HANDOFF: Write .codex/workflow/agents/merge_wave9_tui.handoff.md with status Done or Blocked, files changed, remaining conflict markers if any, and integration notes. Also write .codex/workflow/agents/merge_wave9_tui.files.txt listing modified files. Then exit.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave9_tui.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
