$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-post-final-audit'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Self-review feature follow-up audit after reboot. Root is overseer only; you are an external noninteractive worker.

DO_NOT_INSPECT: Do not repair or normalize the in-progress merge conflict in `codex-rs/app-server/src/request_processors/turn_processor.rs`. Do not run broad builds, cargo test, cargo fmt, schema generation, deploy, activation, or any expensive repair loop. Do not revert unrelated working-tree changes. Do not recursively delegate.

SCOUT_EVIDENCE: Root recovered the post-reboot state from `.codex/workflow/agents/handoffs/self-review-post-core.md`, `.codex/workflow/agents/handoffs/self-review-post-surface.md`, and git status. The feature files currently changed include self-review protocol/core/app-server/test files, while `turn_processor.rs` remains `UU` because another merge session owns it.

WHY_AGENT / ROI: User explicitly requested external noninteractive delegated sessions. This is a bounded final audit that can run independently while root sleeps. ROI estimate: reuse_cost=1, parallel_gain=1, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=7.

FIRST_READS:
1. `.codex/workflow/agents/handoffs/self-review-post-core.md`
2. `.codex/workflow/agents/handoffs/self-review-post-surface.md`
3. `.codex/workflow/agents/handoffs/self-review-ext-auditor.md`, `.codex/workflow/agents/handoffs/self-review-ext-surface.md`, `.codex/workflow/agents/handoffs/self-review-ext-tests.md` if present
4. Target implementation files only as needed:
   - `codex-rs/self-review/src/lib.rs`
   - `codex-rs/self-review/src/git_evidence.rs`
   - `codex-rs/core/src/tasks/review.rs`
   - `codex-rs/app-server-protocol/src/protocol/v2/review.rs`
   - `codex-rs/app-server-protocol/src/protocol/v2/item.rs`
   - `codex-rs/app-server/src/bespoke_event_handling.rs`
   - `codex-rs/app-server/tests/suite/v2/review.rs`
   - `codex-rs/app-server/src/request_processors/turn_processor.rs` only enough to confirm the merge conflict blocks end-to-end validation; do not edit it.

TASK:
Audit whether the implemented feature satisfies the user requirements:
- review evidence is agent-scoped by tracked changed file paths and agent git commits since last review, then cleared after review;
- review is suggestive, inserted like a user prompt;
- pre-review summary/plan/actions prompt is inserted and its answer is stored;
- reminder prompt is reinserted after review/actions;
- follow-up fix-review prompt is inserted if findings are not acted on automatically;
- extended self-review asks about own actions, strategy, long-term quality, delegation/parallelization, automation/scripting, prototyping, architecture/SOLID/decoupling/complexity, and planning/structure;
- code-preserved artifacts include initial user prompts, initial accepted plan, and an activity journal since last review.

BOUNDARY:
This is primarily read-only. If you find a tiny issue outside the merge-conflicted file that is obviously fixable without build/test repair, you may patch it, but prefer reporting findings. Do not touch `turn_processor.rs` except read-only inspection.

VERIFICATION:
Do not attempt full build due active merge. You may run cheap read-only checks such as `git diff --check -- <target files>` if safe. Report exactly what you did and did not verify.

HANDOFF:
Write `.codex/workflow/agents/handoffs/self-review-post-final-audit.md` with:
- status: pass / findings / blocked-by-merge;
- concise evidence summary;
- any actionable findings with file paths;
- whether a follow-up fixer worker is needed;
- percent done estimate and time-to-finish estimate.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-post-final-audit.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
