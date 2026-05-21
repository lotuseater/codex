# solid_refactor_area_review_core_api_worker

You are a visible external Codex review worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Do not edit source or project files except for writing the exact handoff file named at the end of this prompt. Do not stage, commit, or push. This is a read-only handoff-review with one allowed handoff write.

Read first:
- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-review-findings.md`
- `.codex/workflow/agents/solid_refactor_wave3_core_api_boundary_worker.handoff.md`
- `.codex/workflow/agents/solid_refactor_wave4_core_api_consumer_worker.handoff.md`
- `.codex/workflow/agents/solid_refactor_wave3_manifest_planner_worker.handoff.md`

Scope:
- Review only `codex-core-api`, `codex-core-domain-types`, identifier exports, consumer imports, Cargo.lock, and later Bazel/schema implications.
- Confirm whether the `ThreadId` / `ProtocolThreadId` move is sound and whether consumer fallout remains.
- Identify exact commit boundary and verification commands for this area.

Allowed commands: `rg`, `Get-Content`, `git diff`, `git show`, `git status`, and the single write to the assigned handoff file.
Forbidden: `apply_patch`, source/project file edits outside the assigned handoff, `cargo`, `rustc`, `just`, Bazel, scripts, schema generation, staging, commits, pushes.

Write `.codex/workflow/agents/solid_refactor_area_review_core_api_worker.handoff.md` with findings first, file:line evidence, and exact root-owned next action.
