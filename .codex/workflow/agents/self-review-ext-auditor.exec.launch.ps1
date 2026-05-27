$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-ext-auditor'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Handoff-only audit of Codex Rust self-review feature progress.
DO_NOT_INSPECT: Do not inspect unrelated repo areas, do not edit files, do not resolve merge conflicts, do not commit. Do not modify codex-rs/app-server/src/request_processors/turn_processor.rs.
SCOUT_EVIDENCE: Three external editing workers have been running for about 20 minutes with no self-review-ext handoffs yet. Root must stay an overseer. Current targeted status from root shows only M codex-rs/app-server-protocol/src/protocol/v2/item.rs and M codex-rs/app-server/src/bespoke_event_handling.rs among checked feature paths; turn_processor.rs is unmerged in the broader merge.
WHY_AGENT / ROI: User asked root to delegate all work to external noninteractive sessions. This auditor is read-only and produces a compact state handoff without overlapping worker edits. ROI: parallel_gain=2, context_gain=3, repeat_gain=3, loop_followup_gain=3, cost=3, risk=0, net=8.
FIRST_READS: Read .codex/workflow/agents/handoffs/self-review-root-overseer.md, then list .codex/workflow/agents/handoffs/self-review*.md and read the most recent relevant ones. Inspect only these files if needed: codex-rs/core/src/session/review.rs, codex-rs/core/src/review_prompts.rs, codex-rs/core/src/state/turn.rs, codex-rs/app-server-protocol/src/protocol/v2/review.rs, codex-rs/app-server-protocol/src/protocol/v2/item.rs, codex-rs/app-server/src/bespoke_event_handling.rs, codex-rs/core/tests/suite/review*.rs, codex-rs/app-server/tests/suite/v2/review.rs.
TASK: In read-only mode, answer: what is already implemented, what appears missing for the requested self-review improvements, what files are likely needed, what is blocked by the in-progress merge, and what the next editing worker should do. Do not make code changes. Time budget 8 minutes. Write the handoff even if incomplete.
TOOL_HINTS: Use rg and targeted reads only. No cargo unless it is a fast read-only metadata/status command; do not run build/test.
TOKEN_TIP: Compact summary only; no large quotes or diffs.
VERIFICATION: Read-only audit, no tests.
HANDOFF: Write .codex/workflow/agents/handoffs/self-review-ext-auditor.md with findings, blockers, percent estimate, ETA, and recommended next worker prompt.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-ext-auditor.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
