$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave9_protocol_schema'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: merge_wave9_protocol_schema

You are an external non-interactive Codex worker in C:\Users\Oleh\Documents\GitHub\open_ai\codex. The repo is already mid-merge of upstream/main into branch slow-context-budget-mode. Root is only overseeing.

OWNERSHIP: Only modify these conflict areas: codex-rs/app-server-protocol/schema/json/**, codex-rs/app-server-protocol/src/protocol/**. Do not modify app-server, core, tui, Cargo files, scripts, or unrelated docs.

DO_NOT_INSPECT: Avoid broad repo sweeps. Do not run cargo/rustc/just/build/test scripts/schema generation/deploy activation. Do not run git add, commit, merge, rebase, reset, checkout whole repo, or revert unrelated work. You are not alone in this codebase; other workers may edit disjoint files, so do not undo their work.

SCOUT_EVIDENCE: Root grouped current unmerged paths and found protocol/schema conflicts include schema JSON files plus protocol thread_history.rs, v1.rs, v2/config.rs, v2/item.rs, v2/review.rs, v2/tests.rs. Previous handoffs indicated app protocol review changes were a likely conflict area.

WHY_AGENT / ROI: This is a bounded conflict slice that can be resolved independently while other workers handle core/TUI/app-server. Positive ROI: parallel_gain=3 context_gain=3 repeat_gain=2 loop_followup_gain=3 cost=3 risk=1 net=7.

FIRST_READS: git diff --name-only --diff-filter=U; git diff --check -- codex-rs/app-server-protocol; inspect only assigned files with conflict markers. Use git show :1/:2/:3:path if needed to compare base/ours/theirs. Prefer reading exact assigned files over rg.

TASK: Resolve or reduce merge conflicts in assigned protocol/schema files. Keep branch-local slow-context-budget behavior and upstream additions where compatible. For generated JSON schema files, do NOT regenerate; resolve markers consistently with source intent when clear, otherwise leave a precise blocker in handoff.

TOOL_HINTS: Use apply_patch for manual edits. Use focused rg only inside assigned paths for <<<<<<< markers. Avoid expensive commands.

VERIFICATION: Allowed: git diff --check -- assigned paths; focused rg for conflict markers inside assigned paths. Not allowed: cargo/build/test/schema generation.

HANDOFF: Write .codex/workflow/agents/merge_wave9_protocol_schema.handoff.md with status Done or Blocked, files changed, remaining conflict markers if any, and any integration notes. Also write .codex/workflow/agents/merge_wave9_protocol_schema.files.txt listing modified files. Then exit.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave9_protocol_schema.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
