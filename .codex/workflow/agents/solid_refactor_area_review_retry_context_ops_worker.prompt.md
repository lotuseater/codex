# solid_refactor_area_review_retry_context_ops_worker

You are a visible external Codex review worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

This is a retry because the earlier review prompt had a contradictory "no file edits" rule. Do not edit source or project files except for writing the exact handoff file named at the end of this prompt. Do not stage, commit, or push.

You are not alone in the codebase. Other visible workers may be active. Do not revert or overwrite edits made by others.

Read first:
- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-review-findings.md`
- `.codex/workflow/agents/solid_refactor_wave4_context_ops_boundary_worker.handoff.md`

Review scope:
- Context-ops and replacement-shadow fallout, especially the deleted `codex-rs/core/src/tools/handlers/replacement_shadow.rs`.
- Confirm no live Rust references remain and dependency cleanup is correctly classified.
- Identify concrete findings only: bugs, regressions, missing tests, dependency-boundary leaks, or commit blockers.

Allowed commands: `rg`, `Get-Content`, `git diff`, `git show`, `git status`, `git ls-files`, and the single write to the assigned handoff file.
Forbidden: `apply_patch`, source/project file edits outside the assigned handoff, `cargo`, `rustc`, `just`, Bazel, scripts, schema generation, staging, commits, pushes.

Write `.codex/workflow/agents/solid_refactor_area_review_retry_context_ops_worker.handoff.md` with findings first, file:line evidence, and exact root-owned next action.
