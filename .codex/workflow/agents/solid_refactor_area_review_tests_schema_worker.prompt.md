# solid_refactor_area_review_tests_schema_worker

You are a visible external Codex review worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Do not edit files. Do not stage, commit, or push. This is a read-only handoff-review.

Read first:
- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-review-findings.md`
- `.codex/workflow/agents/solid_refactor_wave3_test_support_worker.handoff.md`
- `.codex/workflow/agents/solid_refactor_wave3_compact_tests_worker.handoff.md`
- `.codex/workflow/agents/solid_refactor_wave4_stale_test_api_repair_worker.handoff.md`
- `.codex/workflow/agents/solid_refactor_wave3_manifest_planner_worker.handoff.md`

Scope:
- Review only core test support/test target splits, stale test API repairs, generated app-server schema/config schema files, Cargo.lock, and manifest/Bazel follow-up grouping.
- Identify generated or temporary files that should be excluded from commits.
- Identify exact commit boundaries and verification commands for tests/schema/manifest work.

Allowed commands: `rg`, `Get-Content`, `git diff`, `git show`, `git status`, `Get-ChildItem`.
Forbidden: `apply_patch`, file edits, `cargo`, `rustc`, `just`, Bazel, scripts, schema generation, staging, commits, pushes.

Write `.codex/workflow/agents/solid_refactor_area_review_tests_schema_worker.handoff.md` with findings first, file:line evidence, and exact root-owned next action.
