$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: long_session_perf_tui_impl'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Long-running interactive Codex session performance, TUI frame scheduling only.

DO_NOT_INSPECT: Do not inspect broad unrelated repo areas. Do not run cargo/rustc/just/build scripts/test scripts/schema generation/deploy. Do not edit core, app-server, protocol, wrappers, or docs except an optional short handoff file named `.codex/workflow/agents/long_session_perf_tui_impl.handoff.md`. Do not revert edits made by others.

SCOUT_EVIDENCE: Root inspected `docs/long-running-session-performance-investigation.md` and `.codex/workflow/agents/long_session_perf_tui_review.handoff.md`. Existing dirty change in `codex-rs/tui/src/tui/frame_requester.rs` replaces the old unbounded mpsc scheduler with shared pending-deadline state and tests. TUI scout found one concrete regression: scheduler can outlive the last requester while a delayed frame is pending. Scout also noted a lower-confidence delayed re-arm race; treat it as follow-up unless a very small bounded fix is clearly safe.

WHY_AGENT / ROI: User explicitly requested external non-interactive helpers. This is a bounded single-file implementation/test-canary task with high context value and low merge risk. ROI estimate: cost=3, parallel_gain=2, context_gain=3, repeat_gain=1, loop_followup_gain=2, risk_penalty=1, net=4.

FIRST_READS:
- `codex-rs/tui/src/tui/frame_requester.rs`
- `.codex/workflow/agents/long_session_perf_tui_review.handoff.md`
- `docs/long-running-session-performance-investigation.md`

TASK:
1. Own edits only in `codex-rs/tui/src/tui/frame_requester.rs` unless writing your short handoff.
2. Keep the bounded shared-state scheduler approach already present in the dirty file.
3. Fix the concrete shutdown/lifetime regression: after the last `FrameRequester` is dropped, the scheduler must wake and exit promptly even if a delayed frame deadline is pending, and must not emit that delayed draw after shutdown.
4. Add or adjust focused paused-time unit coverage in this same file for the shutdown case. Existing tests already cover immediate, delayed, coalescing, earlier-deadline wakeup, and rate limiting; keep them aligned with intended coalescing semantics unless you make a deliberately justified tiny improvement.
5. Keep comments sparse and implementation local.
6. Do not run tests/build/fmt. You may use `rg`, `git diff`, and read-only file commands. Use `apply_patch` for manual edits.
7. Write `.codex/workflow/agents/long_session_perf_tui_impl.handoff.md` summarizing changed behavior and exact commands not run due instruction.

TOOL_HINTS: Read targeted line chunks. Prefer `apply_patch`. Avoid broad `rg` sweeps.

TOKEN_TIP: Stop when the single-file patch and handoff are complete.

VERIFICATION: Static review only. No build/test/fmt.

HANDOFF: Include changed files, behavioral summary, remaining risk, and suggested final checks for root.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\long_session_perf_tui_impl.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
