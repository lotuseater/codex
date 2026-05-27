$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-ext-tests'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Codex Rust self-review test/verification and conflict-aware gap review.
DO_NOT_INSPECT: Do not inspect unrelated merge work or broad repo history. Do not resolve merge conflicts. Do not edit codex-rs/app-server/src/request_processors/turn_processor.rs.
SCOUT_EVIDENCE: Existing handoffs in .codex/workflow/agents/handoffs/self-review*.md, plus current targeted status says the merge is in progress and turn_processor.rs is unmerged. Core/surface workers are running separately.
WHY_AGENT / ROI: User requested external noninteractive worker delegation. This worker independently verifies gaps while implementation workers run. ROI: parallel_gain=3, context_gain=2, repeat_gain=3, loop_followup_gain=3, cost=3, risk=1, net=7.
FIRST_READS: Read all .codex/workflow/agents/handoffs/self-review*.md filenames and the most recent relevant handoffs. Then inspect codex-rs/core/tests/suite/review*.rs, codex-rs/app-server/tests/suite/v2/review.rs, and touched review files only as needed.
TASK: Add or adjust focused tests for the improved self-review behavior if safe and non-overlapping: current-agent changed-file and commit scope reset, pre-review summary/plans/actions prompt capture, resume reminder insertion, action-on-findings fallback prompt, expanded self-reflection criteria, and preserved artifacts (initial user prompts, initial accepted plan, activity journal). Prefer tests in codex-rs/core/tests/suite/review*.rs and app-server review tests. If implementation gaps block tests, write failing-test intent or exact coverage gaps in handoff rather than editing conflicted files.
TOOL_HINTS: Use rg and small targeted cargo test commands. Avoid full build. Use apply_patch for test edits.
TOKEN_TIP: Keep output compact; root needs status and blockers, not full diffs.
VERIFICATION: Run the smallest relevant cargo tests. If cargo is blocked by unmerged files, record the exact command and failure reason.
HANDOFF: Write .codex/workflow/agents/handoffs/self-review-ext-tests.md with summary, files touched, tests run/results, blockers, percent estimate, and recommended final verification path.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-ext-tests.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
