$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave4_tui'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: TUI and TUI-render merge conflicts
DO_NOT_INSPECT: Do not edit `codex-rs/core/**`, `codex-rs/app-server*/**`, `codex-rs/cli/**`, manifests, lockfiles, generated schema JSON, or tools. Do not run build/tests/deploy/format/generation.
SCOUT_EVIDENCE: Root observed 112 unmerged paths after starting the merge; grouped counts include `tui` 17 and `tui-render` 6. The current file list is stored in `.codex/workflow/agents/current-unmerged-files.txt`.
WHY_AGENT / ROI: UI conflicts are a distinct ownership area and likely involve repeated styling/rendering patterns that benefit from a focused pass. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=2, risk_penalty=1, net=6.
FIRST_READS: Read `.codex/workflow/agents/merge_wave4_common.md`, then filter `.codex/workflow/agents/current-unmerged-files.txt` for `codex-rs/tui/`, `codex-rs/tui-render/`, and root `tui/` if present. Also read relevant `merge_stage1_tui_config_worker.handoff.md` if present.
TOOL_HINTS: Use focused `rg` for renamed widgets/config/types and inspect snapshots only if they are in your owned path list. Do not run insta/cargo/tests or update snapshots mechanically.
TOKEN_TIP: Keep UI behavior choices explicit in the handoff; if a snapshot/manifests update is implied but outside scope, defer it.
VERIFICATION: Verify no conflict markers remain in owned paths using `rg "^(<<<<<<<|=======|>>>>>>>)" <owned paths>` and inspect diffs. Do not run build/tests.
HANDOFF: Resolve conflicts only in owned TUI/TUI-render paths, then write `.codex/workflow/agents/merge_wave4_tui.handoff.md` with edited files, UI behavior choices, deferred snapshot work, and `HANDOFF_STATUS`.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave4_tui.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
