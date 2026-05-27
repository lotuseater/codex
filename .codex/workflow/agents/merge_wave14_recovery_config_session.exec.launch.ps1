$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave14_recovery_config_session'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: upstream/main merge recovery, config/session slice only. Repo is mid-merge on `slow-context-budget-mode` with `MERGE_HEAD` present. Active broad wave13 workers are still running, so this is an ADVISORY/HANDOFF-ONLY recovery task.

DO_NOT_INSPECT: Do not run broad searches or builds/tests. Do not inspect unrelated app-server, TUI, frontend, docs, or previous solid_refactor wave files unless directly referenced by the files below. Do not edit or stage git-tracked source files. Do not kill or interact with other worker processes.

SCOUT_EVIDENCE: Root verified 21 unresolved core paths after multiple 5-minute checks; no wave13 core handoffs exist. Existing broad workers are alive but have not reduced conflicts for ~80+ minutes.

WHY_AGENT / ROI: Smaller external non-interactive advisory scope should be faster and avoid racing active broad workers. You are not alone in this repo; other workers may be reading/writing. Avoid reverting or overwriting any external edits.

FIRST_READS: Read only these unresolved files plus their conflict stages as needed using `git diff --cc`, `git show :1:path`, `git show :2:path`, `git show :3:path`:
- `codex-rs/core/src/config/config_tests.rs`
- `codex-rs/core/src/config/edit.rs`
- `codex-rs/core/src/config/mod.rs`
- `codex-rs/core/src/hook_runtime.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/session.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/state/session.rs`

TOOL_HINTS: Use `git diff --cc -- <path>` and targeted `git show :2:<path>` / `git show :3:<path>` comparisons. If you need exact line summaries, use `rg -n ''<<<<<<<|=======|>>>>>>>''` only on the listed files.

TOKEN_TIP: Stop as soon as you can give root an actionable resolution map. Do not produce long source dumps. Do not solve by running build/tests.

VERIFICATION: No builds/tests. Verify only by checking that your proposed resolution map covers all conflict markers in your slice and notes any ambiguous semantic decisions.

HANDOFF: Write `.codex/workflow/agents/merge_wave14_recovery_config_session.handoff.md` with: (1) files inspected, (2) recommended resolution per file, (3) any exact snippets or stage choices needed, (4) risks/ambiguous choices, (5) whether root can apply directly. Do not edit any git-tracked source file.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave14_recovery_config_session.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
