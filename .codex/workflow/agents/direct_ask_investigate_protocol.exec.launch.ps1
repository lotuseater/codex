$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: direct_ask_investigate_protocol'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Codex Rust checkout, implementing /direct_ask_llm. Focus on the backend/core/app-server protocol path needed to send a one-shot prompt to the LLM with no previous conversation, no memory, no AGENTS/rules/system/user-instructions context.
DO_NOT_INSPECT: Do not read unrelated frontend styling, docs inventories, or broad repository trees. Do not edit product code. Do not run cargo/build/test commands.
SCOUT_EVIDENCE: Root already inspected tui app_command.rs, slash_command.rs, tui app_server_session.rs, app/thread_routing.rs, and app-server-protocol turn params enough to see normal user turns flow from AppCommand::UserTurn -> app_server.turn_start -> TurnStartParams.
WHY_AGENT / ROI: Parallel backend investigation is independent of UI parsing and implementation. ROI estimate: new_agent_cost=3, parallel_gain=3, context_gain=2, repeat_gain=1, loop_followup_gain=2, risk_penalty=1, net=4.
FIRST_READS: Start with codex-rs/app-server-protocol/src/protocol/v2/turn.rs, codex-rs/app-server/src/request_processors/turn_processor.rs, codex-rs/core/src/session/handlers.rs, codex-rs/core/src/client.rs, codex-rs/core/src/client_common.rs, codex-rs/core/Cargo.toml. If needed, use first_moves_predict before broad rg.
TOOL_HINTS: Prefer rg for exact symbols and small file slices. Write a compact handoff to .codex/workflow/agents/direct_ask_investigate_protocol.handoff.md. Do not change code.
TOKEN_TIP: Stay under 2500 words. Include file paths and specific symbols, not pasted code blocks.
VERIFICATION: Explain the minimal backend/protocol option and risks. Specifically answer whether an existing TurnStart can be made contextless, or whether a separate one-shot core task/client call is required.
HANDOFF: Provide: recommended design, exact files/symbols to touch, fields/APIs to add if any, and a short test/check suggestion that avoids broad builds.

User requirement: implement command /direct_ask_llm. Input after command is either a filepath or literal prompt text. Try to resolve it as file first; if file read fails, use the text directly. It must send only that prompt to the LLM without additional context, previous memory, rules, or conversation history. The implementation should be decoupled, separate cargo crate, no direct product dependencies from the helper crate, SOLID, depend on abstractions.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\direct_ask_investigate_protocol.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
