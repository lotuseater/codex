$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave4_core_runtime'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: core runtime merge conflicts
DO_NOT_INSPECT: Do not edit `codex-rs/tui/**`, `codex-rs/app-server*/**`, `codex-rs/cli/**`, manifests, lockfiles, snapshots, tools, generated schema JSON, or docs. Do not run build/tests/deploy/format/generation.
SCOUT_EVIDENCE: Root observed 112 unmerged paths after starting the merge; grouped counts include `core` 28 plus adjacent runtime crates. The current file list is stored in `.codex/workflow/agents/current-unmerged-files.txt`.
WHY_AGENT / ROI: Core runtime conflicts are numerous and independent enough to handle separately while other workers cover UI/protocol/tooling. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=3, loop_followup_gain=2, risk_penalty=1, net=7.
FIRST_READS: Read `.codex/workflow/agents/merge_wave4_common.md`, then filter `.codex/workflow/agents/current-unmerged-files.txt` for `codex-rs/core/`, `codex-rs/thread/`, `codex-rs/protocol/`, `codex-rs/core-test-suites/`, `codex-rs/network-proxy/`, and `codex-rs/exec-server/`. Use `git show :2:<path>` and `git show :3:<path>` to understand current-branch versus upstream behavior.
TOOL_HINTS: Prefer focused file reads and `rg` for symbols. If many files share the same conflict shape, use a small script only to inventory, not to rewrite. Do not use cargo, rustc, build scripts, tests, git add, git checkout, or formatters.
TOKEN_TIP: Prioritize high marker-count files first, then low-risk mechanical marker removal where both sides are clearly additive.
VERIFICATION: Verify no conflict markers remain in your owned paths using `rg "^(<<<<<<<|=======|>>>>>>>)" <owned paths>` and inspect `git diff -- <owned paths>`. Do not run build/tests.
HANDOFF: Resolve conflicts only in owned core/runtime paths, then write `.codex/workflow/agents/merge_wave4_core_runtime.handoff.md` with edited files, behavior choices, deferred issues, and `HANDOFF_STATUS`.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave4_core_runtime.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
