# Codex Rust Crate Decoupling Candidates

This note ranks crate-split candidates that keep one deployable `codex.exe`.
The goal is to remove broad dependency edges or shrink high-churn downstream
crates, not to introduce another runtime binary.

## Ranking Criteria

- Prefer cuts where a large crate is used for one small API.
- Prefer cuts that reduce rebuilds after common local edits such as tool schema,
  DAB, TUI labels, model presets, or protocol event formatting.
- Avoid splits that only move files while every downstream crate still rebuilds
  with the same dependency surface.
- Keep verification narrow and release-profile only on this Windows checkout.

## Candidates

### 1. Tool Input Schema Parser

- Current edge: `codex-app-server -> codex-tools`.
- Evidence: app-server uses `codex_tools::parse_tool_input_schema` once in
  `app-server/src/request_processors/thread_processor.rs`.
- Split: move `JsonSchema`, `JsonSchemaType`, `AdditionalProperties`, and
  `parse_tool_input_schema` from `codex-tools` to a small crate such as
  `codex-tool-schema`; re-export from `codex-tools` for compatibility.
- Benefit: changes in tool descriptions, DAB tools, agent tools, and registry
  planning stop forcing `codex-app-server` to rebuild through `codex-tools`.
- Risk: low. The parser is pure serde/serde_json code with focused tests.
- Verification:
  - `cargo test -p codex-tool-schema --release -j 1`
  - a focused app-server dynamic-tool validation test/filter
  - `rg "codex_tools::|codex-tools" codex-rs/app-server`

### 2. Shell Words / Command Summary Surface

- Current edges:
  - `codex-app-server -> codex-shell-command` only for `shlex_join`.
  - `codex-app-server-protocol -> codex-shell-command` for item builders.
  - `codex-memories-read -> codex-shell-command` for usage summaries.
- Split:
  - First, extract `shlex_join` into a tiny `codex-shell-words` crate and use it
    from app-server and shell-command.
  - Then consider splitting command summary parsing from command safety into a
    `codex-command-summary` crate so protocol code does not pull safety/parser
    surfaces it does not need.
- Benefit: avoids dragging full shell-command dependencies, including heavier
  parsing/safety modules, into app-server surfaces that only need display text.
- Risk: medium. Command rendering appears in protocol/history output and should
  be snapshot or fixture verified.
- Verification:
  - command summary/shell-word unit tests
  - app-server protocol item-builder tests
  - `rg "codex_shell_command::" codex-rs/app-server codex-rs/app-server-protocol`

### 3. Model Presets And Collaboration Presets

- Current edges:
  - TUI uses `codex-models-manager` only for migration config-key constants and
    `builtin_collaboration_mode_presets`.
  - app-server uses `builtin_collaboration_mode_presets` and `RefreshStrategy`.
- Split:
  - Move collaboration preset construction next to
    `codex-collaboration-mode-templates`, or into a new lightweight presets
    crate if adding protocol/template dependencies there is undesirable.
  - Move the two model migration config-key constants to a tiny shared constants
    crate or to the config crate.
  - Longer slice: move `RefreshStrategy` to a lightweight model-catalog types
    crate used by core, app-server, CLI, and models-manager.
- Benefit: TUI can drop `codex-models-manager`; app-server can drop it once
  `RefreshStrategy` is moved. Model catalog/cache churn then has less impact on
  TUI/app-server rebuilds.
- Risk: medium. The APIs are small, but they touch model picker and
  collaboration mode defaults.
- Verification:
  - collaboration mode preset tests
  - TUI tests around model migration prompt visibility, if present
  - `rg "codex_models_manager::" codex-rs/tui codex-rs/app-server`

### 4. ChatGPT Connectors And Workspace Settings

- Current edge: TUI and app-server depend on `codex-chatgpt` for connectors and
  workspace settings, while CLI also uses the `apply` command surface.
- Split: move connector listing/cache helpers and workspace-settings helpers out
  of `codex-chatgpt` into narrower crates such as `codex-chatgpt-connectors` and
  `codex-workspace-settings`; leave `apply_command` in `codex-chatgpt`.
- Benefit: connector/workspace settings can evolve without tying app-server/TUI
  to CLI apply-command dependencies and vice versa.
- Risk: medium. Connector behavior is user-facing and uses cache/auth/plugin
  paths.
- Verification:
  - connector merge/cache tests
  - app-server app/plugin listing focused tests
  - TUI connector mention tests

### 5. Pure App-Server Protocol Versus Event Builders

