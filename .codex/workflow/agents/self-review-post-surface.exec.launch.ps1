$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-post-surface'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'You are an external non-interactive Codex worker for the self-review feature after a reboot.

CONTEXT_AREA:
- Finish/audit app-server/protocol/session surface for self-review prompts and artifacts.
- Owned write scope: `codex-rs/app-server-protocol/src/protocol/**/review.rs`, related `item.rs` / `tests.rs`, `codex-rs/app-server/src/bespoke_event_handling.rs`, `codex-rs/app-server/src/request_processors/turn_processor.rs`, and `codex-rs/app-server/tests/suite/v2/review.rs`.
- You are not alone in the codebase. Do not revert or overwrite changes by other sessions. Work with the current tree.

DO_NOT_INSPECT:
- Do not repair the in-progress merge from `main`.
- Do not fix broad build failures, dependency issues, or unrelated conflicted files.
- Do not edit `codex-rs/self-review/**` or `codex-rs/core/src/tasks/review.rs`; if core changes are needed, report them in the handoff.
- Do not spawn more agents.

SCOUT_EVIDENCE:
- Root handoff: `.codex/workflow/agents/handoffs/self-review-root-overseer.md`.
- Prior stale worker artifacts: `.codex/workflow/agents/handoffs/self-review-ext-surface.md`, `.codex/workflow/agents/handoffs/self-review-ext-tests.md`, `.codex/workflow/agents/handoffs/self-review-ext-auditor.md`.
- Current root recovery says old self-review workers are gone and separate merge/build work is out of scope.

WHY_AGENT / ROI:
- Root must stay an overseer. Surface/protocol validation can run in parallel with core semantics.

FIRST_READS:
- `codex-rs/app-server-protocol/src/protocol/v2/review.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/item.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/tests.rs`
- `codex-rs/app-server/src/bespoke_event_handling.rs`
- `codex-rs/app-server/src/request_processors/turn_processor.rs`
- `codex-rs/app-server/tests/suite/v2/review.rs`
- `codex-rs/core/src/tasks/review.rs` read-only for interface context

TASK:
1. Inspect whether app-server/protocol/session surfaces support the requested self-review flow:
   - Suggestive review inserted/sent like a user prompt.
   - Pre-review summary/plan/next-actions prompt and stored answer.
   - Post-review resume reminder prompt carrying the stored answer.
   - Follow-up prompt to act on review findings when automatic repair did not happen.
   - Code-preserved artifacts for initial user prompts, accepted plan, and activity journal where this surface needs to expose them.
2. If narrow surface/test patches are needed and files are not conflicted, implement them within owned scope.
3. If merge conflicts or core ownership blocks you, stop at a logical point and document exactly what is blocked.

TOOL_HINTS:
- Use focused `rg` and exact file reads.
- Avoid broad builds/tests; do not run cargo/rustc/schema generation/deploy.

TOKEN_TIP:
- Keep the handoff concise and actionable.

VERIFICATION:
- Run only cheap targeted checks that do not invoke the broad Rust build if available. If you cannot verify due merge state, say so.

HANDOFF:
- Write `.codex/workflow/agents/handoffs/self-review-post-surface.md`.
- Include: files inspected, files changed, behavior status, verification done/not done, blockers, percent done estimate, next recommended action.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-post-surface.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
