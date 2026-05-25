$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: direct_ask_impl'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Implement /direct_ask_llm in the Codex Rust checkout. You are an implementation worker.
DO_NOT_INSPECT: Do not inspect unrelated apps/docs/frontend styling. Do not run broad builds/tests, cargo build, cargo test --workspace, or release builds. Do not revert others'' changes. You are not alone in the codebase.
SCOUT_EVIDENCE: Root already inspected normal user-turn flow: tui slash_command.rs -> app_command.rs -> app/thread_routing.rs -> tui app_server_session.rs -> app-server-protocol TurnStartParams. The new feature must avoid normal context/memory/rules.
WHY_AGENT / ROI: Implementation can proceed in parallel with two read-only investigations. ROI estimate: new_agent_cost=3, parallel_gain=2, context_gain=2, repeat_gain=2, loop_followup_gain=2, risk_penalty=2, net=3.
OWNERSHIP: You may edit only files directly needed for this feature, expected candidates: codex-rs/Cargo.toml, codex-rs/Cargo.lock, codex-rs/direct-ask-llm/**, codex-rs/core/Cargo.toml, codex-rs/core/src/tasks/**, codex-rs/core/src/session/handlers.rs or adjacent core client glue if needed, codex-rs/app-server-protocol/src/protocol/**, codex-rs/app-server/src/request_processors/turn_processor.rs or adjacent request processor glue if needed, codex-rs/tui/Cargo.toml, codex-rs/tui/src/slash_command.rs, codex-rs/tui/src/app_command.rs, codex-rs/tui/src/app_server_session.rs, codex-rs/tui/src/app/thread_routing.rs, and small focused tests near changed helper code. If you need another file, note it in the handoff.
FIRST_READS: Read codex-rs/tui/src/slash_command.rs, codex-rs/tui/src/app_command.rs, codex-rs/tui/src/app/thread_routing.rs, codex-rs/tui/src/app_server_session.rs, codex-rs/app-server-protocol/src/protocol/v2/turn.rs, codex-rs/app-server/src/request_processors/turn_processor.rs, codex-rs/core/src/client.rs, codex-rs/core/src/client_common.rs, codex-rs/core/src/session/handlers.rs, codex-rs/core/Cargo.toml, codex-rs/Cargo.toml. Use rg for exact symbols.
TOOL_HINTS: Use apply_patch for edits. For repeated mechanical edits, script only after inspecting. Prefer cargo fmt --check or cargo check -p for the narrow crates only if cheap; otherwise leave exact commands for root. Do not run broad builds/tests.
TOKEN_TIP: Keep handoff short. Avoid dumping code snippets in final handoff.
VERIFICATION: At minimum run targeted unit tests for the new helper crate if you add tests, plus cargo fmt for touched crates if feasible. Do not do workspace-wide builds.
HANDOFF: Write .codex/workflow/agents/direct_ask_impl.handoff.md with summary, files changed, checks run/results, and any unresolved issues.

User requirement: Add command /direct_ask_llm. It sends a prompt to the LLM without any additional context, no previous memory, no rules, no prior chat. It accepts input either as prompt text or as a filepath that contains the prompt. Try to recognize/read it as a file first; if that fails, treat the input as simple text. Make it decoupled: create a separate cargo crate for prompt-source resolution/business logic, with no direct dependencies on Codex core/TUI/app-server. Use abstraction boundaries/SOLID and depend on traits/abstractions where reasonable. Avoid premature broad builds/tests; prefer code inspection and focused editing.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\direct_ask_impl.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
