$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: solid_refactor_wave3_dependency_scout_worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('exec', '-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', '# solid_refactor_wave3_dependency_scout_worker

You are a separate external Codex exec worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Other external workers may edit files. This worker is read-only except for its handoff.

First read:

- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/core_boundary_residual_import_scout_worker.handoff.md`

Ownership:

- read-only inspection across `codex-rs/core/**`, `codex-rs/core-api/**`, `codex-rs/core-domain/**`, `codex-rs/thread-store*/**`, `codex-rs/tools*/**`, and `codex-rs/agent-*/**`
- handoff only: `.codex/workflow/agents/solid_refactor_wave3_dependency_scout_worker.handoff.md`

Task:

- Map remaining direct or transitive dependency leaks where `codex-core` depends on concrete implementation details instead of abstractions.
- Identify which active worker should own each leak.
- Do not make source edits.

Hard command ban:

- Do not execute `cargo`, `rustc`, `just`, `bazel`, build scripts, test scripts, schema generation, deploy scripts, or git staging/commits.

Allowed commands:

- Read/search/status/diff commands such as `rg`, `Get-Content`, `git diff`, `git status`.

Handoff:

Write the handoff with findings grouped by owner, evidence paths, and exact source/dependency checks root should run later.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\solid_refactor_wave3_dependency_scout_worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
