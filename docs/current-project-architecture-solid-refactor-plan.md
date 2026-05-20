# Current Project SOLID Refactor Plan

Date: 2026-05-20

## Summary

Refactor the current `codex-core` dependency hub into many small, responsibility-owned crates. The rule is strict: domain crates depend on abstractions and DTO crates only. Runtime/application crates may assemble concrete implementations, but no core/session/turn/domain crate may depend directly or transitively on concrete stores, app-server protocol, UI surfaces, or `codex-core-api`.

Compilation may be temporarily broken during the refactor. Keep each slice coherent and committed, and use cheap boundary canaries before expensive checks. Do not refresh Bazel lockfiles or run broad verification until the architecture stops moving.

## Target Crate Folders

Create responsibility folders under `codex-rs/` so related one-purpose crates are grouped without becoming monoliths:

- `core-domain/`: core-owned DTOs and semantic types that used to leak in through app-server protocol or `codex-core-api`.
- `session/`: session-facing one-responsibility crates.
- `turn/`: turn loop, turn state, turn policy, and turn event crates.
- `thread/`: thread identity, handle, manager ports, store API, and concrete store crates.
- `tools-domain/`: tool registry/route abstractions and tool execution ports.
- `context-domain/`: context budget, compaction policy, prompt context, and history abstractions.
- `runtime-domain/`: runtime service ports for auth, backend/model, telemetry, state DB, MCP, skills, hooks, and environment/runtime handles.
- `adapters/`: app-server/TUI/MCP/CLI adapters that translate wire/UI/concrete types into domain abstractions.

Keep crates small enough that each has one reason to change. Prefer a new crate over expanding `codex-core` when a type or policy can stand alone.

## Dependency Rules

- `*-api`, `*-types`, `*-ports`, and `*-policy` crates must not depend on concrete implementation crates.
- Domain/API crates must not depend on `codex-core`, `codex-core-api`, `codex-app-server-protocol`, `codex-app-server`, `codex-tui`, `codex-mcp-server`, `codex-thread-store`, or concrete runtime/store crates.
- Concrete crates may depend on their API crates and low-level infrastructure crates, but must not be re-exported from API crates.
- `codex-core` becomes an orchestrator crate only after extraction: it may depend on domain APIs/ports and call factories handed in from outer surfaces.
- `app-server`, `mcp-server`, CLI, TUI, and sample binaries own assembly of concrete stores, live-thread factories, app-server protocol conversions, and runtime adapters.
- Compatibility re-exports are not allowed as a way to keep old imports compiling. Move callers to the new owning crate.
- Boundary checks must include transitive dependency inspection, not only direct import greps.

## Phase 1: Enforce Boundaries First

- Add a repo-local boundary canary script that fails if protected crates have direct or transitive dependencies on forbidden concrete crates.
- Seed the canary with the current desired graph:
  - `codex-thread-store-api` is API-only.
  - `codex-core` must not depend on `codex-thread-store`.
  - new session/turn/thread/context/tool/runtime domain crates must not depend on `codex-core-api` or app-server protocol.
- Keep the canary cheap: static manifest parsing plus `cargo metadata --no-deps` or equivalent JSON inspection is enough for this stage.
- Commit the plan and canary separately before moving code.

## Phase 2: Thread Package Split

Move current thread work into folder-owned crates:

- `thread/thread-api`: `ThreadId`-adjacent DTOs, thread metadata, thread page/turn page params and results.
- `thread/thread-handle-api`: `LiveThreadHandle`, `LiveThreadFactory`, object-safe futures, unsupported handle/factory.
- `thread/thread-store-api`: `ThreadStore`, store errors/results, store params and unsupported store.
- `thread/thread-store-local`: local concrete store and local live writer.
- `thread/thread-store-memory`: in-memory concrete test/development store.
- `thread/thread-store-factory`: concrete factory selection for outer application assembly only.
- `thread/thread-manager-api`: thread manager command/result DTOs and ports, without concrete session or store dependency.
- `thread/thread-manager`: orchestration implementation that depends only on session/thread/store abstractions.

Remove remaining `codex-core` references to `LocalThreadStore`, `InMemoryThreadStore`, `StoreLiveThreadFactory`, and removed `thread_store_from_config`. Use API-only unsupported/fake implementations in core tests until integration tests are moved outward.

## Phase 3: Session Package Split

Extract session pieces into one-responsibility crates:

