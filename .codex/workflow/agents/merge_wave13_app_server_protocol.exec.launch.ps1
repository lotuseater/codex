$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave13_app_server_protocol'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# merge_wave13_app_server_protocol

CONTEXT_AREA: Resolve the active upstream/main merge conflicts in app-server, app-server tests, and app-server-protocol generated TypeScript schema files.

DO_NOT_INSPECT: Do not inspect unrelated TUI, core tools/tasks, or broad repo areas except direct call/type references needed for assigned files. Do not run broad builds/tests, cargo, schema generation, deploy scripts, or git add/commit/merge/rebase.

SCOUT_EVIDENCE: Root recovered `.codex/workflow/ROOT_TASK_HANDOFF.md` after reboot. Current merge is active on branch `slow-context-budget-mode`; `MERGE_HEAD` is upstream/main `14d80e55cd`. `git ls-files -u` shows assigned app-server/protocol paths still unmerged.

WHY_AGENT / ROI: Independent conflict area; parallel_gain=3, context_gain=2, repeat_gain=2, loop_followup_gain=2, cost=3, risk=1, net=5. Root is only overseeing; this worker should produce edits and a compact handoff.

FIRST_READS:
- `.codex/workflow/ROOT_TASK_HANDOFF.md`
- `.codex/workflow/agents/root_overseer_handoff.md`
- `.codex/workflow/agents/merge_wave9_protocol_schema.handoff.md` if present
- Assigned files:
  - `codex-rs/app-server-protocol/schema/typescript/ClientRequest.ts`
  - `codex-rs/app-server-protocol/schema/typescript/v2/Config.ts`
  - `codex-rs/app-server-protocol/schema/typescript/v2/ConfigRequirements.ts`
  - `codex-rs/app-server-protocol/schema/typescript/v2/index.ts`
  - `codex-rs/app-server-protocol/schema/typescript/v2/ManagedHooksRequirements.ts`
  - `codex-rs/app-server-protocol/schema/typescript/v2/ProfileV2.ts`
  - `codex-rs/app-server/README.md`
  - `codex-rs/app-server/src/request_processors.rs`
  - `codex-rs/app-server/src/request_processors/config_processor.rs`
  - `codex-rs/app-server/src/request_processors/turn_processor.rs`
  - `codex-rs/app-server/tests/common/mcp_process.rs`

TASK:
1. Inspect assigned paths for conflict markers and unmerged state.
2. Resolve conflicts in the working tree only, preserving both upstream/main behavior and current branch slow-context-budget/config/managed-hooks intent.
3. Do not stage files. Root will stage after review.
4. Keep edits narrowly scoped to assigned files and needed adjacent direct references.
5. If generated TypeScript schemas look like generated output, reconcile manually but do not regenerate.

TOOL_HINTS: Use `rg -n "^(<<<<<<<|=======|>>>>>>>)" <assigned paths>` and focused `git diff -- <assigned paths>`. Use small scripts only for mechanical marker inventory if it saves time.

TOKEN_TIP: Read direct assigned files and prior handoff first; avoid repo-wide search unless a symbol cannot be understood locally.

VERIFICATION: No build/test/schema generation. Confirm no conflict markers remain in assigned files and summarize `git diff -- <assigned paths>` at a high level.

HANDOFF: Write `.codex/workflow/agents/merge_wave13_app_server_protocol.handoff.md` with files changed, remaining concerns, and exact root follow-up commands if any. Keep it short.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave13_app_server_protocol.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
