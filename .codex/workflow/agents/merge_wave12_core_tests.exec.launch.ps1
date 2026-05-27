$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave12_core_tests'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Resolve current unmerged conflicts for core integration tests only:
- `codex-rs/core/tests/suite/client_websockets.rs`
- `codex-rs/core/tests/suite/code_mode.rs`
- `codex-rs/core/tests/suite/compact_remote.rs`

DO_NOT_INSPECT:
Do not touch core/src, app-server, protocol schema, TUI, docs, or unrelated tests except for exact helper references needed by these three files.

SCOUT_EVIDENCE:
Root handoff `.codex/workflow/ROOT_TASK_HANDOFF.md` reports 3 unresolved paths under `codex-rs/core/tests`. Root snapshot identified the three test leaves listed above.

WHY_AGENT / ROI:
External worker requested by user. Positive ROI because these tests can be resolved independently after reading nearby helper usage. You are not alone in the codebase; do not revert or overwrite other workers'' edits.

FIRST_READS:
1. `.codex/workflow/ROOT_TASK_HANDOFF.md`
2. `git diff --name-only --diff-filter=U -- codex-rs/core/tests/suite`
3. The three assigned test files.
4. Only exact helper files referenced from those tests if needed.

TOOL_HINTS:
Resolve syntax/API expectations from the surrounding test patterns. Keep both branch behavior and upstream test updates where compatible.

TOKEN_TIP:
Use conflict chunks first. Avoid reading all test suites.

VERIFICATION:
Allowed only:
- `rg -n "^(<<<<<<<|=======|>>>>>>>)" codex-rs/core/tests/suite/client_websockets.rs codex-rs/core/tests/suite/code_mode.rs codex-rs/core/tests/suite/compact_remote.rs`
- `git diff --check -- codex-rs/core/tests/suite/client_websockets.rs codex-rs/core/tests/suite/code_mode.rs codex-rs/core/tests/suite/compact_remote.rs`

Forbidden:
- cargo/rustc/just/build scripts/tests/schema generation
- staging, committing, merge/rebase/reset/checkout
- deploy or activation

HANDOFF:
Write `.codex/workflow/agents/merge_wave12_core_tests.handoff.md` with files changed, marker status, verification commands/exits, and any follow-up risk.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave12_core_tests.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
