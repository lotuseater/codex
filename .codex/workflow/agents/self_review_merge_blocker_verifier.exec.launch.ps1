$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self_review_merge_blocker_verifier'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Self-review feature improvements in the local Codex Rust repo after reboot. The root agent is only overseeing. The repo is currently in an active merge on branch `slow-context-budget-mode` with `MERGE_HEAD` present and many unmerged entries, including self-review/core/app-server surfaces.

DO_NOT_INSPECT: Do not do broad repo sweeps. Do not inspect unrelated features. Do not attempt to resolve merge conflicts. Do not edit any code file. Do not run broad build/test commands or repair build failures. Do not delegate further.

SCOUT_EVIDENCE: Root preflight found `MERGE_HEAD` present, 21 conflicted entries, and unmerged self-review-related files. Latest `.codex/workflow/agents/handoffs/self-review-post-final-audit.md` reports `Status: blocked-by-merge` and estimates 15-30 minutes after merge resolution for targeted tests/audit.

WHY_AGENT / ROI: User requested external noninteractive delegation while root remains overseer. Net positive only for a bounded read-only verifier because implementation files are merge-conflicted. Confirm the blocker and preserve next actions without spending root context.

FIRST_READS: Read these exact files first and stop if they are enough: `.codex/workflow/agents/handoffs/self-review-root-overseer.md`, `.codex/workflow/agents/handoffs/self-review-post-final-audit.md`, `git status --porcelain=v1 --untracked-files=no`, `git diff --name-only --diff-filter=U --`. If you need one source file, inspect only unmerged/self-review paths from those commands, but do not edit them.

TOOL_HINTS: Use `rg`/focused git commands only. You may write one handoff file at `.codex/workflow/agents/handoffs/self-review-merge-blocker-verifier.md`. You may create no other files. Keep commands read-only except writing that handoff.

TOKEN_TIP: Keep this to a short verifier pass. Avoid re-reading large diffs. Do not explain known feature goals at length.

VERIFICATION: Determine whether any merge-independent implementation or test action is still safe now. If safe, name exact file/action and why it avoids conflicts. If not safe, say blocked and list the exact unmerged paths that block final implementation/verification.

HANDOFF: Write `.codex/workflow/agents/handoffs/self-review-merge-blocker-verifier.md` with: status, blocker evidence, safe/unsafe next action, percent done estimate, time to finish after merge, and exact commands future self should run after merge resolution. Final response should only say that the handoff was written.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self_review_merge_blocker_verifier.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
