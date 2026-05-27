$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave14_recovery_spec_tests'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: upstream/main merge recovery, spec/tool-family/core test-suite slice only. Repo is mid-merge on `slow-context-budget-mode` with `MERGE_HEAD` present. Active broad wave13 workers are still running, so this is an ADVISORY/HANDOFF-ONLY recovery task.

DO_NOT_INSPECT: Do not run broad searches or builds/tests. Do not inspect config/session/handler files except for direct references. Do not edit or stage git-tracked source files. Do not kill or interact with other worker processes.

SCOUT_EVIDENCE: Root verified 21 unresolved core paths after multiple 5-minute checks; no wave13 core handoffs exist. Existing broad workers are alive but have not reduced conflicts for ~80+ minutes.

WHY_AGENT / ROI: This narrow advisory scope can unblock root without racing current broad workers. You are not alone in this repo; do not revert or overwrite others.

FIRST_READS: Read only these unresolved files plus conflict stages as needed using `git diff --cc`, `git show :1:path`, `git show :2:path`, `git show :3:path`:
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/spec_plan_tests.rs`
- `codex-rs/core/src/tools/tool_family/shell.rs`
- `codex-rs/core/tests/suite/client_websockets.rs`
- `codex-rs/core/tests/suite/code_mode.rs`
- `codex-rs/core/tests/suite/compact_remote.rs`

TOOL_HINTS: Use targeted `git diff --cc -- <path>`. For deleted/modified conflicts (`UD`/`DU`), explicitly identify whether the file should be kept, deleted, or moved and why.

TOKEN_TIP: Keep handoff concise and actionable. No build/test attempts.

VERIFICATION: No builds/tests. Verify only by checking your handoff covers all conflict markers or delete/keep decisions in listed files.

HANDOFF: Write `.codex/workflow/agents/merge_wave14_recovery_spec_tests.handoff.md` with: (1) files inspected, (2) recommended resolution per file, (3) exact snippets/stage choices or delete/keep decisions, (4) risks/ambiguous choices, (5) whether root can apply directly. Do not edit git-tracked source files.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave14_recovery_spec_tests.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
