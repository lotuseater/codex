$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: solid_refactor_wave3_compact_tests_worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# solid_refactor_wave3_compact_tests_worker

You are a separate external Codex exec worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Other external workers may edit different files. Do not revert, overwrite, reformat, or clean up changes you did not make; adapt to the current dirty tree.

First read:

- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/solid_refactor_wave1_compact_tests_split_worker.prompt.md`
- `.codex/workflow/agents/core_test_split_next_lane_worker.handoff.md`
- `.codex/workflow/agents/core_test_split_followup_lane_worker.handoff.md`

Ownership:

- `codex-rs/core/tests/compact.rs`
- `codex-rs/core/tests/suite/compact*.rs`
- `codex-rs/core/tests/common/compact_fixtures.rs`
- narrowly needed `codex-rs/core/Cargo.toml` compact test target entries
- handoff: `.codex/workflow/agents/solid_refactor_wave3_compact_tests_worker.handoff.md`

Do not edit unrelated core tests, runtime source, workspace root manifests, Bazel files, generated schemas, AGENTS files, app-server protocol files, session/thread source, tools source, or agent runtime source.

Task:

- Split compact-related core tests into smaller focused targets/harnesses if the current target is still over-broad.
- Move only compact fixtures/support that clearly belongs with compact tests.
- Keep protocol/domain-only fixtures separate from runtime-instantiating helpers.

Hard command ban:

- Do not execute `cargo`, `rustc`, `just`, `bazel`, build scripts, test scripts, schema generation, deploy scripts, or git staging/commits.
- If verification seems needed, write the exact command in your handoff instead of running it.

Allowed commands:

- Read/search/status/diff commands such as `rg`, `Get-Content`, `git diff`, `git status`.
- `apply_patch` edits inside ownership only.

Handoff:

Write the handoff with changed files, split target/test family, boundary improvement, remaining fallout, and exact narrow verification commands for root.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\solid_refactor_wave3_compact_tests_worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
