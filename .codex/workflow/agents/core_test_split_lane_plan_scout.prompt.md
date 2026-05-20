# Agent Prompt: core_test_split_lane_plan_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only scout for producing a practical implementation sequence for
the `codex-core` test split. You are not alone in this worktree; do not revert
or overwrite edits from other sessions.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/test_surface_scout.handoff.md`
- `codex-rs/core/tests/all.rs`
- `codex-rs/core/tests/suite/mod.rs`
- top 10 largest files in `codex-rs/core/tests/suite/`

Task:

- Design an implementation sequence that can be done as small commits.
- Prioritize making tests compile and run fast over immediately running broad
  release tests.
- Specify which files each future worker can own without conflicts.
- Keep this read-only. Do not edit files.
- Do not run Cargo, Just, formatters, Git staging/commits, or broad build lanes.

Write `.codex/workflow/agents/core_test_split_lane_plan_scout.handoff.md` with:

- proposed commit sequence
- future worker ownership map
- what to verify after each commit
- what not to run until the split structure exists
