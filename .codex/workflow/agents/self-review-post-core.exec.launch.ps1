$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-post-core'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'You are an external non-interactive Codex worker for the self-review feature after a reboot.

CONTEXT_AREA:
- Finish/audit core self-review semantics only.
- Owned write scope: `codex-rs/self-review/**`, `codex-rs/core/src/tasks/review.rs`, and directly adjacent tests only if they already belong to this feature.
- You are not alone in the codebase. Do not revert or overwrite changes by other sessions. Work with the current tree.

DO_NOT_INSPECT:
- Do not repair the in-progress merge from `main`.
- Do not fix broad build failures, dependency issues, or unrelated conflicted files.
- Do not edit app-server/protocol surface files unless the feature is impossible without it; if so, hand off instead.
- Do not spawn more agents.

SCOUT_EVIDENCE:
- Root handoff: `.codex/workflow/agents/handoffs/self-review-root-overseer.md`.
- Prior stale worker artifacts: `.codex/workflow/agents/handoffs/self-review-ext-surface.md`, `.codex/workflow/agents/handoffs/self-review-ext-tests.md`, `.codex/workflow/agents/handoffs/self-review-ext-auditor.md`.
- Prior core log: `.codex/workflow/agents/self-review-ext-core.exec.visible.log`; read only tail/search snippets if useful, not the whole log.
- Current root recovery says no active self-review worker remains and `.codex/workflow/agents/handoffs/self-review-ext-core.md` does not exist.

WHY_AGENT / ROI:
- Root must stay an overseer. This core slice is parallelizable and high-context; external worker ROI is positive.

FIRST_READS:
- `codex-rs/self-review/src/lib.rs`
- `codex-rs/self-review/src/git_evidence.rs`
- `codex-rs/self-review/Cargo.toml`
- `codex-rs/core/src/tasks/review.rs`
- `codex-rs/core/tests/session_review.rs`

TASK:
1. Inspect current feature state and determine whether the requested core behavior is already implemented:
   - Track current-agent changed file paths and commits in code, not by LLM memory.
   - Review only agent-specific files/commits since the last review checkpoint.
   - Forget those tracked paths/commits after the review checkpoint.
   - Review is suggestive, inserted/sent as a user prompt rather than a disruptive internal behavior.
   - Before review, insert/send a prompt asking the model to: sum up recent changes, write next plans, and list next actions.
   - Store that answer in system-owned state, then after review/actions reinsert a reminder prompt: `Please resume your before-review tasks. Here is the reminder about them: <...>`.
   - After review findings, ensure a fix/action prompt is inserted when Codex did not automatically fix findings.
   - Expand self-review to reflect on own actions, strategy, long-term quality, delegation/parallelization, automation/scripting, prototyping before broad tests, SOLID/architecture/decoupling/complexity, and planning structure.
   - Ensure code-preserved artifacts include initial user prompts, initial accepted plan, and activity journal since last review.
2. If a narrow core patch is needed and files are not conflicted, implement it within owned scope.
3. If merge conflicts or build breakage block you, stop at a logical point and document exactly what is blocked.

TOOL_HINTS:
- Use focused `rg` and exact file reads.
- Prefer small scripts only for repeated inspection; no broad builds.
- Avoid `cargo build`, `cargo test`, rustc, schema generation, deploy/activation, or merge repair.

TOKEN_TIP:
- Keep exploration tight. Do not reread the entire old visible log.

VERIFICATION:
- Run only cheap targeted checks that do not invoke the broad Rust build if available. If you cannot verify due merge state, say so.

HANDOFF:
- Write `.codex/workflow/agents/handoffs/self-review-post-core.md`.
- Include: files inspected, files changed, behavior status, verification done/not done, blockers, percent done estimate, next recommended action.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-post-core.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
