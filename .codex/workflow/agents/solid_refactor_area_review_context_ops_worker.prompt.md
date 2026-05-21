# solid_refactor_area_review_context_ops_worker

You are a visible external Codex review worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Do not edit files. Do not stage, commit, or push. This is a read-only handoff-review.

Read first:
- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-review-findings.md`
- `.codex/workflow/agents/solid_refactor_wave4_context_ops_boundary_worker.handoff.md`
- `.codex/workflow/agents/solid_refactor_wave3_dependency_scout_worker.handoff.md`

Scope:
- Review only context-ops and replacement-shadow fallout.
- Confirm whether deleting `codex-rs/core/src/tools/handlers/replacement_shadow.rs` is safe.
- Identify exactly which `codex-rs/core/Cargo.toml` deps and Bazel entries are dead now, and which are still required by live context-ops handlers.

Allowed commands: `rg`, `Get-Content`, `git diff`, `git show`, `git status`, `git ls-files`.
Forbidden: `apply_patch`, file edits, `cargo`, `rustc`, `just`, Bazel, scripts, schema generation, staging, commits, pushes.

Write `.codex/workflow/agents/solid_refactor_area_review_context_ops_worker.handoff.md` with findings first, file:line evidence, and exact root-owned next action.
