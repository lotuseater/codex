# mcp_elicitation_boundary Handoff

Status: prepared owned MCP elicitation type crate files; manifests and wiring are
left to root.

## Paths Changed

- `codex-rs/mcp/elicitation-api/Cargo.toml`
- `codex-rs/mcp/elicitation-api/src/lib.rs`
- `.codex/workflow/agents/mcp_elicitation_boundary.handoff.md`

## Paths Read

- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/app-server-protocol/src/protocol/v2/mcp.rs`
- `codex-rs/protocol/src/approvals.rs`
- `codex-rs/protocol/src/protocol.rs`
- `codex-rs/core/src/mcp_tool_call.rs`
- `codex-rs/core/src/mcp_tool_call_tests.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/mcp.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/codex-mcp/src/elicitation.rs`
- `codex-rs/codex-mcp/src/connection_manager.rs`
- `codex-rs/codex-mcp/src/rmcp_client.rs`
- `codex-rs/rmcp-client/src/lib.rs`
- `codex-rs/rmcp-client/src/rmcp_client.rs`
- `codex-rs/rmcp-client/src/elicitation_client_service.rs`
- `codex-rs/rmcp-client/src/logging_client_handler.rs`
- `codex-rs/tools-domain/tool-handler-api/{Cargo.toml,src/lib.rs}`
- `codex-rs/tools-domain/tool-execution-api/{Cargo.toml,src/lib.rs}`
- `codex-rs/tools-domain/tool-registry-api/src/lib.rs`

## Exact App-Server-Protocol Types Involved

Direct `codex-core` leak from `codex_app_server_protocol`:

- `McpServerElicitationRequestParams`
- `McpServerElicitationRequest`
- `McpElicitationSchema`
- `McpElicitationObjectType`

Full app-server v2 elicitation request/schema graph inspected:

- `McpElicitationPrimitiveSchema`
- `McpElicitationEnumSchema`
- `McpElicitationStringSchema`
- `McpElicitationStringType`
- `McpElicitationStringFormat`
- `McpElicitationNumberSchema`
- `McpElicitationNumberType`
- `McpElicitationBooleanSchema`
- `McpElicitationBooleanType`
- `McpElicitationLegacyTitledEnumSchema`
- `McpElicitationSingleSelectEnumSchema`
- `McpElicitationUntitledSingleSelectEnumSchema`
- `McpElicitationTitledSingleSelectEnumSchema`
- `McpElicitationMultiSelectEnumSchema`
- `McpElicitationUntitledMultiSelectEnumSchema`
- `McpElicitationTitledMultiSelectEnumSchema`
- `McpElicitationArrayType`
- `McpElicitationUntitledEnumItems`
- `McpElicitationTitledEnumItems`
- `McpElicitationConstOption`

Related response/resolve DTOs inspected but not moved in this slice:

- `McpServerElicitationAction`
- `McpServerElicitationResponse`
- `IntoResponse<McpServerElicitationResponse>`

## Crate Ownership Recommendation

Make `codex-mcp-elicitation-api` the owner for MCP elicitation request and form
schema abstractions. The prepared crate is under
`codex-rs/mcp/elicitation-api` and intentionally depends only on `serde` and
`serde_json`; it does not depend on `codex-core`, `codex-app-server-protocol`,
`rmcp`, `schemars`, or `ts-rs`.

Recommended layering:

- `codex-core` and core tool handlers import request/schema types from
  `codex_mcp_elicitation_api`, not from `codex_app_server_protocol`.
- `codex-app-server-protocol` remains the API export/TypeScript/schema adapter.
  It should either convert from these MCP-owned types into v2 DTOs, or root can
  decide to add schema/TS derives to this crate if direct export is preferred.
- Keep response/action ownership separate for now. Core already uses
  `codex_rmcp_client::ElicitationResponse` and `rmcp::model::ElicitationAction`;
  moving app-server response DTOs is not needed to break the current core
  request/schema dependency.

## Root-Owned Manifest Entries Needed

- Add `mcp/elicitation-api` as a `codex-rs/Cargo.toml` workspace member.
- Add a workspace dependency entry for `codex-mcp-elicitation-api`.
- Add `codex-mcp-elicitation-api = { workspace = true }` to
  `codex-rs/core/Cargo.toml` before replacing the core imports.
- Add `codex-mcp-elicitation-api = { workspace = true }` to
  `codex-rs/app-server-protocol/Cargo.toml` if app-server protocol maps from
  the MCP-owned types.
- Add/adjust Bazel package/build entries for the new crate.
- Refresh `Cargo.lock`, Bazel lockfiles, and generated app-server schema
  fixtures only after root performs the manifest and protocol wiring.

## Verification Performed

- `rg` confirmed current direct core imports of MCP elicitation DTOs from
  `codex_app_server_protocol` in:
  - `codex-rs/core/src/mcp_tool_call.rs`
  - `codex-rs/core/src/session/mod.rs`
  - `codex-rs/core/src/session/tests.rs`
- `rg` confirmed the prepared crate exposes the MCP elicitation request/schema
  type surface.
- File stat checks confirmed the new crate files and this handoff exist.
- No Git, staging, reset, checkout, formatter, Cargo build/test, Just task, or
  broad verification was run, per lane constraints.

## Blockers

- The new crate uses workspace package/dependency/lint fields, so Cargo cannot
  check it until root adds the workspace member.
- Core import rewiring is blocked on root-owned manifest edits.
- App-server protocol conversion/schema fixture updates are blocked by the lane
  restriction against editing app-server protocol files without exact root grant.
