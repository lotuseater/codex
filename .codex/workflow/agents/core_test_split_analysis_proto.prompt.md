# Agent Prompt: core_test_split_analysis_proto

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are an edit-owned prototype worker for `codex-core` test split analysis. You
are not alone in this worktree; do not revert or overwrite edits from other
sessions.

Owned edit paths:

- `.codex/prototypes/plan-core-test-split.ps1`
- `.codex/workflow/agents/core_test_split_analysis_proto.handoff.md`

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/core/tests/all.rs`
- `codex-rs/core/tests/suite/mod.rs`
- `codex-rs/core/tests/common/lib.rs`

Task:

- Create a small PowerShell prototype that inventories
  `codex-rs/core/tests/suite/*.rs` and reports:
  - file size
  - approximate `#[test]` / `#[tokio::test]` counts
  - `super::` references
  - notable imports/dependency hints
  - a suggested lane label if straightforward
- The script must be read-only against source files.
- You may run the script and include a compact sample output in your handoff.
- Do not edit Rust source, manifests, lockfiles, Bazel files, generated files,
  snapshots, or workflow files outside your owned handoff.
- Do not run Cargo, Just, formatters, Git staging/commits, or broad build lanes.
- You may delegate bounded read-only helper questions if useful.

Write `.codex/workflow/agents/core_test_split_analysis_proto.handoff.md` with:

- files changed
- script usage
- sample findings
- recommended next root action
