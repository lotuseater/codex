$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: long_session_perf_app_protocol_impl'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Bounded long-session performance telemetry improvement: app-server protocol v2 should preserve context-window data that core already reports in `codex_protocol::protocol::TokenUsageInfo`.

Owned write scope:
- `codex-rs/app-server-protocol/src/protocol/v2/mod.rs`
- `codex-rs/app-server/src/outgoing_message.rs`
- `codex-rs/app-server/src/request_processors/thread_lifecycle.rs`
- `codex-rs/app-server/tests/suite/v2/mod.rs`
- Any directly required adjacent app-server test fixture in the same v2 suite

DO_NOT_INSPECT:
- Do not inspect unrelated TUI, MCP, CLI, or browser code.
- Do not edit `codex-rs/tui/src/tui/frame_requester.rs`; another worker owns that change.
- Do not run Cargo, Rust, build, test, deploy, schema generation, or formatting commands.
- You are not alone in the codebase. Do not revert others'' changes; adapt to them.

SCOUT_EVIDENCE:
`.codex/workflow/agents/long_session_perf_core_app_scope.handoff.md` found that core already carries `model_context_window` / `current_context_window`, but the app-server/protocol boundary drops live context-window telemetry. Focus on that handoff and the exact files above.

WHY_AGENT / ROI:
The change crosses a client-facing protocol shape and app-server conversion/tests, so an isolated worker reduces root context load and lets root review/integrate. ROI estimate: new_agent_cost=3, parallel_gain=2, context_gain=3, repeat_gain=2, loop_followup_gain=2, risk_penalty=2, net=4.

FIRST_READS:
1. `.codex/workflow/agents/long_session_perf_core_app_scope.handoff.md`
2. `codex-rs/app-server-protocol/src/protocol/v2/mod.rs`
3. `codex-rs/app-server/src/outgoing_message.rs`
4. `codex-rs/app-server/src/request_processors/thread_lifecycle.rs`
5. `codex-rs/app-server/tests/suite/v2/mod.rs`
6. `rg -n "TokenUsageInfo|model_context_window|current_context_window|ThreadInfo" codex-rs/app-server codex-rs/app-server-protocol codex-rs/protocol/src/models.rs -g ''*.rs''`

TOOL_HINTS:
Use focused reads/searches and `apply_patch` for edits. Prefer existing serde naming/patterns and app-server tests. If protocol v2 usage already has a field, avoid duplication and only wire missing conversions.

TOKEN_TIP:
Keep this to the narrow telemetry boundary. Do not broaden into MCP, memory, compaction, or generic session performance work.

VERIFICATION:
Static verification only. Do not compile, run tests, run rustfmt, or invoke build/deploy commands. Root will run verification after all edits/docs are complete.

HANDOFF:
Write `.codex/workflow/agents/long_session_perf_app_protocol_impl.handoff.md` with:
- Files changed
- Behavior summary
- Any important compatibility/serialization note
- Commands not run
- Suggested final checks
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\long_session_perf_app_protocol_impl.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