- `session/session-api`: spawn/resume/read DTOs, session IDs, session output/event DTOs, and public session command/result shapes.
- `session/session-state`: session-owned mutable state types and state transitions.
- `session/session-input`: user input command parsing and queue-facing session input DTOs. It may depend on `codex-input-queue` only if that crate remains abstraction-only.
- `session/session-events`: internal session event enum and event sink/source ports.
- `session/session-policy`: approval, sandbox, cancellation, and session-level policy decisions as abstractions.
- `session/session-runtime-api`: ports required to run a session: model client, auth, tool router, state DB, live thread, telemetry, MCP, and environment handles.
- `session/session-runtime`: implementation that wires the runtime ports together; no concrete store or app-server protocol dependency.
- `session/session-factory`: creation/resume factory that receives all dependencies as ports/factories from outer layers.

`codex-core/src/session` should shrink to a thin adapter during migration and then disappear or become a private compatibility-free orchestrator module.

## Phase 4: Turn Package Split

Extract turn logic separately from session:

- `turn/turn-api`: turn input/output DTOs, turn IDs, turn options, and turn result status.
- `turn/turn-state`: turn-local mutable state and state transition helpers.
- `turn/turn-policy`: retry, truncation, context-budget, approval, and tool-call policy decisions.
- `turn/turn-loop-api`: `TurnLoop` and `TurnLoopFactory` ports.
- `turn/turn-loop`: concrete turn loop implementation depending only on ports.
- `turn/turn-events`: turn event DTOs and mapping to session events.
- `turn/turn-tool-bridge`: abstraction that lets turn logic call tools without importing concrete handler registries.

The turn loop must not import session concrete types. Session runtime starts a turn through `TurnLoopFactory` and receives events/results through abstractions.

## Phase 5: Core Domain Type Ownership

Move app-server protocol leakage out of core:

- Create core-owned types for auth mode, config layer source/provenance, app info/branding/metadata, dynamic tool response shape, and notification-neutral event DTOs.
- Move all conversions between core-owned types and app-server protocol types to app-server/TUI-client adapter crates.
- Remove `codex-app-server-protocol` from `codex-core` and from new domain/API crates.
- Reclassify `codex-core-api`: either delete facade-only exports as callers migrate, or keep it only as an outer client facade that may not be used by domain crates.

## Phase 6: Tool, Context, And Runtime Ports

Continue reducing central switchboards after session/turn boundaries exist:

- Split tool routing into registry API, handler API, concrete registry assembly, and app-server/dynamic-tool adapters.
- Split context budget and compaction into policy crates consumed by turn/session through traits.
- Move model/backend/auth/config/state/telemetry interactions behind runtime-domain ports.
- Keep feature-owned implementations close to their feature crates; central orchestrators receive trait objects/factories.

## Commit Cadence

Commit logical slices even if the project does not compile:

1. `docs: add SOLID refactor plan`
2. `chore: add architecture boundary canary`
3. `refactor(thread): group thread crates under thread folder`
4. `refactor(thread): separate thread API, handle API, and concrete stores`
5. `refactor(session): add session API and runtime port crates`
6. `refactor(turn): add turn API and loop port crates`
7. `refactor(core): remove concrete thread-store dependencies`
8. `refactor(core): replace app-server protocol imports with core domain types`
9. `refactor(app-server): move protocol/domain conversion to adapters`
10. Continue with tool/context/runtime splits as separate commits.

Stage only files owned by the active slice. Never include unrelated dirty work in these commits.

## Verification

Early stages:

- Run the boundary canary after each crate/dependency slice.
- Use focused `rg` checks for forbidden imports.
- Run `just fmt` after Rust edits.
- Skip `just bazel-lock-update`, `just bazel-lock-check`, and broad Cargo checks until the crate graph settles.

Later stages, once the graph is stable:

- Run focused release checks/tests for changed crates.
- Run app-server schema generation/tests only after protocol wire shapes intentionally change.
- Refresh Bazel lockfiles after dependency movement stabilizes.
- Run the full release suite only after the refactor is compile-oriented again.

## Assumptions

- Temporary compile failures are acceptable while ownership boundaries are being established.
- API crates may define small unsupported/fake implementations for tests and edge adapters.
- New crates should use native Rust trait future shapes or existing object-safe boxed future aliases where object safety is required.
- The first implementation priority is dependency direction, then shrinking `codex-core`, then broad verification.
