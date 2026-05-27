$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: stage3_manifest_lock'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Stage 3 manifests/dependencies conflict check for branch slow-context-budget-mode in C:\Users\Oleh\Documents\GitHub\open_ai\codex. Focus only on codex-rs/Cargo.lock, codex-rs/codex-mcp/Cargo.toml, and codex-rs/tui/Cargo.toml. HEAD is 14a9f24005; upstream/main is 9f42c89c01. Current dirty changes intentionally remove local v8-path-only manifest wiring after stage-2 review.

DO_NOT_INSPECT: Do not inspect unrelated code. Do not run cargo, rustc, just, build/test scripts, schema generation, or deploy. Do not mutate the root checkout except writing your handoff under C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage3-20260525-181500\stage3_manifest_lock\.handoff\.md. Do not spawn agents.

SCOUT_EVIDENCE: Root first_moves_predict and the completed stage-2 manifest handoff both identified manifest/lockfile as likely conflict-prone because upstream/main changed pinned v8/rusty_v8 while this branch had local path deps.

WHY_AGENT / ROI: Area-specific manifest review can run in parallel with the dry-merge matrix and reduces risk of cargo metadata damage. Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=2, risk_penalty=1, net=6.

FIRST_READS: git diff -- codex-rs/Cargo.lock codex-rs/codex-mcp/Cargo.toml codex-rs/tui/Cargo.toml; git diff upstream/main...HEAD -- the same paths; optionally perform your own temp-worktree dry merge limited to evidence gathering.

TASK: Determine whether the current manifest/lockfile pre-refactor is sufficient to avoid or simplify merge conflicts with upstream/main. If a dry merge conflicts, identify exact dependency entries to keep from upstream/main vs branch. Do not edit. Treat upstream/main dependency versions and lockfile package checksums as authoritative unless this branch still has a necessary local-only dependency.

TOOL_HINTS: Use git show/diff for specific files; avoid broad search. You may create a temp worktree under C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage3-20260525-181500\stage3_manifest_lock\worktree if useful.

TOKEN_TIP: Report concrete dependency names and files only.

VERIFICATION: Read/dry-merge only. No build/test.

HANDOFF: Write C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage3-20260525-181500\stage3_manifest_lock\.handoff\.md with conflict risk, recommended resolution, and whether any pre-merge edit is still needed.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\stage3_manifest_lock.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
