$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave9_app_server_review'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: merge_wave9_app_server_review

You are an external non-interactive Codex worker in C:\Users\Oleh\Documents\GitHub\open_ai\codex. The repo is already mid-merge of upstream/main into branch slow-context-budget-mode. Root is only overseeing.

OWNERSHIP: Only modify conflicts under codex-rs/app-server/**, especially README.md, src/bespoke_event_handling.rs, src/request_processors.rs, src/request_processors/config_processor.rs, src/request_processors/turn_processor.rs, tests/common/mcp_process.rs, tests/suite/v2/review.rs. Do not modify app-server-protocol, core, tui, Cargo files, scripts, or unrelated docs.

DO_NOT_INSPECT: Avoid broad repo sweeps. Do not run cargo/rustc/just/build/test scripts/schema generation/deploy activation. Do not run git add, commit, merge, rebase, reset, checkout whole repo, or revert unrelated work. You are not alone in this codebase; other workers may edit disjoint files, so do not undo their work.

SCOUT_EVIDENCE: Root grouped current unmerged paths and app-server conflicts are request processing/review/config related. Protocol worker owns shared protocol files; read protocol files only if needed to understand types, but do not edit them.

WHY_AGENT / ROI: App-server request/review conflicts are independent from core/TUI edits. Positive ROI: parallel_gain=3 context_gain=3 repeat_gain=2 loop_followup_gain=3 cost=3 risk=1 net=7.

FIRST_READS: git diff --name-only --diff-filter=U; inspect only assigned app-server files with conflict markers. Use git show :1/:2/:3:path if needed. If a type mismatch depends on protocol files, note it in handoff instead of editing outside ownership.

TASK: Resolve app-server merge conflicts in assigned files. Preserve slow-context-budget behavior and upstream review/config processor changes where compatible. Keep edits minimal and local.

TOOL_HINTS: Use apply_patch for manual edits. Use focused rg only inside codex-rs/app-server for <<<<<<< markers. Avoid expensive commands.

VERIFICATION: Allowed: git diff --check -- codex-rs/app-server; focused rg for conflict markers inside assigned paths. Not allowed: cargo/build/test/schema generation.

HANDOFF: Write .codex/workflow/agents/merge_wave9_app_server_review.handoff.md with status Done or Blocked, files changed, remaining conflict markers if any, and integration notes. Also write .codex/workflow/agents/merge_wave9_app_server_review.files.txt listing modified files. Then exit.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave9_app_server_review.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
