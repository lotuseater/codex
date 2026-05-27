$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave4_config_cli_tools'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: configuration, CLI, tools, manifests, docs, samples, and generated metadata conflicts
DO_NOT_INSPECT: Do not edit core runtime, app-server, app-server-protocol, or TUI implementation files unless they are necessary context and you do not change them. Do not run build/tests/deploy/format/generation.
SCOUT_EVIDENCE: Root observed 112 unmerged paths after starting the merge; grouped counts include `config` 8, `cli` 4, `tools` 4, `ext` 5, root files, manifests, docs, skills, snapshots, and schema JSON. The current file list is stored in `.codex/workflow/agents/current-unmerged-files.txt`.
WHY_AGENT / ROI: Config/CLI/tooling conflicts include metadata and generated files that are easy to corrupt if mixed with runtime edits, so they should be isolated. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=3, loop_followup_gain=2, risk_penalty=1, net=7.
FIRST_READS: Read `.codex/workflow/agents/merge_wave4_common.md`, then filter `.codex/workflow/agents/current-unmerged-files.txt` for `codex-rs/config/`, `codex-rs/cli/`, `codex-rs/tools/`, `codex-rs/ext/`, `codex-rs/features/`, `codex-rs/codex-cli/`, `codex-rs/cloud-requirements/`, `codex-rs/windows-sandbox-rs/`, `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`, `.cargo/`, `.codex/`, `docs/`, `scripts/`, `*.md`, `*.json`, `*.snap`, and root files. Also read existing stage1 handoffs if they mention config/manifests/tests.
TOOL_HINTS: For JSON/TOML, prefer syntactic preservation and additive union when both sides add entries. Do not regenerate schema/snapshots; resolve text if obvious and defer regeneration in handoff. Do not run cargo, rustc, build scripts, tests, git add, checkout, or formatters.
TOKEN_TIP: Separate true source manifests from generated outputs in your handoff so root can decide final regeneration after merge.
VERIFICATION: Verify no conflict markers remain in owned paths using `rg "^(<<<<<<<|=======|>>>>>>>)" <owned paths>` and inspect diffs. Do not run build/tests.
HANDOFF: Resolve conflicts only in owned config/CLI/tools/metadata paths, then write `.codex/workflow/agents/merge_wave4_config_cli_tools.handoff.md` with edited files, union decisions, deferred generated/schema/snapshot work, and `HANDOFF_STATUS`.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave4_config_cli_tools.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