- Current edge: `codex-app-server-protocol` exports wire types and also exports
  event/item builder functions that depend on command parsing.
- Split: keep pure v1/v2 protocol wire types and schema generation in
  `codex-app-server-protocol`; move `item_builders`, `event_mapping`, and
  thread-history projection helpers into a new event-mapping crate.
- Benefit: crates that only need wire types, such as `codex-tools`, do not need
  event-conversion dependencies. This is likely a meaningful dependency cut
  because many crates depend on `codex-app-server-protocol`.
- Risk: high. Exports are public inside this workspace and app-server history
  replay depends on exact conversion behavior.
- Verification:
  - app-server-protocol schema generation unchanged
  - event mapping/thread history tests in the new crate
  - app-server replay/history focused tests

### 6. Core Utility Leakage

- Current examples:
  - `codex-cloud-requirements -> codex-core` for `util::backoff`.
  - `codex-utils-oss -> codex-core` for `config::Config`.
  - `codex-utils-sandbox-summary -> codex-core` for config summary input.
- Split:
  - Move generic retry/backoff helpers to `codex-async-utils`.
  - Prefer `codex-config` or protocol/config types for config-only utility
    inputs when a helper does not need the full core runtime.
- Benefit: reduces low-level utility crates depending on the largest crate.
- Risk: medium. These are smaller edges, but they help resist further
  `codex-core` growth.
- Verification:
  - focused tests for each utility crate
  - `rg "codex_core::" codex-rs/utils codex-rs/cloud-requirements`

### 7. Tools Depending On App-Server Protocol

- Current edge: `codex-tools -> codex-app-server-protocol` for `AppInfo` and
  MCP elicitation request types used by tool discovery/plugin install helpers.
- Split options:
  - Extract shared app/tool metadata types into a smaller protocol-adjacent
    crate and re-export from app-server-protocol.
  - Or move plugin-install tool construction into a narrower crate used only by
    callers that need app-server elicitation.
- Benefit: tool definition work would touch less protocol/schema code.
- Risk: medium-high because these are wire-visible types and schema generation
  must stay stable.
- Verification:
  - app-server protocol schema fixtures unchanged
  - tool registry plan tests
  - plugin install elicitation tests

### 8. App-Server Internal Slices

- Current large app-server files include `thread_processor.rs`,
  `bespoke_event_handling.rs`, `plugins.rs`, and `message_processor.rs`.
- Split: after dependency-edge cuts, consider moving stable app-server domains
  into crates such as event mapping, config service, or thread request
  processing. Keep one `codex.exe`; the app-server crate remains the orchestration
  crate.
- Benefit: if the main app-server crate becomes thin, editing one app-server
  domain recompiles that domain plus a smaller orchestrator instead of the
  entire 34k-LoC crate.
- Risk: high. Most processors share app-server state, outgoing messaging,
  auth/config state, and lifecycle semantics.
- Verification:
  - focused app-server request-processor tests
  - full app-server protocol schema generation when wire types move
  - one final `codex.exe` release build/smoke

### 9. Cloud Tasks Depending On TUI/Core

- Current edge: `codex-cloud-tasks` depends on both `codex-tui` and
  `codex-core`.
- Evidence: cloud tasks uses `codex_tui::ComposerInput`,
  `codex_tui::ComposerAction`, and `codex_tui::render_markdown_text`; it also
  uses `codex_core::config::Config`.
- Split:
  - Move composer input/action primitives and markdown rendering helpers that
    are not inherently TUI-app orchestration into smaller UI utility crates.
  - Move config-only helpers to `codex-config` or a lightweight config view if
    the full core runtime is not needed.
- Benefit: cloud-task changes stop waiting behind the full TUI crate for small
  shared input/rendering utilities, and the final CLI build has one less large
  dependency edge in that branch.
- Risk: medium-high. The cloud tasks UI is interactive, and TUI utility moves
  need snapshot or live terminal smoke coverage.
- Verification:
  - cloud-task UI/helper unit tests
  - focused TUI tests for the moved composer/rendering primitives
  - `rg "codex_tui::|codex_core::" codex-rs/cloud-tasks/src`

## Recommended Order

1. Implement the tool-schema split first; it is low-risk and directly helps the
   DAB/tool work that triggered the slow app-server rebuild.
2. Remove the direct app-server dependency on `codex-shell-command` by extracting
   `codex-shell-words`.
3. Move collaboration/model preset constants out of `codex-models-manager`.
4. Reassess compile behavior before attempting the larger protocol/event-mapping,
   cloud-tasks/TUI, or app-server processor crate splits.
