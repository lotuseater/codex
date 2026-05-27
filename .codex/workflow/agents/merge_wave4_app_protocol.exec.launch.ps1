$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave4_app_protocol'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: app server and app-server-protocol merge conflicts
DO_NOT_INSPECT: Do not edit `codex-rs/core/**`, `codex-rs/tui/**`, `codex-rs/cli/**`, manifests, lockfiles, snapshots, generated schema JSON, or tools unless explicitly listed below. Do not run build/tests/schema generation/deploy.
SCOUT_EVIDENCE: Root observed 112 unmerged paths after starting the merge; grouped counts include `app-server-protocol` 12 and `app-server` 5. The current file list is stored in `.codex/workflow/agents/current-unmerged-files.txt`.
WHY_AGENT / ROI: This area is separable from TUI/core/tooling and can resolve protocol/server conflicts in parallel. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=2, risk_penalty=1, net=6.
FIRST_READS: Read `.codex/workflow/agents/merge_wave4_common.md`, then filter `.codex/workflow/agents/current-unmerged-files.txt` for `codex-rs/app-server-protocol/` and `codex-rs/app-server/`. Also read `codex-rs/app-server/README.md` if listed or relevant. Use `git show :2:<path>` and `git show :3:<path>` for ambiguous hunks.
TOOL_HINTS: Use `rg` and focused `git diff -- <owned paths>`. A tiny PowerShell loop to list your owned files is fine. Do not use cargo, rustc, build scripts, tests, schema generation, formatters, git add, or git checkout.
TOKEN_TIP: Work hunk by hunk; do not read unrelated modules broadly.
VERIFICATION: Verify by checking no conflict markers remain in owned files using `rg "^(<<<<<<<|=======|>>>>>>>)" <owned paths>` and by rereading changed hunks. Do not run build/tests.
HANDOFF: Resolve conflicts only in owned paths under `codex-rs/app-server-protocol/**` and `codex-rs/app-server/**`, then write `.codex/workflow/agents/merge_wave4_app_protocol.handoff.md` with edited files, decisions, deferred generated/schema work, and `HANDOFF_STATUS`.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave4_app_protocol.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
