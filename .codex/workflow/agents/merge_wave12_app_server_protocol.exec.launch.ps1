$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_wave12_app_server_protocol'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Resolve current unmerged conflicts for app-server and app-server-protocol generated schema ownership:
- `codex-rs/app-server/README.md`
- `codex-rs/app-server/src/request_processors.rs`
- `codex-rs/app-server/src/request_processors/config_processor.rs`
- `codex-rs/app-server/src/request_processors/turn_processor.rs`
- `codex-rs/app-server/tests/common/mcp_process.rs`
- `codex-rs/app-server-protocol/schema/typescript/ClientRequest.ts`
- `codex-rs/app-server-protocol/schema/typescript/v2/Config.ts`
- `codex-rs/app-server-protocol/schema/typescript/v2/ConfigRequirements.ts`
- `codex-rs/app-server-protocol/schema/typescript/v2/index.ts`
- `codex-rs/app-server-protocol/schema/typescript/v2/ManagedHooksRequirements.ts`
- `codex-rs/app-server-protocol/schema/typescript/v2/ProfileV2.ts`

DO_NOT_INSPECT:
Do not touch core conflict files, TUI, docs outside the assigned README, or generated schema outside listed files unless `git diff --name-only --diff-filter=U -- codex-rs/app-server codex-rs/app-server-protocol/schema` shows it as unmerged.

SCOUT_EVIDENCE:
Root handoff `.codex/workflow/ROOT_TASK_HANDOFF.md` reports unresolved groups under `app-server`, `app-server/tests`, and `app-server-protocol/schema`. Related prior review context exists in `.codex/workflow/agents/merge_wave5_app_protocol.review.handoff.md` and `.codex/workflow/agents/merge_wave9_protocol_schema.handoff.md`.

WHY_AGENT / ROI:
External worker requested by user. Positive ROI because app-server request processing and protocol schema are separable from core runtime conflicts. You are not alone in the codebase; do not revert or overwrite other workers'' edits.

FIRST_READS:
1. `.codex/workflow/ROOT_TASK_HANDOFF.md`
2. `.codex/workflow/agents/merge_wave5_app_protocol.review.handoff.md`
3. `.codex/workflow/agents/merge_wave9_protocol_schema.handoff.md`
4. `git diff --name-only --diff-filter=U -- codex-rs/app-server codex-rs/app-server-protocol/schema`
5. Assigned files from that exact list.

TOOL_HINTS:
Do not regenerate schema. Resolve generated TypeScript conflicts by preserving the schema shape implied by the corresponding source-side merged behavior and existing generated-file patterns. Use focused diffs and nearby type definitions.

TOKEN_TIP:
Avoid reading all generated schema files. Read only adjacent files needed for import/export consistency.

VERIFICATION:
Allowed only:
- `rg -n "^(<<<<<<<|=======|>>>>>>>)" <assigned files>`
- `git diff --check -- <assigned files>`

Forbidden:
- cargo/rustc/just/build scripts/tests/schema generation
- staging, committing, merge/rebase/reset/checkout
- deploy or activation

HANDOFF:
Write `.codex/workflow/agents/merge_wave12_app_server_protocol.handoff.md` with files changed, marker status, verification commands/exits, and schema/source consistency notes.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_wave12_app_server_protocol.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
