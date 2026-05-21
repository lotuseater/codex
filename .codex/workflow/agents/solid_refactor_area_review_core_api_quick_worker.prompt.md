# solid_refactor_area_review_core_api_quick_worker

You are a visible external Codex review worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

This is a narrow fallback because the broader core-api retry review is taking too long. Do not edit source or project files except for writing the exact handoff file named at the end of this prompt. Do not stage, commit, or push.

You are not alone in the codebase. Other visible workers may be active. Do not revert or overwrite edits made by others.

Read first:
- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-review-findings.md`
- `.codex/workflow/agents/solid_refactor_wave3_core_api_boundary_worker.handoff.md`
- `.codex/workflow/agents/solid_refactor_wave4_core_api_consumer_worker.handoff.md`

Answer only these questions:
1. Does the current dirty source tree have a concrete import/API regression from the core-api identifier move?
2. Are `Cargo.lock`, Bazel lock/build files, and app-server schema JSON correctly grouped as follow-up artifacts, or is any one of them unsafe to commit with the core-api slice?
3. What exact root-owned next action should happen before committing the core-api source slice?

Allowed commands: `rg`, `Get-Content`, `git diff`, `git show`, `git status`, `git ls-files`, `Get-ChildItem`, and the single write to the assigned handoff file.
Forbidden: `apply_patch`, source/project file edits outside the assigned handoff, `cargo`, `rustc`, `just`, Bazel, scripts, schema generation, staging, commits, pushes.

Write `.codex/workflow/agents/solid_refactor_area_review_core_api_quick_worker.handoff.md` with findings first, file:line evidence, and exact root-owned next action.
