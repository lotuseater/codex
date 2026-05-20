# Agent Prompt: hook_runtime_compile_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only scout for the hook runtime compile blocker. You are not alone
in this worktree; do not revert or overwrite edits from other sessions.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/compile_hook_skill_scout.handoff.md`
- `codex-rs/core/src/hook_runtime.rs`
- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/tasks/mod.rs`

Task:

- Inspect the stale `HookExecutionDisposition` callsites and the current
  `HookRuntimeOutcome` contract.
- Produce an exact implementation plan or small patch sketch for a later worker.
- Identify whether this can be edited independently of session input queue and
  thread-store integration work.
- Do not edit source files, manifests, lockfiles, Bazel files, generated files,
  tests, or snapshots.
- Do not run Cargo, Just, formatters, Git staging/commits, or broad build lanes.
- You may delegate bounded read-only questions to helper agents.

Write `.codex/workflow/agents/hook_runtime_compile_scout.handoff.md` with:

- exact stale callsites and replacement shape
- path ownership/conflict risks
- recommended worker prompt if safe
- verification lane
- commit readiness
