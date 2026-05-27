$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave19_diff_sanity'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
You are an external non-interactive Codex worker in the `open_ai/codex` repo. Root is the overseer. The merge from upstream/main has just passed raw conflict resolution: wave17 and wave13 config/session workers wrote handoffs, and root observed no `UU` status after wave17 exited. Your job is a read-only merge/diff sanity pass before final build/deploy.

DO_NOT_INSPECT:
Do not inspect `target/`, dependency caches, old session JSONL, or unrelated repo-wide history. Do not run any build, test, cargo, npm, or deploy command. Do not edit files.

SCOUT_EVIDENCE:
Root has already polled: wave17 config/session handoff exists, wave13 late handoff exists, wave12 session/state handoff exists, and true `UU` conflicts appear cleared. `git diff --check` reported remaining check issues in merge-touched files and worker launch scripts.

WHY_AGENT / ROI:
Independent parallel review is useful while root stays coordinator. new_agent_cost=3, parallel_gain=3, context_gain=2, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=6.

FIRST_READS:
Read these first:
- `.codex/workflow/agents/merge_wave17_core_config_session.handoff.md`
- `.codex/workflow/agents/merge_wave13_core_config_session.handoff.md`
- `.codex/workflow/agents/merge_wave12_core_session_state.handoff.md`
- `.codex/workflow/agents/root_overseer_handoff.md` tail only
Then run read-only checks: `git status --short --branch --untracked-files=no`, `git status --porcelain=v1 -uno`, `git diff --name-only --diff-filter=U`, `git diff --check`, and a focused conflict-marker search over tracked changed files.

TOOL_HINTS:
Use compact PowerShell pipelines or a small read-only script if it saves time. Keep output summarized; do not paste huge diffs. Do not call broad build/test commands.

TOKEN_TIP:
Spend most tokens on findings and exact next root actions. Avoid re-reading old logs unless a handoff explicitly points to one.

VERIFICATION:
No build/tests. Verification is limited to read-only git status, diff-check, and conflict-marker checks.

HANDOFF:
Write `.codex/workflow/agents/merge_wave19_diff_sanity.handoff.md` with: current merge status, remaining blockers if any, exact files with conflict markers or `diff --check` issues, and whether root can proceed to final cleanup/build stage.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave19_diff_sanity.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
