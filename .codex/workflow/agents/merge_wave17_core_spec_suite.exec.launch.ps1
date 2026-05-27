$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave17_core_spec_suite'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Resolve only spec_plan/tool_family/suite-test merge conflicts after upstream/main merge on branch `slow-context-budget-mode`.

DO_NOT_INSPECT: Do not inspect or edit app-server, app-server-protocol, tui, docs, scripts, config/session files, core handler/tasks files, or unrelated areas. Do not run cargo/rustc/just/build/test/deploy/schema generation.

SCOUT_EVIDENCE: Root verified live merge state at 2026-05-26 06:33 EEST. Remaining conflicts are 21 paths in codex-rs/core. Older wave14/wave15/wave16 workers were stale and stopped before this wave.

WHY_AGENT / ROI: Root is only overseer. This slice covers conflict files with delete/modify cases and spec tests, independent from the other wave17 slices. Use high-quality reasoning; do not delegate further.

FIRST_READS: Start with `git status --porcelain=v1 --untracked-files=no`, then inspect only these owned paths and their index stages (`git show :1:path`, `:2:path`, `:3:path` as useful):
- codex-rs/core/src/tools/spec_plan.rs
- codex-rs/core/src/tools/spec_plan_tests.rs
- codex-rs/core/src/tools/tool_family/shell.rs
- codex-rs/core/tests/suite/client_websockets.rs
- codex-rs/core/tests/suite/code_mode.rs
- codex-rs/core/tests/suite/compact_remote.rs

TOOL_HINTS: Prefer `rg` and small targeted reads. For `UD`/`DU` delete-modify conflicts, inspect nearby references and current/upstream intent: preserve deletion if the file was intentionally removed/replaced, or keep/recreate only when still referenced and needed. Resolve by preserving both current-branch slow-context-budget behavior and upstream/main changes where both still apply. Use `apply_patch` or repo-native safe edits; never reset or checkout the whole tree.

VERIFICATION: For owned paths only: ensure no conflict markers remain, run `git diff --check -- <owned paths>` if possible, and stage only owned resolved files with `git add -- <owned paths>`. Do not run build/tests.

HANDOFF: Write `.codex/workflow/agents/merge_wave17_core_spec_suite.handoff.md` with status, files changed/staged/deleted, unresolved risk, and exact verification commands/results. Keep it concise.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave17_core_spec_suite.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
