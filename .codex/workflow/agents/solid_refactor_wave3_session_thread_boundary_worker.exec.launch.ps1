$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: solid_refactor_wave3_session_thread_boundary_worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('exec', '-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', '# solid_refactor_wave3_session_thread_boundary_worker

You are a separate external Codex exec worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Other external workers may edit different files. Do not revert, overwrite, reformat, or clean up changes you did not make; adapt to the current dirty tree.

First read:

- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/solid_refactor_wave1_session_thread_worker.handoff.md`

Ownership:

- `codex-rs/core/src/session/**`
- `codex-rs/thread-store-api/**`
- `codex-rs/thread-store/**`
- narrowly related session/thread call sites in `codex-rs/core/src/**`
- handoff: `.codex/workflow/agents/solid_refactor_wave3_session_thread_boundary_worker.handoff.md`

Do not edit tools, agent handlers, core test targets, workspace manifests, Bazel files, generated schemas, AGENTS files, or unrelated app-server protocol files.

Task:

- Continue the session/thread boundary refactor only if source inspection finds remaining concrete leakage into `codex-core`.
- If the prior session/thread pass is already complete, write a no-change handoff with the exact source checks you ran.
- Avoid broad compatibility re-exports and do not add broad dependencies into `codex-core`.

Hard command ban:

- Do not execute `cargo`, `rustc`, `just`, `bazel`, build scripts, test scripts, schema generation, deploy scripts, or git staging/commits.
- If verification seems needed, write the exact command in your handoff instead of running it.

Allowed commands:

- Read/search/status/diff commands such as `rg`, `Get-Content`, `git diff`, `git status`.
- `apply_patch` edits inside ownership only.

Handoff:

Write the handoff with changed files or no-change status, boundary evidence, remaining fallout, and exact narrow verification commands for root.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\solid_refactor_wave3_session_thread_boundary_worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
