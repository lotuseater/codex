# Agent Prompt: app_server_boundary_rescue

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a replacement read-only scout for the hung `app_server_boundary_scout`.
You are not alone in this worktree; do not revert or overwrite edits from other
sessions.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/agents/core_dependency_map_scout.handoff.md`
- `.codex/workflow/agents/protocol_schema_scout.handoff.md`
- `.codex/workflow/agents/integration_order_scout.handoff.md`

Task:

- Inspect app-server boundary leaks and app-server owned DTO/domain mapping seams.
- Focus on `codex-rs/app-server/src/**`, `codex-rs/app-server-protocol/src/**`,
  and existing app/thread/turn domain API crates.
- Identify the smallest non-overlapping implementation slice that reduces
  direct protocol/core leakage without expanding `codex-core`.
- Do not edit source files, manifests, lockfiles, Bazel files, generated schema
  fixtures, or snapshots.
- Do not run Cargo, Just, formatters, Git staging/commits, or broad build lanes.
- You may delegate bounded read-only questions to helper agents, but keep this
  handoff as the authoritative result.

Write `.codex/workflow/agents/app_server_boundary_rescue.handoff.md` with:

- files read and exact searches run
- current app-server boundary findings
- recommended path-owned implementation slice
- files that must be root-owned
- blockers and verification lane
- commit readiness and exact pathspec recommendation if applicable
