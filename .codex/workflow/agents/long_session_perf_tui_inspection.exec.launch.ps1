$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: long_session_perf_tui_inspection'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Current dirty TUI scheduler fix for long-session performance degradation:
- `codex-rs/tui/src/tui/frame_requester.rs`
- Prior review handoff: `.codex/workflow/agents/long_session_perf_tui_review.handoff.md`
- Implementation handoff: `.codex/workflow/agents/long_session_perf_tui_impl.handoff.md`

DO_NOT_INSPECT:
- Do not inspect unrelated repo areas.
- Do not run Cargo, Rust, build, test, deploy, schema generation, or formatting commands.
- Do not edit production code. Write only your handoff file.

SCOUT_EVIDENCE:
Root already launched external implementation/scope workers and inspected the current frame scheduler diff. The main risk to review is whether explicit requester accounting, scheduler wakeup, and delayed-deadline shutdown handling are logically correct before root accepts the larger patch.

WHY_AGENT / ROI:
Independent code inspection is valuable because the fix is concurrency-adjacent and touches scheduler lifetime behavior. ROI estimate: new_agent_cost=3, parallel_gain=2, context_gain=3, repeat_gain=1, loop_followup_gain=2, risk_penalty=1, net=4.

FIRST_READS:
1. `codex-rs/tui/src/tui/frame_requester.rs`
2. `.codex/workflow/agents/long_session_perf_tui_review.handoff.md`
3. `.codex/workflow/agents/long_session_perf_tui_impl.handoff.md`
4. `git diff -- codex-rs/tui/src/tui/frame_requester.rs`

TOOL_HINTS:
Use focused file reads and `rg`/`git diff` only. If you need to reason about async timing, write notes in your handoff rather than running tests.

TOKEN_TIP:
Keep the review narrow. Report only blockers, material risks, and whether root should accept the patch as-is or request edits.

VERIFICATION:
Static inspection only. Do not compile or run tests.

HANDOFF:
Write `.codex/workflow/agents/long_session_perf_tui_inspection.handoff.md` with:
- Verdict: accept / accept with edits / reject
- Findings with file/line references
- Any required edits before build/test
- Commands not run
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\long_session_perf_tui_inspection.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
