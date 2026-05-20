# Agent Prompt: thread_store_boundary

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You own the thread-store boundary lane. You are not alone in the codebase; do
not revert or overwrite changes outside this lane.

First read:

- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`

Owned paths:

- `codex-rs/thread/thread-store-api/**`
- `codex-rs/thread/thread-store/**`
- core test fakes only if root later grants exact files
- `.codex/workflow/agents/thread_store_boundary.handoff.md`

Forbidden:

- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- Bazel files and lockfiles
- resets or checkouts; path-scoped Git staging/commits are allowed only under
  `.codex/workflow/worker-delegation-commit-protocol.md`
- formatters, broad Cargo builds, or Just tasks

Task:

Inspect concrete `codex_thread_store` usage in `codex-core` and propose or
prepare API-side abstractions/fakes needed to remove concrete store dependency
from core. Do not edit core unless root later grants exact files.

Write `.codex/workflow/agents/thread_store_boundary.handoff.md` with:

- paths changed/read
- exact concrete store symbols blocking core decoupling
- API abstraction changes needed
- root-owned manifest entries needed
- verification performed
- blockers
