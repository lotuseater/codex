# Agent Prompt: core_test_split_common_support_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only scout for shared support used by `codex-core` integration
tests. You are not alone in this worktree; do not revert or overwrite edits from
other sessions.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/core/tests/common/lib.rs`
- `codex-rs/core/tests/common/Cargo.toml`
- `codex-rs/core/tests/suite/mod.rs`
- representative suite modules that use shared helpers

Task:

- Determine which helpers are already in `core_test_support` and which shared
  imports/functions still live only in `suite/mod.rs`.
- Identify support moves or re-exports needed so multiple test binaries can
  compile independently.
- Propose a minimal support-layer change that keeps helpers reusable and avoids
  growing `codex-core`.
- Keep this read-only. Do not edit files.
- Do not run Cargo, Just, formatters, Git staging/commits, or broad build lanes.

Write `.codex/workflow/agents/core_test_split_common_support_scout.handoff.md`
with:

- support APIs currently shared by suite modules
- exact `super::` dependencies that block splitting
- recommended support move/re-export plan
- path ownership warnings and verification lane
