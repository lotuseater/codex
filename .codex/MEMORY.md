# Repo Memory: Local Fork Features

- On every recurring `main` merge or large refactor, check `docs/fork-feature-inventory.md`.
- Treat the feature families in that document as local fork behavior that must be preserved unless intentionally removed.
- If conflicts touch an owner path, run that feature family's focused health checks before FastRelease build/deploy.
- Update the inventory after adding a new local feature or after moving a feature to a new owner crate/module.
- Use `scripts/analyze-branch-conflict-surface.ps1 -BaseRef origin/main -IncludeWorkingTree -Top 20` before recurring `main` merges to identify fork logic that should move out of upstream-hot files first.
