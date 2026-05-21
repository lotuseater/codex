# solid_refactor_commit_grouping_worker

You are a visible external Codex worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Other visible workers may still be running. Do not revert edits made by others. This task is read-only classification, not implementation.

Goal:

Classify the dirty tree into coherent commit/push groups for the SOLID refactor. Identify which groups are immediately useful and commit-ready, which require repair/verification first, and which are unrelated/noise.

Read first:

- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/solid_refactor_wave3_*.handoff.md`
- `.codex/workflow/agents/solid_refactor_wave4_*.handoff.md`
- If present, `.codex/workflow/agents/solid_refactor_review_handoffs_worker.handoff.md`

Scope:

1. Use `git status --short` and focused `git diff --name-status` to group dirty files by owner/slice.
2. Compare groups with completed handoffs.
3. Mark each group as:
   - `commit-ready`: useful, coherent, no known blocking review issue, and only needs root's final allowed verification before commit.
   - `needs-fix`: useful but has a concrete source/review/verification blocker.
   - `defer`: useful but depends on another group landing first.
   - `exclude`: tmp/noise/generated artifact that should not be committed.
4. Include exact suggested `git add` path lists per group. Do not stage anything.
5. Include exact narrow verification commands root should run before each commit.
6. Include suggested commit messages and whether the commit should be pushed immediately after it lands.

Hard command ban:

- Do not edit files.
- Do not run `cargo`, `rustc`, `just`, Bazel, build/test scripts, schema generation, deploy scripts, git staging, commits, pushes, or destructive commands.
- Allowed: `rg`, `Get-Content`, `git diff`, `git status`, `git show`, `git ls-files`, `Get-ChildItem`.

Handoff:

Write `.codex/workflow/agents/solid_refactor_commit_grouping_worker.handoff.md`.

The handoff must start with a short table:

`group | status | files | verification | suggested commit`

Then provide details and exclusions.
