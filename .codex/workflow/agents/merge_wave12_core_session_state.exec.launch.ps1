$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave12_core_session_state'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Resolve current unmerged conflicts for core session/state ownership:
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/session.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/state/session.rs`
- If still listed by `git diff --name-only --diff-filter=U`, also handle top-level core runtime file(s) directly under `codex-rs/core/src`.

DO_NOT_INSPECT:
Do not touch config, app-server, protocol schema, TUI, docs, or core tools except for exact local imports needed by assigned files.

SCOUT_EVIDENCE:
Root handoff `.codex/workflow/ROOT_TASK_HANDOFF.md` reports 18 unresolved paths under `codex-rs/core/src`; root grouping found 4 under `core/src/session` and 1 under `core/src/state`. Earlier runtime context is in `.codex/workflow/agents/merge_wave7_core_runtime_triage.handoff.md`.

WHY_AGENT / ROI:
External worker requested by user. Positive ROI because session/state conflicts are cohesive and can be resolved independently while other workers handle config/tools/app areas. You are not alone in the codebase; do not revert or overwrite other workers'' edits.

FIRST_READS:
1. `.codex/workflow/ROOT_TASK_HANDOFF.md`
2. `.codex/workflow/agents/merge_wave7_core_runtime_triage.handoff.md`
3. `git diff --name-only --diff-filter=U -- codex-rs/core/src/session codex-rs/core/src/state codex-rs/core/src`
4. Assigned files from the resulting list.

TOOL_HINTS:
Use focused `git diff -- <file>` and exact imports/symbol lookups. Keep branch slow-context-budget behavior while preserving upstream main changes. If a top-level `core/src` file is outside session/state and looks unrelated, note it in handoff instead of expanding scope.

TOKEN_TIP:
Do not read whole modules unless needed. Prefer conflict chunks and nearby functions.

VERIFICATION:
Allowed only:
- `rg -n "^(<<<<<<<|=======|>>>>>>>)" <assigned files>`
- `git diff --check -- <assigned files>`

Forbidden:
- cargo/rustc/just/build scripts/tests/schema generation
- staging, committing, merge/rebase/reset/checkout
- deploy or activation

HANDOFF:
Write `.codex/workflow/agents/merge_wave12_core_session_state.handoff.md` with assigned files, changes made, marker status, verification commands/exits, and any files intentionally left for root/other workers.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave12_core_session_state.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
