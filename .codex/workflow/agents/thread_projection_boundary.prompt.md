# Agent Prompt: thread_projection_boundary

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You own the thread projection boundary lane. You are not alone in the codebase;
do not revert or overwrite changes outside this lane.

First read:

- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`

Owned paths:

- new thread projection type/API crate files under `codex-rs/thread/**`
- `.codex/workflow/agents/thread_projection_boundary.handoff.md`

Forbidden:

- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- Bazel files and lockfiles
- app-server protocol files unless root later grants exact files
- resets or checkouts; path-scoped Git staging/commits are allowed only under
  `.codex/workflow/worker-delegation-commit-protocol.md`
- formatters, broad Cargo builds, or Just tasks

Task:

Inspect `ThreadHistoryBuilder`, `TurnStatus`, and related projection DTOs.
Prepare a narrow thread projection owner so core-facing code can depend on
thread projection abstractions/types instead of app-server protocol DTOs. Do
not wire manifests.

Write `.codex/workflow/agents/thread_projection_boundary.handoff.md` with:

- paths changed/read
- exact app-server-protocol types involved
- crate ownership recommendation
- root-owned manifest entries needed
- verification performed
- blockers
