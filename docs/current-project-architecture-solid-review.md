# Current Project Architecture SOLID Review

Date: 2026-05-20

## Scope

This memo reviews the current Codex checkout from an architecture perspective,
with emphasis on SOLID, clean architecture, dependency direction, module
ownership, and long-term maintainability.

This is not a bug report against one feature. It is a structural review of the
architecture described in `docs/current-project-architecture.md` and checked
against source files and crate metadata in this checkout.

## Sources Inspected

- `docs/current-project-architecture.md`
- `codex-rs/Cargo.toml`
- `codex-rs/core/README.md`
- `codex-rs/protocol/README.md`
- `codex-rs/app-server/README.md`
- `codex-rs/tools/README.md`
- `codex-rs/config/src/loader/README.md`
- `codex-rs/memories/README.md`
- `docs/fork-feature-inventory.md`
- `codex-rs/core/Cargo.toml`
- `codex-rs/core-api/src/lib.rs`
- `codex-rs/core-api/Cargo.toml`
- `codex-rs/core/src/lib.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/tools/router.rs`
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/protocol/src/protocol.rs`
- `codex-rs/app-server/src/lib.rs`
- `codex-rs/app-server-client/src/lib.rs`
- `codex-rs/cli/src/main.rs`
- `codex-rs/tui/src/app.rs`
- `codex-rs/tui/src/chatwidget.rs`
- `codex-rs/tui/src/bottom_pane/mod.rs`
- `codex-rs/tui/src/bottom_pane/chat_composer.rs`

## Executive Findings

The architecture has a sound top-level intent: multiple user surfaces route into
a shared Rust runtime, protocol types separate event transport from UI code, and
some responsibilities have already been extracted into narrower crates such as
`codex-tools`, `codex-config`, `codex-execpolicy`, and app-server protocol
modules.

The main weakness is that the intended boundaries are not fully enforced.
`codex-core` is both the central business logic crate and a dependency hub with
109 direct dependencies, including 69 local Codex crates. It also imports
`codex_app_server_protocol` from many core modules. That creates an inward
dependency leak: core business logic knows about an outer integration API that
should normally adapt to core, not the other way around.

The second major risk is size and orchestration concentration. Several files are
well beyond the local architecture guidance for module size:

- `codex-rs/tui/src/bottom_pane/chat_composer.rs`: 9,720 lines
- `codex-rs/protocol/src/protocol.rs`: 4,592 lines
- `codex-rs/core/src/config/mod.rs`: 3,311 lines
- `codex-rs/core/src/session/mod.rs`: 3,244 lines
- `codex-rs/cli/src/main.rs`: 2,953 lines
- `codex-rs/core/src/session/turn.rs`: 2,620 lines
- `codex-rs/tui/src/bottom_pane/mod.rs`: 2,650 lines
- `codex-rs/app-server-client/src/lib.rs`: 2,120 lines
- `codex-rs/core/src/client.rs`: 2,124 lines
- `codex-rs/core/src/mcp_tool_call.rs`: 2,000 lines
- `codex-rs/tui/src/chatwidget.rs`: 1,827 lines
- `codex-rs/tui/src/app.rs`: 1,225 lines
- `codex-rs/app-server/src/lib.rs`: 1,059 lines

Large files are not automatically wrong, but here they align with high-change
orchestration areas. That increases merge conflict risk, hides responsibility
boundaries, and makes local reasoning harder.

## Findings

### 1. Core Depends On App-Server Protocol Types

Severity: High

Evidence:

- `codex-rs/core/Cargo.toml` depends on `codex-app-server-protocol`.
- `rg` finds app-server protocol imports across core modules, including
  `agents_md.rs`, `client.rs`, `apps/render.rs`, `agent/role.rs`,
  `compact_remote.rs`, `connectors.rs`, `context/apps_instructions.rs`,
  config tests, and `tools/handlers/request_plugin_install.rs`.
- `codex-rs/core/README.md` describes `codex-core` as the business logic used by
  Rust UIs.
- `codex-rs/app-server/README.md` describes app-server as a JSON-RPC integration
  surface for rich clients.

Why this is wrong architecturally:

- It violates Dependency Inversion. Core policy should depend on core-owned
  abstractions, not on app-server wire types.
- It weakens the app-server boundary. App-server should translate between
  JSON-RPC protocol models and core models.
- It makes protocol evolution riskier. Changes to app-server API names, feature
  gates, or TypeScript export details can ripple into core runtime logic.

Recommended improvement:

- Introduce or strengthen core-owned domain types for concepts currently reused
  from app-server protocol, especially auth mode, config layer source, app info,
  branding, metadata, and dynamic tool response shapes.
- Move conversions to the edge:
  - app-server maps app-server protocol types to core types;
  - TUI maps app-server client protocol types to UI types;
  - core avoids app-server protocol except in narrow compatibility shims targeted
    for removal.
- Add a dependency-direction check that fails if `codex-core` imports
  `codex_app_server_protocol` outside an explicitly named adapter module.

### 2. `codex-core` Is Still A Dependency Hub

Severity: High

Evidence:

- `codex-rs/core/Cargo.toml` has 109 direct dependencies, 69 of them local
  Codex crates.
- The workspace guidance explicitly says to resist adding code to `codex-core`.
- `codex-core` currently coordinates sessions, turns, tools, model clients, MCP,
  config, auth, rollouts, exec policy, sandboxing, agent control, context
  reduction, and local fork features.

Why this is wrong architecturally:

- It stresses the Single Responsibility Principle. Core has become a runtime
  application kernel plus a coordination layer plus a compatibility layer.
- It makes the crate difficult to test in isolation because many unrelated
  responsibilities enter the same dependency graph.
- It creates "gravity": new functionality is likely to land in core because the
  necessary types and services are already there.

Recommended improvement:

- Treat `codex-core` as an orchestration facade and actively move cohesive
  domains outward.
- Prioritize extractions with clear ownership and low API ambiguity:
  - session lifecycle and turn state machines;
  - tool execution policy and tool registry assembly;
  - app metadata and connector domain types;
  - model/backend client abstraction;
  - core-owned config state and config provenance types.
- Keep compatibility re-exports short-lived and documented with owners.

### 3. `codex-core-api` Is A Facade, Not Yet A Boundary

Severity: Medium-High

Evidence:

- `codex-rs/core-api/src/lib.rs` is a re-export facade for non-core APIs.
- `codex-rs/core-api/Cargo.toml` no longer depends on `codex-core`, which is a
  useful step toward a real boundary.
- Extension crates such as `codex-rs/ext/memories` and
  `codex-rs/ext/guardian` depend directly on `codex-core`.

Why this is wrong architecturally:

- A facade that re-exports non-core API types can simplify imports, but it does
  not by itself move extension-facing contracts behind that boundary.
- Extension crates remain coupled to the full core crate, so they still inherit
  core's large dependency graph and release surface.
- It can create a false sense of decoupling: call sites look API-oriented while
  still being linked to the concrete runtime.

Recommended improvement:

- Decide whether `codex-core-api` is only a convenience facade or a real
  dependency boundary.
- If it is intended as a boundary, move stable extension-facing contracts into
  it or into narrower API crates.
- Make extension crates depend on those boundary crates instead of directly
  depending on `codex-core`.

### 4. Session And Turn Logic Mix Too Many Reasons To Change

Severity: High

Evidence:

- `codex-rs/core/src/session/mod.rs` is 3,244 lines.
- `codex-rs/core/src/session/turn.rs` is 2,620 lines.
- `turn.rs` includes turn execution, pre-sampling compaction, auto-compaction,
  sampling request handling, plan-mode behavior, event emission, in-flight
  draining, and response item handling.
- `session/mod.rs` also interacts with approvals, guardian review, task
  submission, spawned agents, events, rollout, and runtime services.

Why this is wrong architecturally:

- It violates Single Responsibility. A turn runner should not also own broad
  policy for compaction, planning, event projection, and tool lifecycle.
- It makes Open/Closed harder. Adding a new turn phase or policy tends to edit
  the same central file.
- It increases defect risk because local changes can alter unrelated runtime
  phases.

Recommended improvement:

- Split the session runtime around explicit phase objects:
  - `TurnRunner` or `TurnStateMachine` for sequencing;
  - `SamplingPipeline` for request construction and response streaming;
  - `CompactionPolicy` for pre-sampling and automatic compaction;
  - `PlanModeProjector` for plan-mode event transformation;
  - `TurnEventSink` for outbound event emission.
- Make the session module wire those pieces together instead of implementing
  each policy inline.
- Add focused tests per phase before moving behavior, then keep integration
  tests for end-to-end turn flow.

### 5. Tool Routing Is Better Than A Monolith, But Still Too Centralized

Severity: Medium-High

Evidence:

- `codex-rs/tools/README.md` says `codex-tools` is intended to become the home
  for shared tool-related code that does not need runtime state from
  `codex-core`.
- `codex-rs/core/src/tools/router.rs` builds a router from turn context and
  session state.
- `codex-rs/core/src/tools/spec_plan.rs` imports and registers many concrete
  handlers, including apply patch, exec command, first moves, MCP resource
  listing, workflow batch, dynamic tools, update plan, user input, and
  multi-agent handlers.

Why this is wrong architecturally:

- The registry is partially data-driven, but concrete handler assembly still
  concentrates extension logic in core.
- New tools tend to require editing central registration code, which weakens
  Open/Closed.
- Runtime context requirements are implicit. It is not always clear which tools
  need session state, turn state, config state, MCP state, or external services.

Recommended improvement:

- Define a small tool-contribution interface with explicit capability
  requirements.
- Let feature-owned modules or crates contribute tool specs and handlers through
  a registry builder.
- Keep `codex-core` responsible for injecting runtime capabilities, not for
  naming every concrete tool.
- Use `codex-tools` for pure schemas, tool metadata, registry planning, and
  capability descriptions.

### 6. The Public Protocol File Combines Too Many Protocol Families

Severity: Medium-High

Evidence:

- `codex-rs/protocol/src/protocol.rs` is 4,592 lines.
- It defines the submission queue and event queue concepts, including `Op` and
  `EventMsg`.
- `codex-rs/protocol/README.md` says this crate contains internal communication
  types between core and TUI plus external types used with app-server, while
  avoiding material business logic.

Why this is wrong architecturally:

- Large union-style protocol files become change magnets.
- Internal session protocol and external integration protocol evolve for
  different reasons.
- Interface Segregation is strained: consumers must depend on a broad type
  module even when they only need one protocol family.

Recommended improvement:

- Split protocol by stable families rather than by one catch-all file:
  - submission operations;
  - event messages;
  - tools;
  - approvals and permissions;
  - agent control;
  - session metadata;
  - streaming deltas;
  - collaboration events.
- Keep top-level re-exports for compatibility while moving definitions behind
  family modules.
- Add compile-level ownership boundaries so external protocol changes do not
  force internal session protocol churn.

### 7. CLI Entrypoint Is A Command Switchboard

Severity: Medium

Evidence:

- `codex-rs/cli/src/main.rs` is 2,953 lines.
- It owns `Subcommand` and command dispatch for TUI, exec, review, MCP server,
  app-server, login/logout, sandbox helpers, debug commands, schema generation,
  apply, update, features, and other utilities.

Why this is wrong architecturally:

- It mixes parsing, command policy, feature wiring, and command execution.
- Adding a command frequently means editing the central CLI file.
- It weakens Single Responsibility and Open/Closed at the entrypoint.

Recommended improvement:

- Move each command family into a module or crate-local command object with:
  - parser type;
  - validation;
  - execution;
  - strict-config support policy;
  - telemetry command name.
- Keep `main.rs` as a thin parser and dispatcher.
- Use a small `CommandRunner` trait or enum-backed command table if Rust object
  safety would make a trait heavier than needed.

### 8. App-Server Client Facade Is Large And Blurs Transport With Domain Flow

Severity: Medium

Evidence:

- `codex-rs/app-server-client/src/lib.rs` is 2,120 lines.
- It is documented as a shared in-process app-server client facade for CLI
  surfaces.
- It imports app-server protocol types and owns client behavior around thread,
  turn, notification, and remote app-server flows.

Why this is wrong architecturally:

- A facade is useful, but a 2,120-line facade risks becoming a second
  orchestration layer rather than a thin client abstraction.
- Transport behavior, request construction, notification filtering, and surface
  convenience helpers have different reasons to change.
- This can make TUI and CLI surfaces depend on a broad client interface when
  they only need narrow thread or turn operations.

Recommended improvement:

- Split the client facade into smaller interfaces:
  - transport;
  - request/response RPC client;
  - thread service;
  - turn service;
  - notification stream;
  - in-process bootstrap adapter.
- Let TUI and CLI depend on the smallest service needed for each flow.

### 9. TUI Components Have Extreme File And Responsibility Size

Severity: Medium-High

Evidence:

- `codex-rs/tui/src/bottom_pane/chat_composer.rs` is 9,720 lines.
- `codex-rs/tui/src/bottom_pane/mod.rs` is 2,650 lines.
- `codex-rs/tui/src/chatwidget.rs` is 1,827 lines.
- `codex-rs/tui/src/app.rs` is 1,225 lines.
- Local repo guidance already names these as high-touch files and recommends
  new modules instead of growing them.

Why this is wrong architecturally:

- It violates Single Responsibility. UI state, rendering, input handling,
  command composition, async runtime interaction, and workflow-specific behavior
  are difficult to isolate.
- It makes snapshot changes harder to review because rendering and behavior are
  close together.
- It raises merge conflict risk in a fork with active local features.

Recommended improvement:

- Split `chat_composer.rs` first because it has the largest blast radius.
- Extract behavior in this order:
  - input buffer and cursor state;
  - slash-command and mention completion state;
  - paste, image, and file attachment handling;
  - mode controls and feature toggles;
  - render-only view models;
  - async side-effect adapters.
- Keep snapshot tests next to the components they validate.
- Use `codex-rs/tui/styles.md` and existing style helpers to avoid visual churn
  during extraction.

### 10. Config Has A Good Loader Boundary But Core Config Remains Heavy

Severity: Medium

Evidence:

- `codex-rs/config/src/loader/README.md` defines the config loader as the
  canonical place to load and describe config layers.
- `codex-rs/core/src/config/mod.rs` is 3,311 lines.
- Config concepts also appear through core, app-server protocol, CLI overrides,
  and tests.

Why this is wrong architecturally:

- The loader boundary is good, but config state, config shape, provenance,
  validation, defaults, and UI/API projection remain spread across multiple
  layers.
- This can blur Interface Segregation. A caller that only needs runtime config
  may also be exposed to source/provenance or app-server-facing concepts.

Recommended improvement:

- Separate config concerns into narrower ownership zones:
  - config file schema and serde shape;
  - resolved runtime config;
  - config provenance and layer metadata;
  - config update/write API;
  - app-server projection types.
- Keep the loader as the composition boundary, but prevent app-server protocol
  details from becoming shared core config vocabulary.

### 11. Local Fork Features Depend On High-Touch Core Paths

Severity: Medium

Evidence:

- `docs/fork-feature-inventory.md` lists local fork features and merge-time
  health checks.
- It explicitly calls out merge-time risk around compaction, sampling, and
  history mapping.
- Several local features touch core session, TUI, operation cache, multi-agent
  tools, and task memory paths.

Why this is wrong architecturally:

- Fork-specific features increase pressure on the same central modules already
  used by upstream behavior.
- Merge conflicts can silently weaken behavior when feature ownership is not
  isolated.
- It raises the cost of broad refactors because local and upstream concerns are
  interleaved.

Recommended improvement:

- Keep each durable fork feature behind a feature-owned module, service, or
  extension boundary.
- Add feature-family canaries for the owner paths listed in
  `docs/fork-feature-inventory.md`.
- Prefer extension points over direct edits to session, turn, TUI, or protocol
  switchboards.

## SOLID Mapping

- Single Responsibility: weakest in session/turn runtime, CLI routing, protocol
  definitions, core config, app-server client facade, and TUI composition files.
- Open/Closed: weak where new commands, tools, app-server mappings, or turn
  policies require editing central files.
- Liskov Substitution: not the dominant issue from this review. The main risk is
  broad concrete coupling rather than broken subtype contracts.
- Interface Segregation: strained by large protocol surfaces and broad config,
  session, app-server client, and UI modules.
- Dependency Inversion: weakest where `codex-core` depends on app-server
  protocol types and where concrete tool handlers are registered centrally.

## Recommended Refactoring Roadmap

### Phase 1: Protect Dependency Direction

- Create core-owned domain types for concepts currently imported from
  `codex_app_server_protocol`.
- Move app-server protocol conversions into app-server, TUI client, or explicit
  adapter modules.
- Decide whether `codex-core-api` is a true stable boundary. If it is, move
  extension-facing contracts there or into narrower API crates and migrate
  extension crates away from direct `codex-core` dependencies.
- Add a CI or local lint check preventing new `codex_app_server_protocol` imports
  from `codex-core`.

Acceptance criteria:

- `rg "codex_app_server_protocol" codex-rs/core/src` returns only approved
  temporary adapter paths or nothing.
- App-server protocol schema generation still passes.
- Existing core tests for config, auth, apps, tools, and dynamic tool responses
  still pass.

### Phase 2: Split Session And Turn Policy

- Extract compaction policy, sampling pipeline, plan-mode event projection, and
  event emission from `session/turn.rs`.
- Keep `session/mod.rs` as lifecycle orchestration, not policy implementation.
- Add focused tests around each extracted policy before moving code.

Acceptance criteria:

- `session/mod.rs` and `session/turn.rs` shrink materially without behavior
  loss.
- Core turn tests and focused compaction/sampling tests pass under the local
  release test lane.

### Phase 3: Modularize Tool Contributions

- Define a tool contribution API that separates pure metadata from runtime
  capability injection.
- Move concrete tool family registration out of a central switchboard.
- Keep `codex-tools` responsible for pure metadata and registry planning.

Acceptance criteria:

- Adding a tool family does not require editing a central list unless the tool
  genuinely introduces a new runtime capability.
- Existing tool router tests and multi-agent tool tests pass.

### Phase 4: Split Protocol Families

- Move `Op`, `EventMsg`, tool-related event types, agent-control types, approval
  types, and session metadata into family modules.
- Preserve top-level re-exports until downstream code is migrated.

Acceptance criteria:

- Public API compatibility remains stable for current callers.
- App-server protocol and TUI continue compiling against re-exports.
- New protocol modules have clear ownership and minimal cross-imports.

### Phase 5: Reduce TUI, CLI, And Client Switchboards

- Split `chat_composer.rs` into state, completion, attachments, modes,
  rendering, and side-effect adapters.
- Move CLI command families out of `main.rs`.
- Split `app-server-client/src/lib.rs` into transport, RPC, thread, turn,
  notification, and bootstrap modules.
- Keep parser, client facade, and top-level UI shells thin.

Acceptance criteria:

- TUI snapshot tests cover intentional visual changes.
- Command tests still cover app-server schema generation, exec, login, MCP, and
  sandbox paths.
- App-server client tests cover thread/turn and notification behavior after the
  split.

## Verification Guidance

Because this review recommends architecture changes rather than making Rust
changes, no Rust test was required for this memo. For future implementation
slices:

- Run `just fmt` in `codex-rs` after Rust edits.
- Run the package-specific release test script for the changed crate, for
  example `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core`.
- For TUI visual changes, update and review `insta` snapshots.
- For app-server protocol changes, run `just write-app-server-schema` and
  `cargo test -p codex-app-server-protocol` through the local release-safe lane
  expected by this checkout.
- For config schema changes, run `just write-config-schema`.
- For dependency changes, run `just bazel-lock-update` and
  `just bazel-lock-check`.

## Bottom Line

The project is architecturally workable, but the center is too heavy. The best
improvement is not a broad rewrite. The highest-value path is to enforce
dependency direction first, then gradually turn the largest switchboards into
thin orchestration layers backed by feature-owned modules and core-owned domain
types.

## Addendum: Core To App-Server Protocol Boundary

Date: May 20, 2026.

### Sources Inspected

- `codex-rs/core/Cargo.toml`
- Focused search for `codex_app_server_protocol` under `codex-rs/core/src`
- `codex-rs/config-types/src/lib.rs`
- `codex-rs/app-server-protocol/src/protocol/common.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/apps.rs`
- `codex-rs/app-server-protocol/src/protocol/thread_history.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/turn.rs`

### Finding

`codex-core` currently depends directly on `codex-app-server-protocol`, and the
usage is broader than a single wire adapter boundary. The imports cover config
provenance, auth mode, app catalog metadata, MCP elicitation payloads, and
thread/turn projection types. This is an inward dependency leak: the central
runtime crate knows about an outer app-server protocol crate that should mostly
adapt domain state into a wire schema.

The imported types fall into different ownership classes:

- `ConfigLayerSource` is already defined in `codex-config-types`, so core should
  import it from that domain/config crate instead of through app-server protocol.
- `AuthMode` represents login/provider state and is not inherently an
  app-server wire concept. It should live in a shared auth/config/domain owner,
  with protocol conversion or re-export kept at the boundary.
- `AppInfo`, `AppMetadata`, and `AppBranding` describe connector/app catalog
  metadata that core uses for connector discovery, rendering, and instructions.
  App-server protocol is not the right owner if core needs these values before
  app-server projection.
- MCP elicitation request/schema types are closer to an MCP or protocol boundary
  and should not be moved as a drive-by cleanup.
- `ThreadHistoryBuilder` and `TurnStatus` look like app-server/thread projection
  concerns and need a separate boundary review before changing ownership.

### Recommendation

The smallest high-value slice is to remove the config provenance dependency
first:

1. Replace core imports of `codex_app_server_protocol::ConfigLayerSource` with
   the existing `codex_config_types::ConfigLayerSource` owner, or with the
   established local re-export if one is already preferred.
2. Verify no `codex-core` source still reaches into app-server protocol for
   config-layer provenance.
3. Run
   `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core`.
   If the config schema output changes, also run `just write-config-schema`.

The next coherent slice is to move app catalog metadata out of
`codex-app-server-protocol` into connector/app-domain ownership, likely
`codex-connectors` if that crate already owns the discovery/cache concepts, or a
new narrow crate if that keeps dependency direction cleaner. The app-server
protocol crate should then become an adapter/schema layer that converts from the
shared domain type rather than owning the domain type.

Do not start this cleanup with MCP elicitation or thread projection types. Those
types have more wire-shape and projection coupling, so moving them first would
mix a dependency-direction fix with API-boundary redesign.
