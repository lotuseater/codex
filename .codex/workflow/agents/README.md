# SOLID Refactor Agent Work Queue

External Codex sessions use this directory to coordinate with the root
director.

Rules for every session:

- Read `.codex/workflow/solid-refactor-handoff.md` first.
- Read `.codex/workflow/worker-delegation-commit-protocol.md` before editing.
- Do not spawn additional worker sessions or subagents unless root explicitly
  assigns that.
- Do not edit `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`, Bazel files, or
  lockfiles unless a prompt explicitly grants that.
- Do not run builds or tests while your owned refactor is still in progress.
  After the refactor is complete, run only the focused verification lane your
  prompt allows.
- Edit only owned paths.
- Write final state to the assigned `*.handoff.md` file.
- Commit coherent scoped changes when safe. If a clean commit is blocked, leave
  changes unstaged or path-staged only and record the exact blocker.

Root remains the only integrator.
