# Agent Prompt: dab_internal_canary_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only scout for the internal DAB availability slice. You are not
alone in this worktree; do not revert or overwrite edits from other sessions.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/dab_availability_worker.handoff.md`
- `codex-rs/core/src/tools/handlers/mod.rs`
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/spec_plan_tests.rs`
- `.codex/prototypes/check-core-boundaries.ps1`

Task:

- Inspect the DAB worker's source changes and confirm whether they use internal
  Codex DAB wiring rather than external Wizard DAB.
- Identify the smallest canary/test command root should run after compile
  blockers are cleared.
- Do not edit source files, manifests, lockfiles, Bazel files, generated files,
  tests, or snapshots.
- Do not run Cargo, Just, formatters, Git staging/commits, broad build lanes, or
  external Wizard DAB.
- You may delegate bounded read-only questions to helper agents.

Write `.codex/workflow/agents/dab_internal_canary_scout.handoff.md` with:

- files read and findings
- whether DAB wiring appears internal-only
- exact verification command root should run later
- blockers and commit readiness
