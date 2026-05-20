# Agent Prompt: bazel_lock_rescue

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a replacement read-only scout for the hung `bazel_lock_scout`. You are
not alone in this worktree; do not revert or overwrite edits from other sessions.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/agents/manifest_wiring_scout.handoff.md`
- `.codex/workflow/agents/protocol_schema_scout.handoff.md`
- `codex-rs/Cargo.toml`

Task:

- Inspect manifest, workspace dependency, and Bazel-lock impact of the prepared
  boundary crates and current dirty manifest changes.
- Focus on what root must own: `codex-rs/Cargo.toml`, crate `Cargo.toml` files,
  `Cargo.lock`, Bazel files, and `MODULE.bazel.lock`.
- Do not edit any files.
- Do not run `just bazel-lock-update`, `just bazel-lock-check`, Cargo, Just,
  formatters, Git staging/commits, or broad build lanes.
- You may delegate bounded read-only searches to helper agents.

Write `.codex/workflow/agents/bazel_lock_rescue.handoff.md` with:

- exact manifest/lock files read
- manifest wiring status for new crates
- Bazel/lock refresh commands root should run later and prerequisites
- risks if root commits source before lock/schema refresh
- commit grouping recommendation
