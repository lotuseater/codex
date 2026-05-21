# Current Project Architecture

Date: 2026-05-20

## Scope

This note describes the architecture of the current repository checkout. It is
an internal, repo-derived map of the project as it exists in this tree, not a
product guide or an external user manual.

Sources inspected:

- Root `README.md`, `package.json`, and `codex-cli/package.json`
- `codex-rs/README.md` and `codex-rs/Cargo.toml`
- Crate READMEs for `codex-core`, `codex-protocol`, `codex-app-server`,
  `codex-tools`, `codex-config` loader, `codex-execpolicy`,
  `codex_file_search`, and `memories`
- `docs/install.md`, `docs/contributing.md`, and
  `docs/fork-feature-inventory.md`
- Targeted source inspection of CLI subcommands, app-server protocol modules,
  protocol queue types, and the top-level source directories in `core`, `tui`,
  and `app-server`

## Executive Summary

This repository is the OpenAI Codex CLI monorepo. The main maintained
implementation is the Rust workspace under `codex-rs`, distributed to end users
through packaging surfaces such as the npm package in `codex-cli`, Homebrew, and
release artifacts.

At runtime, Codex is organized around thin user-facing entrypoints and a shared
Rust core. The entrypoints decide how a user, automation script, app client, or
integration server drives Codex; `codex-core` owns the common session and agent
behavior behind those surfaces:

```text
installer / npm wrapper / release artifact
  -> codex native executable
    -> codex-cli command router
      -> interactive TUI, headless exec, app-server, MCP server, or utility commands
        -> codex-core session and agent orchestration
          -> model/backend clients, tool execution, sandboxing, config, auth, state
```

The most important architectural boundary is between surfaces and orchestration:
`codex-cli`, `codex-tui`, `codex-exec`, and `codex-app-server` expose different
ways to drive Codex, while `codex-core` owns the session, turn, model, tool, and
sandbox behavior shared by those surfaces.

The local fork is more than a vanilla CLI checkout. It contains first-class
architecture around context reduction, operation caching, multi-agent state,
memory, self-review, release-only local build workflow, and fork feature
preservation. These features are documented in `docs/fork-feature-inventory.md`
and are represented by dedicated crates such as `context-pack`,
`context-reduction`, `context-ops-impl`, `operation-cache`, `prompt-reducer`,
`agent-graph-store`, `agent-identity`, `agent-policy`, `task-memory`, and
related support crates.

## Repository Layout

The top-level repository is a monorepo with these major regions:

- `codex-rs/`: the primary Rust implementation. This is a Cargo workspace with
  many `codex-*` crates, shared workspace dependency policy, and most runtime
  behavior.
- `codex-cli/`: the npm package `@openai/codex`. It exposes the `codex` binary
  through `bin/codex.js` and packages `bin` plus `vendor` content so JavaScript
  package managers can install the native CLI.
- `docs/`: repo-facing documentation such as installation, contribution
  guidance, and local fork maintenance notes. This document lives here as an
  internal architecture memo.
- `scripts/`: local build, test, release, and maintenance scripts. In this
  checkout, local Rust verification is expected to use the release-oriented
  scripts rather than broad debug-profile Cargo builds.
- `.github/`: workflow and automation definitions for CI, release, labeling,
  and related repository automation.
- `sdk/`, `tools/`, `third_party/`, and Bazel files: supporting generated API
  clients, repo tooling, vendored or external integration points, and alternate
  build/test metadata.

The root `package.json` is not the product package. It is a private
`codex-monorepo` maintenance package with scripts such as Markdown/JSON/JS/YAML
format checks.

## Runtime Entrypoints

The Rust executable is routed by the `codex-cli` crate. Invoking `codex` without
an explicit subcommand starts the interactive TUI. Explicit subcommands fan out
to command-specific surfaces and utilities, including:

- `Exec`: non-interactive/headless execution
- `Review`: review-oriented workflows
- `Login` and `Logout`: account authentication flows
- `Mcp`: MCP configuration and management commands
- `Plugin`: plugin commands
- `McpServer`: experimental MCP server mode
- `AppServer`: app/IDE integration server
- `RemoteControl`: remote-control service management
- `App`: desktop/app integration commands
- `Completion`: shell completion generation
- `Update`: updater command
- `Doctor`: diagnostics
- `Sandbox`: sandbox helper commands
- `Debug`: debug commands
- `Execpolicy`: execution-policy preview commands
- `Apply`: patch application command
- `Resume`: session/thread resume flow

The main user-facing runtime surfaces are:

- `codex-tui`: the default interactive fullscreen terminal UI implemented with
  Ratatui. It renders conversation state, composer controls, status/footer
  information, streaming output, multi-agent views, onboarding, resume picker,
  notifications, and related terminal widgets.
