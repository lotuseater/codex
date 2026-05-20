# Agent Prompt: core_test_split_cargo_bazel_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only scout for Cargo/Bazel wiring needed to split `codex-core`
integration tests. You are not alone in this worktree; do not revert or
overwrite edits from other sessions.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/core/Cargo.toml`
- `codex-rs/core/BUILD.bazel`
- `codex-rs/core/tests/all.rs`
- `codex-rs/core/tests/common/BUILD.bazel`
- any Bazel files that currently mention `core/tests/all.rs` or
  `codex_core`

Task:

- Determine whether adding multiple `codex-rs/core/tests/*.rs` integration test
  binaries requires Cargo manifest changes, Bazel target changes, both, or
  neither.
- Identify lock/schema/generated artifacts that root must own.
- Keep this read-only. Do not edit files.
- Do not run Cargo, Just, Bazel, formatters, Git staging/commits, or broad build
  lanes.
- You may use targeted `rg` and file reads.

Write `.codex/workflow/agents/core_test_split_cargo_bazel_scout.handoff.md`
with:

- Cargo behavior and exact expected test binary names
- Bazel impact and exact files root must edit
- minimal first split that avoids manifest churn if possible
- verification commands after structure changes
