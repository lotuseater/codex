$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: long_session_perf_tui_review'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# External worker: TUI long-session performance review

CONTEXT_AREA: Codex Rust TUI long-running interactive-session performance degradation. The root overseer has found current dirty edits in `codex-rs/tui/src/tui/frame_requester.rs` that appear to replace an unbounded frame-request channel with coalesced scheduling state. Treat that file as incoming work from another session; do not revert it.

DO_NOT_INSPECT: Do not run builds, tests, formatters, cargo, rustc, just, deploy scripts, schema generation, or broad unrelated repo sweeps. Do not edit code. Only write your handoff file.

SCOUT_EVIDENCE: Prior scout found old `FrameRequester` used `mpsc::UnboundedSender<Instant>`, `schedule_frame()` sent every request, scheduler coalesced only after dequeueing, and rate limiting applied only to emitted frames. This matches visual freezes with low CPU/memory because a burst can backlog the scheduler.

WHY_AGENT / ROI: Independent review of the TUI patch is high value while root coordinates docs and other workers. new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=1, loop_followup_gain=3, risk_penalty=1, net=6.

FIRST_READS: `codex-rs/tui/src/tui/frame_requester.rs`, then targeted `rg` for `schedule_frame(`, `schedule_frame_in(`, `FrameRequester`, and draw-loop usage under `codex-rs/tui/src`. Read only exact files needed from those hits.

TOOL_HINTS: Use `git diff -- codex-rs/tui/src/tui/frame_requester.rs` and focused `rg`. If you need to inspect behavior, reason statically only. No tests.

TOKEN_TIP: Keep this under 20 minutes. Stop once you have concrete findings or confidence.

VERIFICATION: Static review only. Check coalescing correctness, timer reset behavior, shutdown/drop behavior, mutex/notify race windows, delayed-deadline semantics, and whether the test/canary coverage proves the intended behavior.

HANDOFF: Write `.codex/workflow/agents/long_session_perf_tui_review.handoff.md` with: summary, files inspected, findings with file/line refs if possible, recommended edits, and confidence. Keep it concise.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\long_session_perf_tui_review.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
