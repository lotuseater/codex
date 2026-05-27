$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave14_recovery_tools_handlers'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: upstream/main merge recovery, core tools/handlers/tasks slice only. Repo is mid-merge on `slow-context-budget-mode` with `MERGE_HEAD` present. Active broad wave13 workers are still running, so this is an ADVISORY/HANDOFF-ONLY recovery task.

DO_NOT_INSPECT: Do not run broad searches or builds/tests. Do not inspect config/session/test-suite files except for direct references. Do not edit or stage git-tracked source files. Do not kill or interact with other worker processes.

SCOUT_EVIDENCE: Root verified 21 unresolved core paths after multiple 5-minute checks; no wave13 core handoffs exist. Existing broad workers are alive but have not reduced conflicts for ~80+ minutes.

WHY_AGENT / ROI: This smaller advisory scope should provide merge decisions faster without racing active broad workers. You are not alone in this repo; do not revert or overwrite others.

FIRST_READS: Read only these unresolved files plus conflict stages as needed using `git diff --cc`, `git show :1:path`, `git show :2:path`, `git show :3:path`:
- `codex-rs/core/src/tasks/mod.rs`
- `codex-rs/core/src/tools/code_mode/execute_handler.rs`
- `codex-rs/core/src/tools/handlers/extension_tools.rs`
- `codex-rs/core/src/tools/handlers/mcp.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/core/src/tools/registry_tests.rs`

TOOL_HINTS: Prefer targeted per-file conflict inspection. Use `rg -n ''<<<<<<<|=======|>>>>>>>''` only on the listed files. If a resolution depends on a nearby type/function, read the nearest same-file context only.

TOKEN_TIP: Produce a compact resolution map, not a transcript. No build/test attempts.

VERIFICATION: No builds/tests. Verify only by ensuring your handoff covers every conflict marker in listed files and identifies exact stage preference or merged content.

HANDOFF: Write `.codex/workflow/agents/merge_wave14_recovery_tools_handlers.handoff.md` with: (1) files inspected, (2) recommended resolution per file, (3) exact snippets/stage choices, (4) risks/ambiguous choices, (5) whether root can apply directly. Do not edit git-tracked source files.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave14_recovery_tools_handlers.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
