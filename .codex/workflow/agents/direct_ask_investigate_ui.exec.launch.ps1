$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: direct_ask_investigate_ui'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Codex Rust TUI command surface for implementing /direct_ask_llm. Focus on slash command parsing, inline args, app command dispatch, and how the result should appear in chat.
DO_NOT_INSPECT: Do not inspect backend client internals beyond the app_server_session boundary. Do not edit product code. Do not run cargo/build/test commands.
SCOUT_EVIDENCE: Root already inspected slash_command.rs enum/description/supports_inline_args, app_command.rs AppCommand::UserTurn, app/thread_routing.rs AppCommand routing, and tui app_server_session.rs turn_start wrapper.
WHY_AGENT / ROI: Parallel UI investigation is independent from backend protocol and implementation. ROI estimate: new_agent_cost=3, parallel_gain=3, context_gain=2, repeat_gain=1, loop_followup_gain=2, risk_penalty=1, net=4.
FIRST_READS: Start with codex-rs/tui/src/slash_command.rs, codex-rs/tui/src/chatwidget.rs, codex-rs/tui/src/bottom_pane/chat_composer.rs, codex-rs/tui/src/app/event_dispatch.rs, codex-rs/tui/src/app/thread_routing.rs, codex-rs/tui/src/app_command.rs, codex-rs/tui/src/app_server_session.rs. If routing is unclear, use first_moves_predict before broad rg.
TOOL_HINTS: Prefer rg for SlashCommand::Review/Rename/Plan/Goal inline args as examples. Write a compact handoff to .codex/workflow/agents/direct_ask_investigate_ui.handoff.md. Do not change code.
TOKEN_TIP: Stay under 2500 words. Include exact command parsing and dispatch symbols.
VERIFICATION: Explain where /direct_ask_llm should be parsed, how inline args are preserved, what to do for missing args/file errors, and how to avoid using normal UserTurn context.
HANDOFF: Provide recommended UI/TUI file changes, exact symbols to add/match, and one or two focused checks.

User requirement: implement command /direct_ask_llm. Input after command is either a filepath or literal prompt text. Try file first; if file read fails, use literal text. It must send only that prompt to LLM without additional context, previous memory, rules, etc. The helper should be decoupled as a separate cargo crate.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\direct_ask_investigate_ui.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
