$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: wave6_large_diff_static_review'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# External Worker: wave6_large_diff_static_review

You are running as an external non-interactive Codex worker. The root session is only overseeing. Do not spawn subagents.

CONTEXT_AREA:
- Repo: C:/Users/Oleh/Documents/GitHub/open_ai/codex
- Branch: slow-context-budget-mode
- Task: static review the large dirty tree for integration blockers before final build/deploy.
- The root will have a separate worker edit codex-rs/tui/src/tui/frame_requester.rs. You are read-only except for your handoff.

OWNERSHIP:
- Read-only review. Write only .codex/workflow/agents/wave6_large_diff_static_review.handoff.md
- You are not alone in the codebase. Do not revert or modify source files.

DO_NOT_INSPECT:
- Do not do a broad source sweep. Start from git diff name/status/stat and prior handoffs/docs. Avoid generated/vendor/cache directories.
- Do not inspect frame_requester.rs deeply unless needed to understand whether the previous blocker is still present; a separate worker owns that fix.

SCOUT_EVIDENCE:
- Root saw 102 changed files, ~34284 insertions and 3143 deletions, and prior handoffs under .codex/workflow/agents.

WHY_AGENT / ROI:
- Positive ROI: static review of change scope can run independently while fix worker handles the blocker.

FIRST_READS:
1. git diff --name-status
2. git diff --stat
3. docs/long-running-session-performance-investigation.md
4. docs/long-running-session-performance-verification.md
5. .codex/workflow/agents/merge_stage1_core_contracts_worker.handoff.md
6. .codex/workflow/agents/merge_stage1_tests_manifests_worker.handoff.md
7. app-server-protocol/src/protocol/common.rs and app-server-protocol/src/protocol/v2.rs if git diff shows them changed
8. codex-rs/core/config.schema.json if git diff shows it changed

TOOL_HINTS:
- Use focused `git diff -- <path>` on only files that look like public contracts, config/schema, build/deploy scripts, or docs required by the task.
- Do not run cargo build, cargo test, rustc, npm, deploy scripts, schema generation, or broad test/build commands.

REVIEW TARGET:
- Identify blockers to commit/build/deploy: broken protocol/config contracts, missing docs for investigation, suspicious untracked generated files, accidental unrelated files, or obvious compile failures by inspection.
- Separate blocking findings from non-blocking risks.

VERIFICATION:
- Code inspection only in this worker.
- Include exact commands run. Note that no builds/tests were run.

HANDOFF:
- Write .codex/workflow/agents/wave6_large_diff_static_review.handoff.md with: verdict (deploy-blocking or not), findings with file paths/line refs when possible, commit-scope recommendations, and final suggested next action for root.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\wave6_large_diff_static_review.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