- `codex-exec`: the headless automation surface used for non-interactive tasks.
  It shares core configuration and session behavior but emits machine-oriented
  output instead of running the fullscreen UI.
- `codex-app-server`: a JSON-RPC 2.0 server used by richer clients such as the
  Codex VS Code extension and other app integrations.
- `codex-mcp-server`: an MCP server path that lets Codex expose functionality
  through Model Context Protocol clients.
- Utility commands such as sandbox helpers, `execpolicy`, login/logout, MCP
  management, diagnostics, completion generation, update, patch application, and
  session resume.

## Core Agent And Session Layer

`codex-core` is the business-logic crate. Its README explicitly describes it as
the logic used by the Rust UIs. It sits below the TUI, exec, app-server, and
other command surfaces.

The core source tree is organized into functional areas such as:

- `agent`: agent behavior and turn orchestration
- `session`: session lifecycle and conversation state
- `context` and `context_manager`: context collection, budgeting, and
  preparation for model turns
- `tools`: tool definitions, dispatch, execution events, and integration with
  core runtime state
- `sandboxing`: sandbox policy integration and platform-specific enforcement
  wiring
- `config`: core-facing configuration adaptation
- `apps` and `plugins`: app/plugin integration paths
- `state` and `tasks`: durable state and task-oriented runtime state
- `guardian`: safety or guardrail-related runtime behavior
- `unified_exec`: common execution plumbing used by command/tool paths
- `utils`: shared implementation utilities inside core

The core library deliberately avoids writing directly to stdout or stderr.
User-visible output is expected to flow through the relevant frontend or
protocol abstraction. That keeps `codex-core` usable from multiple surfaces.

The project also has an explicit architectural pressure to keep `codex-core`
from becoming the default home for every new concept. The repository guidance
and `codex-tools` README both prefer extracting shared behavior into narrower
crates when a feature does not need direct access to core runtime state.

## Protocol Boundaries

There are two important protocol layers with different audiences:

- `codex-protocol`: shared Codex session protocol types used around the core
  session boundary. Its README says it contains internal types for
  communication between `codex-core` and `codex-tui`, plus external types used
  with `codex app-server`. Source comments describe a Submission Queue/Event
  Queue pattern for asynchronous client-agent communication.
- `codex-app-server-protocol`: the app-server JSON-RPC API contract for rich
  clients. The v2 protocol is split by resource area, including account, apps,
  attestation, collaboration mode, command execution, config, environment,
  experimental features, feedback, filesystem, hooks, items, MCP, model,
  notifications, permissions, plugins, process, realtime, remote control,
  review, thread data, threads, turns, and Windows sandbox support.

The app-server README describes `codex app-server` as bidirectional JSON-RPC
2.0. It supports local transports for applications, including stdio-oriented
and WebSocket-oriented flows. The app-server protocol is schema-generated for
client use, with v2 as the active API development surface. New app-facing API
surface should be added through app-server v2 rather than through v1 or TUI-only
types.

The intended layering is:

```text
TUI / exec / app-server / MCP surface
  -> codex-protocol or app-server-protocol API types
    -> codex-core operations
      -> events, thread items, tool calls, model calls, state changes
    -> protocol events or responses back to caller
```

This separation lets terminal UI, headless automation, and app/IDE clients all
observe the same underlying session behavior without duplicating the agent
implementation.

## Tools, Command Execution, And Sandboxing

Tool execution is spread across a few deliberately separated crates:

- `codex-tools`: shared tool-related code that does not need to stay coupled to
  `codex-core`. Its README states that adding runtime-state-dependent logic
  there should trigger a boundary review.
- `tool-schema`: schema-oriented tool definitions.
- `shell-command` and `shell-escalation`: shell command execution and escalation
  surfaces.
- `apply-patch`: patch parsing and application.
- `file-search`, `file-system`, and `git-utils`: focused file and Git
  capabilities.
- Core-local `tools` modules: runtime dispatch and integration points that do
  need core session state.

The practical split is that focused crates own reusable tool capabilities,
while `codex-core` owns the session-aware routing, policy checks, and event
emission that make those capabilities usable inside a Codex turn.

Sandbox and policy behavior is also modular:

- `sandboxing`: common sandboxing abstractions.
- `linux-sandbox`, `bwrap`, and vendored bubblewrap support: Linux isolation
  paths.
- `codex-windows-sandbox` in `windows-sandbox-rs`: Windows sandbox support.
- `process-hardening`: process-level hardening support.
- `execpolicy` and `execpolicy-legacy`: command policy and compatibility paths.

The `codex-core` README documents platform-specific sandbox behavior. On macOS,
Codex uses Seatbelt. On Linux, it prefers Landlock/seccomp APIs. On Windows,
there is an elevated helper-based sandbox with policy levels and compatibility
behavior.

