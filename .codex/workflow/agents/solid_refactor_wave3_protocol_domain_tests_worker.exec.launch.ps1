$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: solid_refactor_wave3_protocol_domain_tests_worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('exec', '-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', '# solid_refactor_wave3_protocol_domain_tests_worker

You are a separate external Codex exec worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Other external workers may edit different files. Do not revert, overwrite, reformat, or clean up changes you did not make; adapt to the current dirty tree.

First read:

- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/core_test_split_next_lane_worker.handoff.md`
- `.codex/workflow/agents/core_test_split_followup_lane_worker.handoff.md`

Ownership:

- protocol/domain-only core tests under `codex-rs/core/tests/**`
- protocol/domain fixture support under `codex-rs/core/tests/common/**`
- narrowly needed `codex-rs/core/Cargo.toml` test target entries for that split
- excluding compact files owned by the compact worker
- handoff: `.codex/workflow/agents/solid_refactor_wave3_protocol_domain_tests_worker.handoff.md`

Do not edit core runtime source except tiny support imports required by moved tests. Do not edit compact files, workspace root manifests, Bazel files, generated schemas, AGENTS files, app-server protocol files, session/thread source, tools source, or agent runtime source.

Task:

- Identify one protocol/domain-only core test family that can be split from runtime-instantiating core tests.
- Move or retarget that family into a smaller test target/harness with minimal dependency fan-in.
- Keep runtime helpers out of protocol/domain-only fixture support.

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
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\solid_refactor_wave3_protocol_domain_tests_worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
