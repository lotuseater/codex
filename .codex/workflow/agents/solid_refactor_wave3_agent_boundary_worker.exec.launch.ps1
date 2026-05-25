$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: solid_refactor_wave3_agent_boundary_worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('exec', '-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', '# solid_refactor_wave3_agent_boundary_worker

You are a separate external Codex exec worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Other external workers may edit different files. Do not revert, overwrite, reformat, or clean up changes you did not make; adapt to the current dirty tree.

First read:

- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/solid_refactor_wave1_agent_boundary_worker.prompt.md`

Ownership:

- `codex-rs/agent-*/**`
- `codex-rs/core/src/agent/**`
- agent-related core handlers only
- handoff: `.codex/workflow/agents/solid_refactor_wave3_agent_boundary_worker.handoff.md`

Do not edit session/thread files, generic tool runtime files outside agent handlers, compact tests, workspace manifests, Bazel files, generated schemas, AGENTS files, or unrelated app-server protocol files.

Task:

- Implement one concrete agent/job/policy/identity/graph ownership improvement by moving domain logic or policy out of `codex-core` where an existing/small agent crate boundary fits, or replacing concrete coupling with narrow traits/domain types.
- Avoid broad compatibility re-exports and do not add broad dependencies into `codex-core`.
- Preserve real data flow through proper models/APIs.

Hard command ban:

- Do not execute `cargo`, `rustc`, `just`, `bazel`, build scripts, test scripts, schema generation, deploy scripts, or git staging/commits.
- If verification seems needed, write the exact command in your handoff instead of running it.

Allowed commands:

- Read/search/status/diff commands such as `rg`, `Get-Content`, `git diff`, `git status`.
- `apply_patch` edits inside ownership only.

Handoff:

Write the handoff with changed files, boundary improvement, dependency impact, remaining fallout, and exact narrow verification commands for root.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\solid_refactor_wave3_agent_boundary_worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
