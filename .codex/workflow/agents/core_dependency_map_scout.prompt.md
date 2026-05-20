# Agent Prompt: core_dependency_map_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only `codex-core` dependency map scout.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/agents/manifest_wiring_scout.handoff.md`
- `.codex/workflow/agents/boundary_delta_scout.handoff.md`

Task:

- Identify current direct and likely transitive dependency leaks from
  `codex-core` into outer app/protocol/server/tooling crates.
- Use targeted searches around `codex_app_server_protocol`,
  `codex_app_server`, `codex_connectors`, `codex_mcp`, and newly introduced
  domain/API crates.
- Do not edit source files, manifests, lockfiles, generated files, or handoff
  documents other than your own handoff.
- Do not run Cargo, Just, formatters, or Git staging/commits.
- You may delegate focused read-only questions to helper agents if useful.

Write `.codex/workflow/agents/core_dependency_map_scout.handoff.md` with:

- exact forbidden imports/dependencies still visible from `codex-core`
- candidate owner crate or boundary API for each leak
- recommended implementation order
- files likely owned by each future implementation slice
- commit readiness notes, but do not make commits
