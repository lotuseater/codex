$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: long_session_perf_race_static_review'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Review the latest fix in `codex-rs/tui/src/tui/frame_requester.rs` for the long-running session performance investigation. The fix separates pending deadlines from the in-flight frame deadline to address a delegated blocker: a delayed request could be lost if scheduled after the current target elapsed but before the old pending slot was cleared.

DO_NOT_INSPECT:
Do not inspect unrelated dirty work outside `codex-rs/tui/src/tui/frame_requester.rs`, `docs/long-running-session-performance-investigation.md`, and `docs/long-running-session-performance-verification.md` unless required to understand an import or type. Do not revert or rewrite other agents'' work.

SCOUT_EVIDENCE:
The previous external handoff `.codex/workflow/agents/long_session_perf_final_static_review.handoff.md` reported a blocking pending-deadline race at `frame_requester.rs:128`/`emit_draw`. Root patched that race and added focused state tests. First-moves routing already pointed to the same TUI file and investigation docs.

WHY_AGENT / ROI:
Independent code inspection before build/deploy is valuable because this is concurrency scheduling code. ROI estimate: reuse/new external review cost=3, parallel_gain=2, context_gain=2, repeat_gain=1, loop_followup_gain=2, risk_penalty=1, net=3. This is a static review only.

FIRST_READS:
1. `git diff -- codex-rs/tui/src/tui/frame_requester.rs`
2. `codex-rs/tui/src/tui/frame_requester.rs`
3. `.codex/workflow/agents/long_session_perf_final_static_review.handoff.md`

TOOL_HINTS:
Do not run build, tests, cargo, rustc, schema generation, deploy, or activation commands. Use `git diff`, `rg`, and file reads only. If you need line numbers, use PowerShell no-profile commands or `rg -n`.

TOKEN_TIP:
Keep the review narrow: validate the in-flight/pending deadline state machine, wakeup behavior, drop handling, and the new tests. Stop when you have actionable findings or confidence.

VERIFICATION:
Static/code inspection only. Specifically check for lost wakeups, stale `Notify` permits, delayed request loss during elapsed/emitting windows, requesters-dropped behavior, and whether the tests assert the repaired invariant.

HANDOFF:
Write `.codex/workflow/agents/long_session_perf_race_static_review.handoff.md` with:
- Verdict: proceed or block
- Findings with severity and exact file/line references
- Any residual risk
- Whether build/deploy may proceed
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\long_session_perf_race_static_review.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
