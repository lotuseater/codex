$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave17_core_tools_tasks'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Resolve only core tools/tasks/handler merge conflicts after upstream/main merge on branch `slow-context-budget-mode`.

DO_NOT_INSPECT: Do not inspect or edit app-server, app-server-protocol, tui, docs, scripts, config/session files, spec_plan files, suite tests, or unrelated areas. Do not run cargo/rustc/just/build/test/deploy/schema generation.

SCOUT_EVIDENCE: Root verified live merge state at 2026-05-26 06:33 EEST. Remaining conflicts are 21 paths in codex-rs/core. Older wave14/wave15/wave16 workers were stale and stopped before this wave.

WHY_AGENT / ROI: Root is only overseer. This handler/tasks slice is independent from config/session and spec/suite. Use high-quality reasoning; do not delegate further.

FIRST_READS: Start with `git status --porcelain=v1 --untracked-files=no`, then inspect only these owned paths and their index stages (`git show :1:path`, `:2:path`, `:3:path` as useful):
- codex-rs/core/src/tasks/mod.rs
- codex-rs/core/src/tools/code_mode/execute_handler.rs
- codex-rs/core/src/tools/handlers/extension_tools.rs
- codex-rs/core/src/tools/handlers/mcp.rs
- codex-rs/core/src/tools/handlers/request_plugin_install.rs
- codex-rs/core/src/tools/registry_tests.rs

TOOL_HINTS: Prefer `rg` and small targeted reads. Resolve conflicts by preserving both current-branch slow-context-budget behavior and upstream/main changes. Use `apply_patch` or repo-native safe edits; do not revert unrelated changes. You are not alone in this codebase; other workers may stage their files, so never reset or checkout the whole tree.

VERIFICATION: For owned paths only: ensure no conflict markers remain, run `git diff --check -- <owned paths>` if possible, and stage only owned resolved files with `git add -- <owned paths>`. Do not run build/tests.

HANDOFF: Write `.codex/workflow/agents/merge_wave17_core_tools_tasks.handoff.md` with status, files changed/staged, unresolved risk, and exact verification commands/results. Keep it concise.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave17_core_tools_tasks.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
