$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_stage1_core_contracts_worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Pre-merge static review of uncommitted refactors in core API/protocol/agent-tool contracts before upstream/main is merged.

DO_NOT_INSPECT: Do not run builds, tests, cargo check, cargo test, or formatters. Do not inspect unrelated UI files except if a diff dependency requires it. Do not modify .codex/workflow/agents except your handoff file.

SCOUT_EVIDENCE: Root ran first_moves_predict for the upstream/main merge plan and existing wave3 worker handoffs are present. Relevant prior handoffs include solid_refactor_wave3_core_api_boundary_worker.handoff.md, solid_refactor_wave3_agent_boundary_worker.handoff.md, solid_refactor_wave3_tools_boundary_worker.handoff.md, and solid_refactor_wave3_protocol_domain_tests_worker.handoff.md.

WHY_AGENT / ROI: Independent static review can run while root coordinates other areas. new_agent_cost=3, parallel_gain=3, context_gain=2, repeat_gain=1, loop_followup_gain=2, risk_penalty=1, net=4. User requested external non-interactive top-model/high-effort workers.

FIRST_READS: Start with `git status --short --branch --untracked-files=no`, then `git diff -- codex-rs/core-api codex-rs/codex-mcp codex-rs/agent-policy app-server-protocol/src/protocol`. Read the four handoff files named above before broad search. Then inspect only touched files in those areas, especially codex-rs/core-api/src/lib.rs, codex-rs/core-api/src/attestation.rs, codex-rs/codex-mcp/src/mcp_connection_manager.rs, codex-rs/agent-policy/src/lib.rs, app-server-protocol/src/protocol/common.rs, and app-server-protocol/src/protocol/v2.rs.

TOOL_HINTS: Use `git diff --check` only if you can keep it strictly textual/static; no build/test commands. If you need repeated file analysis, write a tiny one-off script under .codex/workflow/agents and delete it before handoff, or avoid it.

TOKEN_TIP: Keep the review bounded to changed lines and nearby signatures. Do not summarize the whole repo.

VERIFICATION: Static review only. Identify likely compile conflicts, API drift, broken imports, duplicate types, stale re-exports, or merge-sensitive duplicated code. If you find a critical small fix in your owned scope, patch it and record the exact file/line. Otherwise do not edit.

HANDOFF: Write .codex/workflow/agents/merge_stage1_core_contracts_worker.handoff.md with sections: Scope, Files inspected, Changes made, Findings/blockers, Merge-risk notes, Recommended staging. Keep it concise and explicit whether you edited files.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_stage1_core_contracts_worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
