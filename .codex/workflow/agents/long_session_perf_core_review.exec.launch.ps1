$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: long_session_perf_core_review'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# External worker: core/session long-session performance review

CONTEXT_AREA: Codex long-running sessions degrade after hours even when CPU/memory/system load are low. Non-interactive sessions degrade less; interactive sessions can visually freeze. Investigate core/session surfaces for retained per-turn state, transcript growth, event fan-out, or periodic work that could amplify over long sessions.

DO_NOT_INSPECT: Do not run builds, tests, formatters, cargo, rustc, just, deploy scripts, schema generation, or broad unrelated repo sweeps. Do not edit code. Only write your handoff file.

SCOUT_EVIDENCE: TUI scout already identified frame-request backlog as the primary interactive-freeze candidate. Your job is to find any adjacent core/session contributors or say none found.

WHY_AGENT / ROI: This is independent from the TUI review and can run in parallel while root coordinates. new_agent_cost=3, parallel_gain=3, context_gain=2, repeat_gain=1, loop_followup_gain=3, risk_penalty=1, net=5.

FIRST_READS: Start with `codex-rs/core`, especially session/event/task/turn handling files found by targeted `rg` for `Session`, `Conversation`, `Event`, `mpsc::unbounded`, `UnboundedSender`, `VecDeque`, `history`, `transcript`, `truncate`, and `compaction`. Read exact files from hits only.

TOOL_HINTS: Use focused `rg` and small file reads. Prefer checking existing queues and retained vectors/maps over broad architecture reading. No tests.

TOKEN_TIP: Keep this under 20 minutes. Prioritize concrete mechanisms that grow with session length.

VERIFICATION: Static review only. Identify whether there are obvious unbounded queues or repeated full-history operations on hot paths that could explain degradation. Note if no high-confidence fix should be made now.

HANDOFF: Write `.codex/workflow/agents/long_session_perf_core_review.handoff.md` with: summary, files inspected, findings with file/line refs if possible, recommended edits, and confidence. Keep it concise.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\long_session_perf_core_review.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
