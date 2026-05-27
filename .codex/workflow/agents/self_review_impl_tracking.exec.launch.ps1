$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self_review_impl_tracking'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Self-review current-agent change tracking and activity journal event wiring in Codex Rust core.

DO_NOT_INSPECT: Do not inspect unrelated handoffs or broad unrelated modules. Read only self-review handoffs under .codex/workflow/agents/handoffs/self-review*.md as needed.

SCOUT_EVIDENCE: Root checked first_moves and handoffs. Prior worker notes identify event wiring near tool execution/apply_patch/create file/git commit handling plus optional activity journal preservation.

WHY_AGENT / ROI: Event source wiring is separable from app-server tests and prompt wording. ROI estimate: cost=3, parallel_gain=3, context_gain=2, repeat_gain=2, loop_followup_gain=2, risk_penalty=2, net=4.

FIRST_READS:
- .codex/workflow/agents/handoffs/self-review-sidecar-tracking.md
- .codex/workflow/agents/handoffs/self-review-sidecar-commit-scope.md
- .codex/workflow/agents/handoffs/self-review-split-tracking-commit.md
- codex-rs/core/src/turn_diff_tracker.rs
- Use rg for exact symbols: committed_file_changes_from_apply_patch_delta, apply_patch, create_file, git commit, on_commit, conversation history/activity journal.

OWNERSHIP:
- You may edit core event-wiring files needed to record changed file paths and git commits.
- You may edit codex-rs/core/src/guardian/review_session.rs only to call or minimally expose record APIs if the core worker has not created them yet.
- You may add a tiny focused helper module if necessary.
- Do not edit app-server files, protocol v2 files, or broad config.
- You are not alone in the codebase. Do not revert or overwrite edits made by others; if a nearby change appears, adapt and leave it intact.

TASK:
Wire code-owned tracking for current-agent changes:
1. Record changed file paths for successful apply_patch/create-file/file-edit paths. Use structured patch delta/file-change data where available, not broad git diff.
2. Record git commits by this agent when a successful command is a git commit. Prefer extracting commit hash after successful commit in a narrow, deterministic way.
3. Preserve an activity journal since last review as code-owned artifact data. Keep the journal concise and driven by existing events/actions; do not rely on LLM memory.
4. Ensure tracked file paths and commit ids are consumed/cleared only by the self-review cycle, not by unrelated reviews.

TOOL_HINTS: Use rg for exact symbols. If repeated command parsing is needed, prototype with a tiny focused Rust helper/unit test rather than broad builds.

VERIFICATION:
- Run cargo fmt if you edited Rust.
- Run focused tests you add/change. Avoid broad debug builds.

HANDOFF:
Write .codex/workflow/agents/handoffs/self-review-impl-tracking.md with:
- event source files changed
- exactly what events are recorded
- how git commits are detected
- activity journal behavior
- tests run and results
- integration notes for root
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self_review_impl_tracking.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