## Model, Backend, Auth, And Configuration

Model and backend access are separated from UI and core orchestration through
client/provider crates:

- `codex-client`, `codex-api`, `backend-client`, and
  `codex-backend-openapi-models`: client and generated model/API surfaces.
- `model-provider`, `model-provider-info`, and `models-manager`: model
  provider metadata, selection, and management.
- `responses-api-proxy`: proxy tooling around Responses API flows.
- `aws-auth` and related backend/auth helpers: provider-specific auth support.

Authentication and credentials are handled by dedicated crates such as:

- `login`: login/logout flow implementation used by CLI commands.
- `keyring-store`: secure local credential storage.
- account-related app-server protocol and processor modules.

Configuration is owned by `config` and `config-types`, with core-facing
adaptation where needed. The `codex-config` loader README identifies the loader
as the canonical place for configuration layers, including user config,
CLI/session overrides, managed config, and MDM-managed inputs. The loader also
tracks merge behavior, per-layer descriptions, stable hashing, and key-origin
traversal.

That layering gives Codex a predictable path from user or managed settings to
runtime behavior:

```text
config files / managed config / CLI flags / session overrides
  -> codex-config loader and config types
    -> core-compatible runtime config
      -> model selection, sandbox policy, tool policy, UI/app behavior
```

## App, MCP, Plugin, And External Integration

The repository has several integration surfaces beyond the terminal CLI:

- `app-server`, `app-server-transport`, `app-server-daemon`,
  `app-server-client`, `app-server-protocol`, and `app-server-test-client`
  provide the local app/IDE integration stack.
- `mcp`, `mcp-server`, and `rmcp-client` support Codex acting as an MCP client,
  exposing an MCP server, or using RMCP client plumbing.
- `plugin` plus CLI plugin commands support plugin discovery and management.
- app-server v2 protocol modules expose integration areas such as app listing,
  MCP tool calls, config read/write/list, filesystem operations, hooks, review,
  remote control, thread and turn operations, and Windows sandbox readiness.

The app-server design is the main boundary for rich clients. Instead of linking
directly against the TUI or recreating CLI parsing, clients speak app-server
protocol methods and receive structured thread, turn, item, and notification
data.

## Context, Memory, And Local Fork Features

This checkout includes a significant local fork layer around context efficiency,
memory, and agent orchestration.

Context-related crates include:

- `context-pack`: reusable context package construction.
- `context-reduction`: context compaction and reduction behavior.
- `context-ops-impl`: implementation support for context operations.
- `prompt-reducer`: prompt reduction machinery.
- `repo-context-scout`: repository context discovery.
- `first-moves`: learned first-read/search routing for new tasks.
- `operation-cache`: cache behavior for repeated operations.
- `replacement-shadow`: replacement/shadow behavior used by local fork tooling.

Memory and self-review surfaces include:

- `memories/*`: reusable memory crates and the memory pipeline documentation.
- `task-memory`: task-specific memory behavior.
- `self-review`: review or reflection support.

Multi-agent and identity/state surfaces include:

- `agent-graph-store`
- `agent-identity`
- `agent-policy`
- `external-agent-sessions`
- `external-agent-migration`

`docs/fork-feature-inventory.md` is the durable maintenance checklist for these
local fork capabilities. It calls out features that must be preserved during
upstream merges, including plan-mode UX, token saving and context routing,
operation cache APIs, footer/session limit telemetry, self-review and task
memory, MultiAgentV2, release-only local build workflow, dependency dedupe, and
tool-schema/code-mode decoupling.

Architecturally, these fork features are not just UI affordances. They affect
how tasks are routed, how context is discovered and reduced, how repeated work
is cached, how agents are represented, and how long-running sessions preserve
state. Merge or refactor work that touches their owner paths should treat the
inventory as an architectural contract, not as optional release notes.

## State, Observability, And Runtime Metadata

Several crates provide cross-cutting runtime support:

- `state`, `thread-store`, and core state modules hold durable or resumable
  session/thread data.
- `analytics` records product/runtime event data.
- `otel` provides OpenTelemetry initialization and telemetry plumbing.
- `runtime-metrics-types` defines runtime metric payloads.
- `rollout` and `rollout-trace` support rollout state and traceability.
- `turn-diff` tracks or summarizes turn-level changes.

These crates let Codex persist conversation state, resume work, observe runtime
behavior, and inspect or compare turn outcomes without putting all of that logic
inside a UI crate.

## Packaging, Build, And Verification

Packaging has two main layers:

- The Rust workspace builds the native executable and supporting binaries.
- `codex-cli` publishes the npm-facing package `@openai/codex`, whose `bin`
  entry points at `bin/codex.js` and whose package contents include `bin` and
  `vendor`.

