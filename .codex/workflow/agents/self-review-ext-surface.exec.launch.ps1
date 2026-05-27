$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-ext-surface'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Codex Rust self-review app-server/protocol surface integration.
DO_NOT_INSPECT: Do not inspect unrelated merge work or broad repo history. Do not resolve merge conflicts. Do not edit codex-rs/app-server/src/request_processors/turn_processor.rs; it is unmerged from the user''s main merge.
SCOUT_EVIDENCE: Existing handoffs in .codex/workflow/agents/handoffs/self-review*.md and targeted status show M codex-rs/app-server-protocol/src/protocol/v2/item.rs, M codex-rs/app-server/src/bespoke_event_handling.rs, and UU turn_processor.rs.
WHY_AGENT / ROI: User explicitly requested external noninteractive delegation. This worker owns surface files separate from core implementation. ROI: parallel_gain=3, context_gain=2, repeat_gain=2, loop_followup_gain=3, cost=3, risk=1, net=6.
FIRST_READS: Read .codex/workflow/agents/handoffs/self-review-gap-event-flow.md, self-review-sidecar-prompt-flow.md, self-review-sidecar-artifacts-journal.md if present. Then read codex-rs/app-server-protocol/src/protocol/v2/review.rs, codex-rs/app-server-protocol/src/protocol/v2/item.rs, codex-rs/app-server/src/bespoke_event_handling.rs, codex-rs/app-server/tests/suite/v2/review.rs. You may read turn_processor.rs only to understand the integration gap; do not edit it.
TASK: Implement or finish the app-server/protocol surface for self-review improvements outside turn_processor.rs: expose/replay review prompts as suggestive user-style inserts, carry review artifacts needed by the client/session, support review findings/action prompts if already routed through owned files, and keep protocol structs/tests aligned with core behavior. If the unmerged turn_processor.rs is required, write a precise deferred patch description in handoff.
TOOL_HINTS: Use rg for exact symbols. Use apply_patch for edits. Keep the diff scoped to owned files. Avoid changing generated lock/schema files unless tests require and it is clearly owned.
TOKEN_TIP: Report only high-signal changes and blockers.
VERIFICATION: Run focused app-server/protocol review tests if feasible; otherwise explain merge blocker.
HANDOFF: Write .codex/workflow/agents/handoffs/self-review-ext-surface.md with summary, files touched, tests run/results, blockers, percent estimate, and exact deferred turn_processor requirements if any.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-ext-surface.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
