$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave12_core_tools_tasks'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Resolve current unmerged conflicts for core tools/tasks ownership:
- `codex-rs/core/src/tools/**`
- `codex-rs/core/src/tasks/mod.rs`
- `codex-rs/core/src/tasks/review.rs` only if still listed as unmerged

Known unresolved leaves from root snapshot include:
- `codex-rs/core/src/tools/code_mode/execute_handler.rs`
- `codex-rs/core/src/tools/handlers/extension_tools.rs`
- `codex-rs/core/src/tools/handlers/mcp.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/core/src/tools/registry_tests.rs`
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/spec_plan_tests.rs`
- `codex-rs/core/src/tools/tool_family/shell.rs`
- `codex-rs/core/src/tasks/mod.rs`

DO_NOT_INSPECT:
Do not touch config, session/state, app-server, protocol schema, TUI, or docs except for exact import references required by assigned files.

SCOUT_EVIDENCE:
Root handoff `.codex/workflow/ROOT_TASK_HANDOFF.md` reports the merge is active. Root grouping found 3 unresolved files under `core/src/tools`, 3 under `core/src/tools/handlers`, and one each under `core/src/tools/code_mode`, `core/src/tools/tool_family`, and `core/src/tasks`. Recent related handoffs: `.codex/workflow/agents/merge_wave10_core_tools_state.handoff.md` and `.codex/workflow/agents/merge_wave11_core_tools_state_exact.handoff.md`.

WHY_AGENT / ROI:
External worker requested by user. Positive ROI because tools/tasks conflicts are cohesive and can be resolved in parallel with config/session/app workers. You are not alone in the codebase; do not revert or overwrite other workers'' edits.

FIRST_READS:
1. `.codex/workflow/ROOT_TASK_HANDOFF.md`
2. `.codex/workflow/agents/merge_wave10_core_tools_state.handoff.md`
3. `.codex/workflow/agents/merge_wave11_core_tools_state_exact.handoff.md`
4. `git diff --name-only --diff-filter=U -- codex-rs/core/src/tools codex-rs/core/src/tasks`
5. Assigned files from that exact list.

TOOL_HINTS:
Use conflict chunks and exact local imports. Preserve branch multi-agent/tool behavior while accepting upstream tool family/plugin/code-mode changes where compatible.

TOKEN_TIP:
Do not chase unrelated tests or run broad searches. If you need a symbol, search the smallest containing directory.

VERIFICATION:
Allowed only:
- `rg -n "^(<<<<<<<|=======|>>>>>>>)" <assigned files>`
- `git diff --check -- <assigned files>`

Forbidden:
- cargo/rustc/just/build scripts/tests/schema generation
- staging, committing, merge/rebase/reset/checkout
- deploy or activation

HANDOFF:
Write `.codex/workflow/agents/merge_wave12_core_tools_tasks.handoff.md` with files changed, conflict marker status, verification commands/exits, and integration notes.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave12_core_tools_tasks.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
