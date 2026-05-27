$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: merge_stage1_tests_manifests_worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Pre-merge static review of uncommitted refactors in Cargo manifests, lockfile, and compact/test-support suites before upstream/main is merged.

DO_NOT_INSPECT: Do not run builds, tests, cargo check, cargo test, cargo metadata, or formatters. Do not inspect unrelated runtime/UI files except when a manifest or test import points there. Do not modify .codex/workflow/agents except your handoff file.

SCOUT_EVIDENCE: Root ran first_moves_predict for the upstream/main merge plan and existing wave3 worker handoffs are present. Relevant prior handoffs include solid_refactor_wave3_compact_tests_worker.handoff.md, solid_refactor_wave3_test_support_worker.handoff.md, and solid_refactor_wave3_protocol_domain_tests_worker.handoff.md.

WHY_AGENT / ROI: Independent static review can run while root coordinates core and UI reviews. new_agent_cost=3, parallel_gain=3, context_gain=2, repeat_gain=1, loop_followup_gain=2, risk_penalty=1, net=4. User requested external non-interactive top-model/high-effort workers.

FIRST_READS: Start with `git status --short --branch --untracked-files=no`, then `git diff -- codex-rs/Cargo.lock codex-rs/**/Cargo.toml codex-rs/core-test-suites`. Read the three handoff files named above before broad search. Then inspect only touched manifests and changed test files, especially codex-rs/core-test-suites/compact/Cargo.toml, codex-rs/core-test-suites/compact/tests/compact_snapshot_request_shape.rs, codex-rs/core-test-suites/compact/tests/suite/compact.rs, and any Cargo.toml files shown in the diff.

TOOL_HINTS: Use targeted text inspection only. No commands that resolve dependencies or compile code. If duplicate manifest entries or stale lockfile references are obvious from text, patch only those within scope and record them.

TOKEN_TIP: Focus on dependency names/features, test helper imports, duplicated fixtures, and lockfile churn that will make the merge harder.

VERIFICATION: Static review only. Identify likely manifest conflicts, duplicate dependency entries, stale path imports, snapshot-shape drift, or merge-sensitive repeated test blocks. If you find a critical small fix in your owned scope, patch it and record the exact file/line. Otherwise do not edit.

HANDOFF: Write .codex/workflow/agents/merge_stage1_tests_manifests_worker.handoff.md with sections: Scope, Files inspected, Changes made, Findings/blockers, Merge-risk notes, Recommended staging. Keep it concise and explicit whether you edited files.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\merge_stage1_tests_manifests_worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
