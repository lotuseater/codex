$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave12_core_config_finalize'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Finalize unresolved merge state for core config files only:
- `codex-rs/core/src/config/config_tests.rs`
- `codex-rs/core/src/config/edit.rs`
- `codex-rs/core/src/config/mod.rs`

DO_NOT_INSPECT:
Do not inspect unrelated large areas. Do not run broad `rg` over the whole repo unless the exact config files require a local symbol lookup. Do not touch app-server, protocol schema, TUI, docs, or non-config core files.

SCOUT_EVIDENCE:
Root handoff `.codex/workflow/ROOT_TASK_HANDOFF.md` says the merge is active on `slow-context-budget-mode`, with 32 unresolved paths and 3 under `codex-rs/core/src/config`. Prior worker handoff `.codex/workflow/agents/merge_wave10_core_config_src.handoff.md` reported these files had no conflict markers but still need root integration.

WHY_AGENT / ROI:
External worker requested by user. Positive ROI because this is a small, separable validation/fix lane that can run while other workers handle larger core/app areas. You are not alone in the codebase; other workers may be resolving other conflict areas. Do not revert or overwrite their work.

FIRST_READS:
1. `.codex/workflow/ROOT_TASK_HANDOFF.md`
2. `.codex/workflow/agents/merge_wave10_core_config_src.handoff.md`
3. `git diff --name-only --diff-filter=U -- codex-rs/core/src/config`
4. The three config files listed in CONTEXT_AREA.

TOOL_HINTS:
Use focused commands only. If conflicts remain, resolve only these files. Preserve both branch multi-agent/config-profile behavior and upstream permission/workspace-root/tool-namespace behavior as described in the prior handoff.

TOKEN_TIP:
Keep reads scoped. Use `git diff -- <file>` and exact symbol searches inside the three files instead of broad exploration.

VERIFICATION:
Allowed only:
- `rg -n "^(<<<<<<<|=======|>>>>>>>)" codex-rs/core/src/config/config_tests.rs codex-rs/core/src/config/edit.rs codex-rs/core/src/config/mod.rs`
- `git diff --check -- codex-rs/core/src/config/config_tests.rs codex-rs/core/src/config/edit.rs codex-rs/core/src/config/mod.rs`

Forbidden:
- cargo/rustc/just/build scripts/tests/schema generation
- staging, committing, merge/rebase/reset/checkout
- deploy or activation

HANDOFF:
Write `.codex/workflow/agents/merge_wave12_core_config_finalize.handoff.md` with files inspected/changed, conflict marker status, verification commands and exits, and any integration notes.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave12_core_config_finalize.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