The installation docs describe npm, Homebrew, and source-build paths. The Rust
README emphasizes that the Rust implementation is provided as a standalone
executable for zero-dependency installation.

This Windows checkout has a release-oriented local build policy. Repo
instructions prefer `scripts\build-local-codex.ps1 -Mode FastRelease` for local
binary verification and release-profile test scripts for focused Rust tests.
Broad debug-profile Cargo builds are avoided because they can exhaust local disk
or memory.

For app-server API changes, the repo guidance requires schema regeneration and
protocol tests. For config shape changes, schema regeneration is required. For
Rust code changes, formatting and scoped release-profile tests are expected.
This document is documentation-only, so the relevant local verification is a
Markdown formatter check.

## Extension And Ownership Boundaries

The clearest ownership boundaries in the current tree are:

- Add user-facing command routing in `codex-cli`, but keep reusable behavior in
  lower-level crates.
- Add interactive terminal UI behavior in `codex-tui`, but drive agent behavior
  through protocol/core events rather than duplicating core logic.
- Add headless automation behavior in `codex-exec`, sharing config and core
  session behavior with the TUI.
- Add app/IDE integration behavior through `app-server` and
  `app-server-protocol` v2, not v1.
- Add shared tool logic to `codex-tools` or focused tool crates when it does not
  require core runtime state.
- Add session/agent behavior to `codex-core` only when it genuinely belongs to
  core orchestration; otherwise prefer focused crates.
- Preserve local fork feature owner crates during upstream merges rather than
  hiding boundary problems behind compatibility re-exports.

The main architectural risk is centralization pressure: `codex-core` is the
natural place to add behavior quickly, but the repo explicitly discourages
growing it when a dedicated crate or module would keep ownership cleaner. The
current workspace shows an active effort to extract context, memory, tools,
agent identity, app-server protocol, config, and policy behavior into narrower
owners.

## Practical Development Guidance

- For new command routing, start at `codex-cli`, but keep reusable behavior in a
  narrower crate or in `codex-core` only when it needs shared session state.
- For rich clients, prefer app-server protocol v2 and generated schemas over
  parsing terminal output or linking directly to TUI internals.
- For tool work, separate reusable tool capability from core-local policy,
  routing, and event emission.
- For local fork features, check `docs/fork-feature-inventory.md` before
  changing owner paths so merge-preservation requirements stay visible.
- For this Windows checkout, use the release-oriented scripts documented by the
  repo instead of broad debug-profile Cargo builds.

## High-Level Request Flow

A typical interactive turn follows this shape:

```text
User input in TUI
  -> TUI converts UI action into protocol operation
  -> codex-core receives operation through session queues
  -> core prepares context and configuration for the turn
  -> core calls model/backend clients
  -> model output requests tools, messages, or reasoning updates
  -> core applies tool policy and sandbox policy
  -> tool crates execute shell/file/git/patch/MCP operations as allowed
  -> core emits events and thread items
  -> TUI renders updated conversation, status, diffs, and tool results
```

A typical headless exec flow is similar but replaces TUI rendering with
machine-oriented output:

```text
codex exec arguments
  -> codex-cli routes to codex-exec
  -> exec loads shared config and starts a core session
  -> core runs the requested task
  -> exec emits final result, events, or JSONL-style output depending on mode
```

A typical app integration flow uses app-server:

```text
VS Code extension or other app client
  -> app-server JSON-RPC transport
  -> app-server protocol request processor
  -> codex-core session/thread operation
  -> protocol response or notification
  -> client renders structured app UI
```

## Verified Facts And Inferences

Verified from source and repo docs:

- `codex-rs` is the main Rust implementation and Cargo workspace.
- `codex-cli` is the npm package wrapper for `@openai/codex`.
- `codex-core` is the shared business-logic crate for Rust UIs.
- `codex-protocol` contains shared protocol types and uses an asynchronous
  submission/event queue model.
- `codex app-server` is a bidirectional JSON-RPC 2.0 interface for rich clients.
- The CLI command router exposes TUI-adjacent, exec, app-server, MCP, sandbox,
  policy, auth, plugin, diagnostics, apply, resume, and utility commands.
- The local fork includes durable context, cache, memory, multi-agent, and
  release-build workflow features documented in
  `docs/fork-feature-inventory.md`.

Structural inferences from the inspected crate layout:

- The project is intentionally moving shared behavior out of large orchestration
  crates into smaller ownership crates.
- `codex-core` remains the central runtime coordinator, but app-server,
  protocol, config, tools, sandbox, memory, and context crates are designed to
  keep responsibilities separable.
- The app-server and protocol crates are the safest extension point for rich
  clients because they expose structured data without coupling clients to the
  terminal UI.
