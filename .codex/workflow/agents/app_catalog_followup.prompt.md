# Agent Prompt: app_catalog_followup

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You own the app catalog follow-up lane. You are not alone in the codebase; do
not revert or overwrite changes outside this lane.

First read:

- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`

Owned paths:

- `codex-rs/app/app-catalog-types/**`
- `codex-rs/app/app-catalog-api/**`
- app catalog conversion helpers only if root later grants exact files
- `.codex/workflow/agents/app_catalog_followup.handoff.md`

Forbidden:

- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- Bazel files and lockfiles
- app-server protocol files unless root later grants exact files
- resets or checkouts; path-scoped Git staging/commits are allowed only under
  `.codex/workflow/worker-delegation-commit-protocol.md`
- formatters, broad Cargo builds, or Just tasks

Task:

Inspect app catalog domain/wire separation. Identify any remaining
app-server-protocol imports in core/connectors/tools that should use
`codex-app-catalog-types` or `codex-app-catalog-api` instead. Make lane-owned
type/API improvements only if needed; root will wire callers.

Write `.codex/workflow/agents/app_catalog_followup.handoff.md` with:

- paths changed/read
- remaining app catalog protocol leaks
- crate ownership recommendation
- root-owned manifest entries needed
- verification performed
- blockers
