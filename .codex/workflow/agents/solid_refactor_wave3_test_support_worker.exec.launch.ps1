$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: solid_refactor_wave3_test_support_worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('exec', '-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', '# solid_refactor_wave3_test_support_worker

You are a separate external Codex exec worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Other external workers may edit different files. Do not revert, overwrite, reformat, or clean up changes you did not make; adapt to the current dirty tree.

First read:

- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/core_test_split_next_lane_worker.handoff.md`
- `.codex/workflow/agents/core_test_split_followup_lane_worker.handoff.md`

Ownership:

- `codex-rs/core/tests/common/**`
- shared support wiring in `codex-rs/core/tests/common.rs` or equivalent
- narrowly needed `codex-rs/core/Cargo.toml` test target entries for support split only
- handoff: `.codex/workflow/agents/solid_refactor_wave3_test_support_worker.handoff.md`

Do not edit runtime source, suite test bodies outside support imports, compact-specific tests, protocol-domain split files owned by the protocol worker, workspace root manifests, Bazel files, generated schemas, AGENTS files, or app-server protocol files.

Task:

- Separate protocol/domain fixtures from helpers that instantiate `codex-core` runtime behavior.
- Keep shared fixtures below the runtime harness layer.
- Prefer a coherent support split over broad churn.

Hard command ban:

- Do not execute `cargo`, `rustc`, `just`, `bazel`, build scripts, test scripts, schema generation, deploy scripts, or git staging/commits.
- If verification seems needed, write the exact command in your handoff instead of running it.

Allowed commands:

- Read/search/status/diff commands such as `rg`, `Get-Content`, `git diff`, `git status`.
- `apply_patch` edits inside ownership only.

Handoff:

Write the handoff with changed files, support boundary improvement, remaining fallout, and exact narrow verification commands for root.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\solid_refactor_wave3_test_support_worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
