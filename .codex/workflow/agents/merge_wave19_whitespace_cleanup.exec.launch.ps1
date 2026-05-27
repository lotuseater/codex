$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave19_whitespace_cleanup'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
You are an external non-interactive Codex worker in the `open_ai/codex` repo. Root is the overseer. The merge from upstream/main has just passed raw conflict resolution, but `git diff --check` still reports issues. Your job is minimal cleanup only.

DO_NOT_INSPECT:
Do not inspect `target/`, dependency caches, old session JSONL, or unrelated repo-wide history. Do not run any build, test, cargo, npm, or deploy command. Do not edit generated worker launch/log files unless root explicitly asks later.

SCOUT_EVIDENCE:
Root observed wave17 config/session handoff at 09:32 and no true `UU` conflicts afterward. `git diff --check` reported issues in several Rust files and generated `.codex/workflow/agents/*.exec.launch.ps1` scripts. The launch scripts should be left alone for now.

WHY_AGENT / ROI:
Independent cleanup is useful while root stays coordinator. new_agent_cost=3, parallel_gain=3, context_gain=2, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=6.

FIRST_READS:
Read these first:
- `.codex/workflow/agents/merge_wave17_core_config_session.handoff.md`
- `.codex/workflow/agents/merge_wave13_core_config_session.handoff.md`
- `.codex/workflow/agents/merge_wave12_core_session_state.handoff.md`
Then run `git diff --check` and a focused conflict-marker search over tracked changed files.

TOOL_HINTS:
You are not alone in the codebase; do not revert others'' work. Apply only mechanical whitespace cleanup and leftover conflict-marker fixes in Rust source files that are already merge-touched. If a marker needs semantic judgement, do not guess; leave it and report it. Leave `.codex/workflow/agents/*.exec.launch.ps1` alone.

TOKEN_TIP:
Keep the edit surface tiny. Prefer one or two exact file edits over broad formatting.

VERIFICATION:
After edits, run only `git diff --check` and focused conflict-marker search. Do not build/test. Stage files you changed only if they are already staged/part of the merge and staging is needed to preserve resolved conflict state.

HANDOFF:
Write `.codex/workflow/agents/merge_wave19_whitespace_cleanup.handoff.md` with files changed, what issues remain from `git diff --check`, conflict-marker status, and exact commands run. Keep it concise.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave19_whitespace_cleanup.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
