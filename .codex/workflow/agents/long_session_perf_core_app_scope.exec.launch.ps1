$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: long_session_perf_core_app_scope'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Long-running interactive Codex session performance, core/app-server/MCP handoff reconciliation.

DO_NOT_INSPECT: Read-only except one handoff file: `.codex/workflow/agents/long_session_perf_core_app_scope.handoff.md`. Do not edit source code. Do not run cargo/rustc/just/build scripts/test scripts/schema generation/deploy. Do not revert edits made by others.

SCOUT_EVIDENCE: Root found fresh handoff files: `.codex/workflow/agents/long_session_perf_core_review.handoff.md` and `.codex/workflow/agents/long_session_perf_app_mcp_review.handoff.md`. Investigation doc currently treats core/app as inconclusive and may be stale. TUI fix is being handled separately.

WHY_AGENT / ROI: User explicitly requested external non-interactive helpers. This is independent from TUI implementation and can clarify whether root should patch anything beyond TUI before final verification. ROI estimate: cost=3, parallel_gain=3, context_gain=3, repeat_gain=1, loop_followup_gain=2, risk_penalty=2, net=4.

FIRST_READS:
- `.codex/workflow/agents/long_session_perf_core_review.handoff.md`
- `.codex/workflow/agents/long_session_perf_app_mcp_review.handoff.md`
- `docs/long-running-session-performance-investigation.md`
- Targeted files only if needed to validate line references: `codex-rs/core/src/session/mod.rs`, `codex-rs/app-server/src/thread_state.rs`, `codex-rs/app-server/src/outgoing_message.rs`

TASK:
1. Do read-only reconciliation of core/app handoffs against the investigation doc.
2. Decide whether any core/app source change is small, safe, and appropriate in this pass, or whether those findings should remain documented follow-ups. Do not make source edits yourself.
3. Recommend the exact doc update root should make before final verification.
4. Write `.codex/workflow/agents/long_session_perf_core_app_scope.handoff.md` with: high-confidence findings, patch-now vs defer decision, rationale, and final verification suggestions.
5. Do not run tests/build/fmt.

TOOL_HINTS: Use exact reads and `rg` only for cited symbols. Keep this under a tight time budget.

TOKEN_TIP: No broad repo exploration. Stop after producing the handoff.

VERIFICATION: Static read-only validation only.

HANDOFF: Markdown file with concise bullets and file references.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\long_session_perf_core_app_scope.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
