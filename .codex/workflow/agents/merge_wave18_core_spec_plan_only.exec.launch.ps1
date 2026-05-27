$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave18_core_spec_plan_only'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'You are a fresh external Codex worker. Ignore any prior worker context or old handoff content. Do not read `.codex/workflow/agents/*.handoff.md` except to write your own handoff at the end.

CONTEXT_AREA: Resolve exactly two remaining merge conflicts after upstream/main merge on branch `slow-context-budget-mode`:
- codex-rs/core/src/tools/spec_plan.rs
- codex-rs/core/src/tools/spec_plan_tests.rs

DO_NOT_INSPECT: Do not inspect or edit app-server, app-server-protocol, app-server-transport, config loader tests, guardian tests, suite_order files, tui, docs, scripts, or other core files except for minimal `rg` references needed to understand these two files. Do not edit config/session/handler/task files owned by other active workers. Do not run cargo/rustc/just/build/test/deploy/schema generation.

SCOUT_EVIDENCE: Root poll after wave17 showed current unmerged list still includes only these spec_plan paths from the spec slice. Wave17 spec handoff was inconsistent and must not be trusted.

WHY_AGENT / ROI: This tiny slice is independent and should complete quickly while config/session and tools/tasks workers continue.

FIRST_READS: Run `git status --short -- codex-rs/core/src/tools/spec_plan.rs codex-rs/core/src/tools/spec_plan_tests.rs`, then inspect conflict stages for only those two paths with `git show :1:path`, `git show :2:path`, `git show :3:path` as useful.

TOOL_HINTS: Preserve both current-branch slow-context-budget behavior and upstream/main changes where both apply. Use targeted reads, `rg`, and `apply_patch`. You are not alone in this codebase; never reset or checkout the whole tree.

VERIFICATION: For these two paths only: no conflict markers, `git diff --check -- codex-rs/core/src/tools/spec_plan.rs codex-rs/core/src/tools/spec_plan_tests.rs`, and `git ls-files -u -- <these two paths>` has no output after staging. Stage only these two files.

HANDOFF: Write `.codex/workflow/agents/merge_wave18_core_spec_plan_only.handoff.md` with actual status, files changed/staged, and exact verification output. Keep it concise and factual.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave18_core_spec_plan_only.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
