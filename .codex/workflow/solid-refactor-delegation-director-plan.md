# SOLID Refactor Delegation Director Plan

Date: 2026-05-20

## Current Objective

Continue the `codex-rs` SOLID refactor by splitting responsibilities into small
crates and moving dependency edges toward narrow abstractions.

The target rule is strict:

- Inner/domain/API crates must not depend directly or transitively on
  `codex-core`.
- Inner/domain/API crates must not depend directly or transitively on
  `codex-app-server-protocol`.
- `codex-app-server-protocol` remains an outer wire DTO crate.
- Prefer small ownership crates over a broad `core-api` replacement. Examples:
  `auth-api`, `app-catalog-types`, `app-catalog-api`, `thread-store-api`,
  `turn-*`, `session-*`, and `runtime-*`.
- Adjacent crates are allowed when they reduce coupling with adjacent areas and
  have one clear responsibility. Workers must not invent neighboring crates just
  to avoid doing the assigned slice.

Temporary compile breakage is acceptable while dependency direction is being
repaired. Structural canaries and focused release checks should be restored as
each coherent slice is integrated.

## Director Model

Root agent is the director and only integrator.

Root owns:

- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- Bazel files and lockfiles
- Git state, staging, commits, and merge state
- workspace-wide formatting and fix commands
- broad verification and boundary canaries
- final decisions when worker findings conflict

Workers own only explicitly assigned paths and must report through handoff files
under `.codex/workflow/agents/`.

## Delegation Policy

Use the built-in subagent limit for compact high-value lanes:

- SOLID overseer: review planned and actual edges for boundary violations.
- Boundary scout: identify exact remaining protocol/core/thread-store leaks.
- Focused implementer: work on one file/module slice when root has already set
  the manifest and ownership direction.

The user also approved external Codex sessions in separate PowerShell tabs to
increase parallelism beyond the built-in helper limit. External sessions must
receive the same contract as built-in workers:

- exact owned paths
- forbidden paths
- explicit non-goals
- no Git/staging/commits
- no root manifests, lockfiles, Bazel, or formatters unless root grants them
- no broad Cargo/Just builds unless root grants them
- handoff path under `.codex/workflow/agents/`
- short final handoff with changed/read paths, verification, blockers, and
  SOLID concerns

External sessions may delegate further inside their own session if doing so
keeps their lane smaller, but they remain responsible for their lane contract
and must not let child agents edit outside the parent lane.

## Active Lanes

1. `canary_observer`
   - Owns: no source files.
   - Reads: current canary script and focused search output.
   - Delivers: periodic boundary-leak summaries and SOLID concerns.

2. `app_catalog_followup`
   - Owns: app catalog conversion and app-related imports only after root grants
     exact files.
   - Goal: keep app catalog domain types out of app-server wire DTOs except at
     the app-server edge.

3. `auth_boundary`
   - Owns: auth API/type files only after root grants exact files.
   - Goal: move `AuthMode` ownership away from app-server protocol.

4. `mcp_elicitation_boundary`
   - Owns: elicitation schema/request type extraction only after root grants
     exact files.
   - Goal: remove MCP elicitation type ownership from app-server protocol.

5. `thread_projection_boundary`
   - Owns: thread projection DTO/type extraction only after root grants exact
     files.
   - Goal: remove `ThreadHistoryBuilder` and `TurnStatus` protocol ownership
     from core-facing code.

6. `thread_store_boundary`
   - Owns: thread-store abstraction usage and tests only after root grants exact
     files.
   - Goal: remove concrete `codex-thread-store` use from `codex-core`.

## Static Verification

Run cheap scans after each slice:

```powershell
rg -n "\[workspace\]|Cargo.lock|path = \"\.\.|codex_core|codex_core_api|codex_app_server_protocol|codex_thread_store|codex-core|codex-core-api|codex-app-server-protocol|codex-thread-store" codex-rs/session codex-rs/turn codex-rs/thread codex-rs/core-domain codex-rs/context-domain codex-rs/tools-domain codex-rs/runtime-domain -g "Cargo.toml" -g "*.rs" -g "Cargo.lock"
powershell -ExecutionPolicy Bypass -File .codex/prototypes/check-core-boundaries.ps1
```

Use targeted release checks only after the structural edge is coherent. This
Windows checkout is release-only; avoid debug-profile Cargo builds.

## Current Immediate Order

1. Save this plan and update the handoff because compaction is expected soon.
2. Create `.codex/workflow/agents/` prompts and handoff files.
3. Re-establish one SOLID overseer and at least one boundary scout.
4. Establish current static leak counts.
5. Integrate the next boundary slice from root, starting with the smallest edge
   that removes a direct app-server-protocol/core leak without expanding
   `codex-core`.
6. Update handoff with exact state before compaction or interruption.
