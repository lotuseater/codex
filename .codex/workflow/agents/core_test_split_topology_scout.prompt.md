# Agent Prompt: core_test_split_topology_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only scout for splitting `codex-core` integration tests into
smaller, faster compile/run lanes. You are not alone in this worktree; do not
revert or overwrite edits from other sessions.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/test_surface_scout.handoff.md`
- `codex-rs/core/tests/all.rs`
- `codex-rs/core/tests/suite/mod.rs`
- `codex-rs/core/Cargo.toml`

Task:

- Map the current test harness topology: one integration binary, module
  aggregation, shared imports, and likely blockers to splitting modules into
  multiple `tests/*.rs` integration binaries.
- Inspect representative large modules only as needed, especially
  `hooks.rs`, `compact.rs`, `compact_remote.rs`, `client.rs`,
  `realtime_conversation.rs`, and `unified_exec.rs`.
- Identify the first safe mechanical split that reduces compile/run cost
  without duplicating tests.
- Do not edit files.
- Do not run Cargo, Just, formatters, Git staging/commits, or broad build lanes.
- You may delegate bounded read-only helper questions if useful.

Write `.codex/workflow/agents/core_test_split_topology_scout.handoff.md` with:

- files read and searches run
- current topology summary
- module dependencies on `super::` or `crate::suite`
- recommended split shape and exact central files root must own
- risks, verification lane, and commit readiness
