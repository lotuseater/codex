# Agent Prompt: thread_store_integration_rescue

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a replacement read-only scout for the hung
`thread_store_integration_scout`. You are not alone in this worktree; do not
revert or overwrite edits from other sessions.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/thread_store_boundary.handoff.md`
- `.codex/workflow/agents/compile_session_store_scout.handoff.md`
- `.codex/workflow/agents/integration_order_scout.handoff.md`
- `.codex/workflow/agents/core_dependency_map_scout.handoff.md`

Task:

- Inspect the thread-store/core integration blockers without editing files.
- Focus on `codex-rs/core/src/thread_manager.rs`,
  `codex-rs/core/src/prompt_debug.rs`, `codex-rs/core/src/session/**`,
  `codex-rs/core/tests/common/**`, and `codex-rs/thread/**`.
- Produce a non-overlapping path-ownership plan for the next implementation
  worker that preserves the rule: do not add `codex-thread-store` as a
  `codex-core` dependency.
- Do not edit source files, manifests, lockfiles, Bazel files, generated files,
  or snapshots.
- Do not run Cargo, Just, formatters, Git staging/commits, or broad build lanes.
- You may delegate bounded read-only searches to helper agents.

Write `.codex/workflow/agents/thread_store_integration_rescue.handoff.md` with:

- exact compile blockers and files read
- recommended implementation order
- path ownership warnings
- root-owned manifest/build actions, if any
- verification lane and commit readiness
