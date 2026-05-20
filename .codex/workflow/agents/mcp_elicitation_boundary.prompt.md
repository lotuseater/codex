# Agent Prompt: mcp_elicitation_boundary

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You own the MCP elicitation boundary lane. You are not alone in the codebase;
do not revert or overwrite changes outside this lane.

First read:

- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`

Owned paths:

- new MCP elicitation type/API crate files under `codex-rs/mcp/**` if needed
- MCP elicitation-only files under `codex-rs/tools-domain/**` if needed
- `.codex/workflow/agents/mcp_elicitation_boundary.handoff.md`

Forbidden:

- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- Bazel files and lockfiles
- app-server protocol files unless root later grants exact files
- resets or checkouts; path-scoped Git staging/commits are allowed only under
  `.codex/workflow/worker-delegation-commit-protocol.md`
- formatters, broad Cargo builds, or Just tasks

Task:

Inspect MCP elicitation request/schema type ownership. Prepare the smallest
narrow owner crate/files that let core/tools depend on MCP elicitation
abstractions instead of app-server protocol DTOs. Do not wire manifests.

Write `.codex/workflow/agents/mcp_elicitation_boundary.handoff.md` with:

- paths changed/read
- exact app-server-protocol types involved
- crate ownership recommendation
- root-owned manifest entries needed
- verification performed
- blockers
