# Agent Prompt: skill_dependency_compile_worker

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are an edit-owned implementation worker for the narrow skill dependency
compile blocker. You are not alone in this worktree; preserve edits from other
sessions and do not revert unrelated changes.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/agents/compile_hook_skill_scout.handoff.md`
- `codex-rs/core/src/skills.rs`
- `codex-rs/core-skills/src/lib.rs`
- files under `codex-rs/core-skills/src/` that define skill dependency
  collection/resolution
- `codex-rs/core/src/session/turn.rs` only to understand the expected call
  signature

Owned edit paths:

- `codex-rs/core/src/skills.rs`
- `codex-rs/core-skills/src/lib.rs`
- a focused file under `codex-rs/core-skills/src/` only if needed to expose an
  existing skill dependency resolver
- `.codex/workflow/agents/skill_dependency_compile_worker.handoff.md`

Rules:

- Do not edit `codex-rs/core/src/session/turn.rs`, hook runtime files, tasks,
  manifests, lockfiles, Bazel files, generated files, tests, or snapshots.
- Do not run Cargo, Just, formatters, Git staging/commits, or broad build lanes.
- You may run targeted `rg`/read-only searches.
- You may delegate bounded read-only questions, but keep ownership of the edit.
- Prefer thin re-exports/adapters around existing `codex-core-skills` logic over
  duplicating dependency resolution behavior in `codex-core`.

Task:

- Restore `skills::SkillDependency`, `skills::SkillResolution`, and
  `skills::resolve_skill_dependencies_for_turn` exports expected by
  `codex-rs/core/src/lib.rs` and `codex-rs/core/src/session/turn.rs`, using the
  current `codex-core-skills` source of truth.

Write `.codex/workflow/agents/skill_dependency_compile_worker.handoff.md` with:

- files read and changed
- exact symbols restored
- commands/searches used for verification
- remaining compile blockers outside your owned files
- exact commit pathspec if root can commit this slice after verification
