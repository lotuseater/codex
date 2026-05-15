# Repo Memory: Local Fork Features

- On every recurring `main` merge or large refactor, check `docs/fork-feature-inventory.md`.
- Treat the feature families in that document as local fork behavior that must be preserved unless intentionally removed.
- Choose engineering steps by expected long-term value, impact, dependency order, and ownership clarity, not by minimum risk or the safest-looking patch. Use safety, reversibility, canaries, scratch branches, and checkpoints as constraints that enable ambitious work.
- If conflicts touch an owner path, run that feature family's focused health checks before FastRelease build/deploy.
- Update the inventory after adding a new local feature or after moving a feature to a new owner crate/module.
- Use `scripts/analyze-branch-conflict-surface.ps1 -BaseRef origin/main -IncludeWorkingTree -Top 20` before recurring `main` merges to identify fork logic that should move out of upstream-hot files first.
- For recurring upstream-main merge work, follow `docs/upstream-main-merge-iteration.md`: rehearse on a temporary branch/worktree, refactor there to reduce future conflicts, retry, then bring the successful verified result back to the working branch.
