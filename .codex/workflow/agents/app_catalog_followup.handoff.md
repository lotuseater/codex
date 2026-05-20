# app_catalog_followup Handoff

Status: complete
Date: 2026-05-20

## Paths Changed

- `.codex/workflow/agents/app_catalog_followup.handoff.md`

## Paths Read

- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/app/app-catalog-types/Cargo.toml`
- `codex-rs/app/app-catalog-types/src/lib.rs`
- `codex-rs/app/app-catalog-api/Cargo.toml`
- `codex-rs/app/app-catalog-api/src/lib.rs`
- `codex-rs/tools/src/request_plugin_install.rs`
- `codex-rs/tools/src/tool_discovery.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/connectors/src/accessible.rs`
- `codex-rs/connectors/src/**` via scoped `rg`
- `codex-rs/app-server/src/app_catalog_protocol.rs` read-only for boundary confirmation
- `codex-rs/app-server-protocol/src/protocol/v2/apps.rs` read-only for wire/domain confirmation
- Manifest references were inspected read-only with scoped `rg`; no forbidden manifest was edited.

## Remaining App Catalog Protocol Leaks

No remaining app catalog data-model leak was found in `core`, `connectors`, or `tools`.

- `codex-rs/connectors/src/**` uses `codex_app_catalog_types::AppInfo` and related catalog types, with no `codex_app_server_protocol` import found.
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs` already consumes `codex_app_catalog_types::AppInfo` plus `codex_tools::{DiscoverableTool, DiscoverableToolAction, DiscoverableToolType}` for install suggestions.
- `codex-rs/tools/src/tool_discovery.rs` uses `codex_app_catalog_types::AppInfo` for connector discovery entries.
- `codex-rs/app-server/src/app_catalog_protocol.rs` is the expected app-server boundary: it converts `codex_app_catalog_types` values into `codex_app_server_protocol` v2 wire structs.

Remaining `codex_app_server_protocol` imports observed in this search are not app catalog model imports:

- `codex-rs/tools/src/request_plugin_install.rs` imports `McpElicitationObjectType`, `McpElicitationSchema`, `McpServerElicitationRequest`, and `McpServerElicitationRequestParams` to construct the MCP elicitation request. This is a protocol dependency in `codex-tools`, but it is transport/schema ownership rather than app catalog domain ownership.
- `codex-rs/core/src/mcp_tool_call.rs`, `codex-rs/core/src/session/mod.rs`, and `codex-rs/core/src/session/tests.rs` still import MCP elicitation protocol types. Those should be handled by an MCP/elicitation boundary lane, not app catalog.
- `codex-rs/core/src/client.rs`, `codex-rs/core/src/client_tests.rs`, `codex-rs/core/src/compact_remote.rs`, and `codex-rs/core/src/realtime_conversation.rs` import `AuthMode`; `codex-rs/core/src/thread_manager.rs` imports `ThreadHistoryBuilder` and `TurnStatus`. These are auth/thread protocol leaks, not app catalog leaks.

## Crate Ownership Recommendation

- Keep `codex-app-catalog-types` as the canonical transport-neutral crate for app/connector catalog records (`AppInfo`, branding, metadata, screenshots, reviews).
- Keep `codex-app-catalog-api` as the provider boundary for listing catalog entries and accessible entries. It currently defines the right high-level traits and does not need speculative additions until root wires actual providers.
- Do not move MCP elicitation schema/request types into app catalog crates. If root wants to remove `codex-app-server-protocol` from `codex-tools`, move only a transport-neutral install-suggestion/request model into an appropriate domain/API crate, then let a protocol-facing owner convert that model into `McpServerElicitationRequest`.
- `codex_tools::DiscoverableTool` currently mixes plugin discovery entries and catalog-backed connector entries. If that becomes a dependency problem, root should decide whether it belongs in a small tool-discovery API crate or in `codex-app-catalog-api`; this lane did not move it because the current callers are outside owned paths.

## Root-Owned Manifest Entries Needed

- No root manifest edit is needed for this handoff-only slice.
- `codex-rs/core/Cargo.toml`, `codex-rs/tools/Cargo.toml`, and `codex-rs/connectors/Cargo.toml` already have `codex-app-catalog-types = { workspace = true }`.
- `codex-app-catalog-api` exists in the workspace but is not yet consumed outside its own crate. If root wires provider traits into `core`, `connectors`, or `tools`, add `codex-app-catalog-api = { workspace = true }` only to the consuming crate manifests.
- If root later moves MCP request construction out of `codex-tools`, remove `codex-app-server-protocol` from `codex-rs/tools/Cargo.toml` in the same root-owned manifest slice.

## Verification Performed

- Read all required workflow and delegation notes.
- Inspected app catalog type/API crate source and crate manifests.
- Ran scoped searches for `codex_app_server_protocol`, `app_server_protocol`, `protocol::v2`, `codex_app_catalog_types`, `codex_app_catalog_api`, `AppInfo`, `DiscoverableTool`, and app catalog conversion helpers across `core`, `connectors`, `tools`, `app-server`, and `app-server-protocol`.
- Confirmed no `codex_app_server_protocol` import exists under `codex-rs/connectors/src`.
- No formatter, Just task, Cargo build, Git staging, commit, reset, or checkout was run, per lane constraints. No Rust code changed, so no compile verification was required.

## Blockers

- Root owns all caller wiring, manifest edits, app-server protocol edits, and app catalog conversion helper changes outside the owned paths.
- Root must grant exact files if it wants this lane to change conversion helpers or move `codex_tools::DiscoverableTool` / request-plugin-install structures.
