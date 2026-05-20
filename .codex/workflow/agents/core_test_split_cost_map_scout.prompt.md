# Agent Prompt: core_test_split_cost_map_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only scout for estimating `codex-core` test split cost and
grouping. You are not alone in this worktree; do not revert or overwrite edits
from other sessions.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/core/tests/all.rs`
- `codex-rs/core/tests/suite/mod.rs`
- the file inventory under `codex-rs/core/tests/suite/`

Task:

- Build a ranked map of test modules by size, approximate test count,
  async/runtime use, network/mock use, snapshot/golden use, and likely compile
  heaviness.
- Propose 3-6 split lanes with names and module membership, optimizing for fast
  targeted release test iteration.
- Keep this read-only. Do not edit files.
- Do not run Cargo, Just, formatters, Git staging/commits, or broad build lanes.
- You may use `rg`, `Get-ChildItem`, and simple read-only commands.

Write `.codex/workflow/agents/core_test_split_cost_map_scout.handoff.md` with:

- ranked module table
- suggested lane names and exact module sets
- first lane to implement
- verification strategy after splitting
