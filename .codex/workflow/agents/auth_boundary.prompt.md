# Agent Prompt: auth_boundary

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You own the auth boundary lane. You are not alone in the codebase; do not
revert or overwrite changes outside this lane.

First read:

- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`

Owned paths:

- `codex-rs/runtime-domain/auth-api/**`
- new auth-only crate files under `codex-rs/auth/**` if needed
- `.codex/workflow/agents/auth_boundary.handoff.md`

Forbidden:

- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- Bazel files and lockfiles
- app-server protocol files unless root later grants exact files
- resets or checkouts; path-scoped Git staging/commits are allowed only under
  `.codex/workflow/worker-delegation-commit-protocol.md`
- formatters, broad Cargo builds, or Just tasks

Task:

Inspect remaining `AuthMode` ownership/use and prepare the narrow auth
abstraction/type crate changes needed to remove app-server-protocol ownership
from core-facing code. Prefer using the existing `runtime-domain/auth-api` if it
is the right owner. If a neighboring auth crate is better, explain why and
create only the lane-owned crate files; root will wire manifests.

Write `.codex/workflow/agents/auth_boundary.handoff.md` with:

- paths changed/read
- exact remaining imports/callers
- crate ownership recommendation
- root-owned manifest entries needed
- verification performed
- blockers
