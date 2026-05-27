$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: stage3_dry_merge_matrix'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Stage 3 post-refactor dry-merge matrix for branch slow-context-budget-mode in C:\Users\Oleh\Documents\GitHub\open_ai\codex. HEAD is 14a9f24005, upstream/main is 9f42c89c01, merge-base is cfa16fcc2e24ba01816fb53e8cfb581f4019e42e. Root fetched upstream/main and four stage-2 workers finished. Current dirty pre-merge conflict-reduction files are codex-rs/Cargo.lock, codex-rs/codex-mcp/Cargo.toml, codex-rs/tui/Cargo.toml, codex-rs/collaboration-mode-templates/templates/plan.md, codex-rs/config/src/config_toml.rs, codex-rs/config/src/profile_toml.rs, codex-rs/tui/src/tui/frame_requester.rs. Ignore MergePrompt.txt and RefactorGoOnPrompt.txt.

DO_NOT_INSPECT: Do not inspect unrelated repo areas. Do not run cargo, rustc, just, npm, build scripts, test scripts, schema generation, or deploy/activation commands. Do not mutate the root checkout except writing your handoff under C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage3-20260525-181500\stage3_dry_merge_matrix\.handoff\.md. Do not spawn agents.

SCOUT_EVIDENCE: Root ran first_moves_predict for this merge task and reviewed stage-2 handoffs. Relevant scopes are manifests/lockfile, config_toml/profile_toml, collaboration plan template, and tui frame_requester.

WHY_AGENT / ROI: Independent dry-merge verification after pre-refactors has high parallel value. Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=3, loop_followup_gain=2, risk_penalty=1, net=7.

FIRST_READS: Read git status --short -uno, git diff --name-only, and the changed files listed above. Then create your own temp worktree under C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage3-20260525-181500\stage3_dry_merge_matrix\worktree.

TASK: Build a fresh dry-merge matrix that includes the current uncommitted pre-refactor patch. Suggested method: create a temp worktree from HEAD, export a patch from the root checkout limited to the seven codex-rs files above, apply it inside the temp worktree, make a temporary local commit there only, then run git merge --no-commit --no-ff upstream/main. Report exactly whether the merge is clean or which files conflict. If conflicts occur, capture conflict file list, which side each conflicting hunk appears to come from, and the least risky resolution direction. Abort the merge only after collecting evidence, or leave the temp worktree with a clear status note.

TOOL_HINTS: Prefer git commands and targeted reads. Use small helper scripts only if they save repeated manual parsing. No broad rg sweeps unless a conflict file references an unknown symbol.

TOKEN_TIP: Keep output short. Do not paste large diffs. Summarize hunks by file/function/field.

VERIFICATION: Verification is dry-merge only. No builds/tests before the real merge.

HANDOFF: Write C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage3-20260525-181500\stage3_dry_merge_matrix\.handoff\.md with: result, conflict files, command outline, temp worktree path, resolution recommendations, and any files root should edit before the real merge.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\stage3_dry_merge_matrix.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
