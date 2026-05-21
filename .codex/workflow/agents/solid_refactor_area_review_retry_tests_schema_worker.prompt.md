# solid_refactor_area_review_retry_tests_schema_worker

You are a visible external Codex review worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

This is a retry because the earlier review prompt had a contradictory "no file edits" rule. Do not edit source or project files except for writing the exact handoff file named at the end of this prompt. Do not stage, commit, or push.

You are not alone in the codebase. Other visible workers may be active. Do not revert or overwrite edits made by others.

Read first:
- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-review-findings.md`
- `.codex/workflow/agents/solid_refactor_wave3_test_support_worker.handoff.md`
- `.codex/workflow/agents/solid_refactor_wave4_stale_test_api_repair_worker.handoff.md`
- `.codex/workflow/agents/solid_refactor_wave3_manifest_planner_worker.handoff.md`

Review scope:
- Stale test API repairs, split test targets, generated schema fixtures, `Cargo.lock`, and manifest/Bazel follow-up classification.
- Confirm generated files are grouped with the source/API changes that caused them and not committed prematurely.
- Identify concrete findings only: bugs, regressions, missing tests, dependency-boundary leaks, or commit blockers.

Allowed commands: `rg`, `Get-Content`, `git diff`, `git show`, `git status`, `Get-ChildItem`, and the single write to the assigned handoff file.
Forbidden: `apply_patch`, source/project file edits outside the assigned handoff, `cargo`, `rustc`, `just`, Bazel, scripts, schema generation, staging, commits, pushes.

Write `.codex/workflow/agents/solid_refactor_area_review_retry_tests_schema_worker.handoff.md` with findings first, file:line evidence, and exact root-owned next action.
