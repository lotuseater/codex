$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: solid_refactor_wave3_manifest_planner_worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# solid_refactor_wave3_manifest_planner_worker

You are a separate external Codex exec worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Other external workers may edit files. This worker is read-only except for its handoff.

First read:

- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/solid_refactor_wave1_manifest_bazel_worker.handoff.md`
- `.codex/workflow/agents/manifest_bazel_followup_planner_worker.handoff.md`

Ownership:

- read-only inspection of `codex-rs/**/Cargo.toml`, `codex-rs/**/BUILD.bazel`, `codex-rs/Cargo.toml`
- handoff only: `.codex/workflow/agents/solid_refactor_wave3_manifest_planner_worker.handoff.md`

Task:

- Prepare the manifest/Bazel update queue implied by current source/test split work.
- Do not edit manifests or BUILD files yet; source owners must finish first.
- Do not add `codex-app-server-protocol` or other broad implementation dependencies back into `codex-core`.

Hard command ban:

- Do not execute `cargo`, `rustc`, `just`, `bazel`, build scripts, test scripts, schema generation, deploy scripts, or git staging/commits.

Allowed commands:

- Read/search/status/diff commands such as `rg`, `Get-Content`, `git diff`, `git status`.

Handoff:

Write the handoff with deferred manifest/Bazel changes grouped by source owner and exact later verification commands.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\solid_refactor_wave3_manifest_planner_worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
