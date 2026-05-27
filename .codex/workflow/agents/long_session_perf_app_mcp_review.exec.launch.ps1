$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: long_session_perf_app_mcp_review'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# External worker: app-server/MCP long-session performance review

CONTEXT_AREA: Codex long-running sessions degrade after hours; interactive TUI freezes more than non-interactive exec sessions. Investigate app-server, protocol, and MCP connection surfaces for event stream backlog, unbounded channels, missing coalescing/backpressure, reconnect loops, or retained state growth that can affect long sessions.

DO_NOT_INSPECT: Do not run builds, tests, formatters, cargo, rustc, just, deploy scripts, schema generation, or broad unrelated repo sweeps. Do not edit code. Only write your handoff file.

SCOUT_EVIDENCE: Correct paths are `codex-rs/app-server`, `codex-rs/app-server-protocol`, and `codex-rs/codex-mcp`. Prior broad search accidentally used wrong root paths. TUI frame backlog is already a high-confidence candidate; you are checking adjacent surfaces.

WHY_AGENT / ROI: This is independent from the TUI and core reviews and can run in parallel. new_agent_cost=3, parallel_gain=3, context_gain=2, repeat_gain=1, loop_followup_gain=3, risk_penalty=1, net=5.

FIRST_READS: `codex-rs/app-server`, `codex-rs/app-server-protocol/src/protocol`, `codex-rs/codex-mcp/src/mcp_connection_manager.rs`. Use targeted `rg` for `mpsc::unbounded`, `UnboundedSender`, `broadcast`, `watch`, `event`, `subscribe`, `connection`, `VecDeque`, `retain`, and `pending` within those paths.

TOOL_HINTS: Use focused `rg` and read exact hit files only. No tests.

TOKEN_TIP: Keep this under 20 minutes. Stop once you have concrete findings or confidence.

VERIFICATION: Static review only. Identify whether any issue is likely enough to patch now, or whether it should be documented as follow-up.

HANDOFF: Write `.codex/workflow/agents/long_session_perf_app_mcp_review.handoff.md` with: summary, files inspected, findings with file/line refs if possible, recommended edits, and confidence. Keep it concise.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\long_session_perf_app_mcp_review.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
