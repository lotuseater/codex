# core_dependency_map_scout Handoff

Status: completed read-only dependency map scout on 2026-05-20.

## Scope

This pass inspected the required SOLID refactor handoffs, `codex-core` manifest
and source imports, the app/server/protocol/MCP/connector manifests, and the
newly introduced domain/API crates. It did not edit source, manifests, lockfiles,
generated files, or other handoffs. No Cargo, Just, formatters, staging, or
commits were run.

## Sources Inspected

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/agents/manifest_wiring_scout.handoff.md`
- `.codex/workflow/agents/boundary_delta_scout.handoff.md`
- `codex-rs/core/Cargo.toml`
- `codex-rs/core/src/client.rs`
- `codex-rs/core/src/client_tests.rs`
- `codex-rs/core/src/compact_remote.rs`
- `codex-rs/core/src/realtime_conversation.rs`
- `codex-rs/core/src/mcp_tool_call.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/thread_manager.rs`
- `codex-rs/core/src/connectors.rs`
- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/plugins/injection.rs`
- `codex-rs/core/src/plugins/mentions.rs`
- `codex-rs/core/src/tools/handlers/mcp.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/core/src/tools/handlers/tool_search.rs`
- `codex-rs/core/tests/suite/rmcp_client.rs`
- `codex-rs/app-server/Cargo.toml`
- `codex-rs/app-server-protocol/Cargo.toml`
- `codex-rs/codex-api/Cargo.toml`
- `codex-rs/codex-mcp/Cargo.toml`
- `codex-rs/connectors/Cargo.toml`
- `codex-rs/login/Cargo.toml`
- `codex-rs/core-api/Cargo.toml`
- `codex-rs/core-api/src/lib.rs`
- `codex-rs/runtime-domain/auth-api/*`
- `codex-rs/runtime-domain/model-client-api/*`
- `codex-rs/mcp/elicitation-api/*`
- `codex-rs/thread/thread-projection-api/*`

## Exact Forbidden Imports And Dependencies Still Visible

### App-server protocol DTOs inside `codex-core`

`codex-core` source still imports app-server protocol DTOs directly:

- `codex-rs/core/src/client.rs:64:use codex_app_server_protocol::AuthMode;`
- `codex-rs/core/src/client_tests.rs:17:use codex_app_server_protocol::AuthMode;`
- `codex-rs/core/src/compact_remote.rs:25:use codex_app_server_protocol::AuthMode;`
- `codex-rs/core/src/realtime_conversation.rs:23:use codex_app_server_protocol::AuthMode;`
- `codex-rs/core/src/mcp_tool_call.rs:29:use codex_app_server_protocol::McpElicitationObjectType;`
- `codex-rs/core/src/mcp_tool_call.rs:30:use codex_app_server_protocol::McpElicitationSchema;`
- `codex-rs/core/src/mcp_tool_call.rs:31:use codex_app_server_protocol::McpServerElicitationRequest;`
- `codex-rs/core/src/mcp_tool_call.rs:32:use codex_app_server_protocol::McpServerElicitationRequestParams;`
- `codex-rs/core/src/session/mod.rs:54:use codex_app_server_protocol::McpServerElicitationRequest;`
- `codex-rs/core/src/session/mod.rs:55:use codex_app_server_protocol::McpServerElicitationRequestParams;`
- `codex-rs/core/src/session/tests.rs:79:use codex_app_server_protocol::McpElicitationSchema;`
- `codex-rs/core/src/thread_manager.rs:19:use codex_app_server_protocol::ThreadHistoryBuilder;`
- `codex-rs/core/src/thread_manager.rs:20:use codex_app_server_protocol::TurnStatus;`

No matching `codex-app-server-protocol` entry was found in
`codex-rs/core/Cargo.toml`, so this is also a manifest/source mismatch in the
current tree.

Candidate owners:

- `AuthMode`: `codex-auth-api` (`codex-rs/runtime-domain/auth-api/src/lib.rs`)
  already defines `AuthMode`.
- MCP elicitation request/schema types: `codex-mcp-elicitation-api`
  (`codex-rs/mcp/elicitation-api/src/lib.rs`) already defines the matching
  request/schema surface.
- `TurnStatus` and projected thread history: `codex-thread-projection-api`
  (`codex-rs/thread/thread-projection-api/src/turn.rs`) already defines
  `TurnStatus`, `ProjectedTurn`, and `ThreadHistoryProjection`.
- `ThreadHistoryBuilder`: likely belongs beside `codex-thread-projection-api`,
  with app-server protocol keeping only wire DTO conversion and TS/schema export.

### Direct `codex-core` manifest edges into outer/runtime adapter crates

Current direct dependencies in `codex-rs/core/Cargo.toml`:

- `codex-rs/core/Cargo.toml:25:codex-api = { workspace = true }`
- `codex-rs/core/Cargo.toml:33:codex-connectors = { workspace = true }`
- `codex-rs/core/Cargo.toml:47:codex-login = { workspace = true }`
- `codex-rs/core/Cargo.toml:50:codex-mcp = { workspace = true }`
- `codex-rs/core/Cargo.toml:63:codex-protocol = { workspace = true }`

`codex-protocol` is pervasive shared protocol surface and not the first removal
target for this scout. The other four edges are high-value coupling targets.

### `codex-api` usage inside `codex-core`

`codex-rs/core/src/client.rs` imports concrete request/transport/client types:

- `34:use codex_api::ApiError;`
- `35:use codex_api::AuthProvider;`
- `36:use codex_api::CompactClient as ApiCompactClient;`
- `37:use codex_api::CompactionInput as ApiCompactionInput;`
- `38:use codex_api::Compression;`
- `39:use codex_api::MemoriesClient as ApiMemoriesClient;`
- `40:use codex_api::MemorySummarizeInput as ApiMemorySummarizeInput;`
- `41:use codex_api::MemorySummarizeOutput as ApiMemorySummarizeOutput;`
- `42:use codex_api::Provider as ApiProvider;`
- `43:use codex_api::RawMemory as ApiRawMemory;`
- `44:use codex_api::RealtimeCallClient as ApiRealtimeCallClient;`
- `45:use codex_api::RealtimeSessionConfig as ApiRealtimeSessionConfig;`
- `46:use codex_api::Reasoning;`
- `47:use codex_api::RequestTelemetry;`
- `48:use codex_api::ReqwestTransport;`
- `49:use codex_api::ResponseCreateWsRequest;`
- `50:use codex_api::ResponsesApiRequest;`
- `51:use codex_api::ResponsesClient as ApiResponsesClient;`
- `52:use codex_api::ResponsesOptions as ApiResponsesOptions;`
- `53:use codex_api::ResponsesWebsocketClient as ApiWebSocketResponsesClient;`
- `54:use codex_api::ResponsesWebsocketConnection as ApiWebSocketConnection;`
- `55:use codex_api::ResponsesWsRequest;`
- `56:use codex_api::SharedAuthProvider;`
- `57:use codex_api::SseTelemetry;`
- `58:use codex_api::TransportError;`
- `59:use codex_api::WebsocketTelemetry;`
- `60:use codex_api::auth_header_telemetry;`
- `61:use codex_api::build_session_headers;`
- `62:use codex_api::create_text_param_for_request;`
- `63:use codex_api::response_create_client_metadata;`

Candidate owner: `codex-model-client-api`
(`codex-rs/runtime-domain/model-client-api/src/lib.rs`) exists but is not used
by `codex-core`, `codex-api`, or `app-server` yet. It is currently too skeletal
to replace `client.rs` directly, but it is the likely owner for the abstraction
side of a later `codex-core -> codex-api` split.

### `codex-connectors` usage inside `codex-core`

Direct imports and fully-qualified calls remain in core:

- `codex-rs/core/src/connectors.rs:13:use codex_connectors::ConnectorDirectoryCacheContext;`
- `codex-rs/core/src/connectors.rs:14:use codex_connectors::ConnectorDirectoryCacheKey;`
- `codex-rs/core/src/connectors.rs:118:codex_connectors::merge::merge_plugin_connectors(...)`
- `codex-rs/core/src/connectors.rs:123:codex_connectors::filter::filter_tool_suggest_discoverable_connectors(...)`
- `codex-rs/core/src/connectors.rs:154:codex_connectors::filter::filter_disallowed_connectors(...)`
- `codex-rs/core/src/connectors.rs:171:codex_connectors::filter::filter_disallowed_connectors(...)`
- `codex-rs/core/src/connectors.rs:234:codex_connectors::filter::filter_disallowed_connectors(...)`
- `codex-rs/core/src/connectors.rs:345:codex_connectors::filter::filter_disallowed_connectors(...)`
- `codex-rs/core/src/connectors.rs:405:codex_connectors::CONNECTORS_CACHE_TTL`
- `codex-rs/core/src/connectors.rs:475:codex_connectors::cached_directory_connectors(...)`
- `codex-rs/core/src/connectors.rs:486:codex_connectors::accessible::AccessibleConnectorTool`
- `codex-rs/core/src/connectors.rs:493:codex_connectors::accessible::collect_accessible_connectors(...)`
- `codex-rs/core/src/mcp_tool_call.rs:623:codex_connectors::metadata::connector_install_url(...)`
- `codex-rs/core/src/plugins/injection.rs:3:use codex_connectors::metadata::connector_display_label;`
- `codex-rs/core/src/plugins/mentions.rs:4:use codex_connectors::metadata::connector_mention_slug;`
- `codex-rs/core/src/connectors_tests.rs:16:use codex_connectors::merge::plugin_connector_to_app_info;`
- `codex-rs/core/src/connectors_tests.rs:17:use codex_connectors::metadata::connector_install_url;`
- `codex-rs/core/src/connectors_tests.rs:18:use codex_connectors::metadata::sanitize_name;`

Candidate owner: keep connector metadata/filter/merge policy in
`codex-connectors`, but move orchestration that needs MCP manager state behind a
narrow connector service/port. Avoid moving this policy into `codex-core`.

### `codex-mcp` usage inside `codex-core`

Direct imports and calls remain in core:

- `codex-rs/core/src/connectors.rs:36:use codex_mcp::CODEX_APPS_MCP_SERVER_NAME;`
- `codex-rs/core/src/connectors.rs:37:use codex_mcp::McpConnectionManager;`
- `codex-rs/core/src/connectors.rs:38:use codex_mcp::McpRuntimeEnvironment;`
- `codex-rs/core/src/connectors.rs:39:use codex_mcp::ToolInfo;`
- `codex-rs/core/src/connectors.rs:40:use codex_mcp::ToolPluginProvenance;`
- `codex-rs/core/src/connectors.rs:41:use codex_mcp::codex_apps_tools_cache_key;`
- `codex-rs/core/src/connectors.rs:42:use codex_mcp::compute_auth_statuses;`
- `codex-rs/core/src/connectors.rs:43:use codex_mcp::host_owned_codex_apps_enabled;`
- `codex-rs/core/src/connectors.rs:44:use codex_mcp::with_codex_apps_mcp;`
- `codex-rs/core/src/mcp_tool_call.rs:37:use codex_mcp::CODEX_APPS_MCP_SERVER_NAME;`
- `codex-rs/core/src/mcp_tool_call.rs:38:use codex_mcp::MCP_TOOL_CODEX_APPS_META_KEY;`
- `codex-rs/core/src/mcp_tool_call.rs:39:use codex_mcp::McpPermissionPromptAutoApproveContext;`
- `codex-rs/core/src/mcp_tool_call.rs:40:use codex_mcp::SandboxState;`
- `codex-rs/core/src/mcp_tool_call.rs:41:use codex_mcp::auth_elicitation_completed_result;`
- `codex-rs/core/src/mcp_tool_call.rs:42:use codex_mcp::build_auth_elicitation_plan;`
- `codex-rs/core/src/mcp_tool_call.rs:43:use codex_mcp::declared_openai_file_input_param_names;`
- `codex-rs/core/src/mcp_tool_call.rs:44:use codex_mcp::mcp_permission_prompt_is_auto_approved;`
- `codex-rs/core/src/mcp_tool_call.rs:720:codex_mcp::MCP_SANDBOX_STATE_META_CAPABILITY.to_string()`
- `codex-rs/core/src/mcp_tool_call.rs:728:codex_mcp::MCP_SANDBOX_STATE_META_CAPABILITY.to_string()`
- `codex-rs/core/src/plugins/injection.rs:11:use codex_mcp::CODEX_APPS_MCP_SERVER_NAME;`
- `codex-rs/core/src/plugins/injection.rs:12:use codex_mcp::ToolInfo;`
- `codex-rs/core/src/tools/handlers/mcp.rs:20:use codex_mcp::ToolInfo;`
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs:5:use codex_mcp::CODEX_APPS_MCP_SERVER_NAME;`
- `codex-rs/core/src/tools/handlers/tool_search.rs:143:use codex_mcp::ToolInfo;`
- `codex-rs/core/tests/suite/rmcp_client.rs:28:use codex_mcp::MCP_SANDBOX_STATE_META_CAPABILITY;`

Candidate owners:

- MCP runtime/connection manager behavior should stay in `codex-mcp`.
- Elicitation request/schema types should move through
  `codex-mcp-elicitation-api`.
- Core should depend on small MCP API/port crates or traits where possible,
  while concrete `McpConnectionManager` wiring stays in an adapter layer.

### `codex-login` direct and transitive leak

`codex-core` directly depends on `codex-login`:

- `codex-rs/core/Cargo.toml:47:codex-login = { workspace = true }`

`codex-login` itself depends on app-server protocol:

- `codex-rs/login/Cargo.toml:15:codex-app-server-protocol = { workspace = true }`
- `codex-rs/login/Cargo.toml:21:codex-protocol = { workspace = true }`

So even after replacing direct `codex_app_server_protocol::AuthMode` imports in
core, the `core -> login -> app-server-protocol` transitive path remains until
`AuthMode` and any login-owned app-server DTO usage are moved to `codex-auth-api`.

Candidate owner: `codex-auth-api` for `AuthMode`, `AuthCredential`, and auth
provider traits; `codex-login` should implement/adapt that API rather than own
or import app-server wire DTOs.

### `codex-core-api` facade leak

`codex-core-api` is not currently used by `codex-core`, but it is a likely
future transitive leak if treated as the primary boundary facade:

- `codex-rs/core-api/Cargo.toml:17:codex-app-server-protocol = { workspace = true }`
- `codex-rs/core-api/Cargo.toml:24:codex-login = { workspace = true }`
- `codex-rs/core-api/Cargo.toml:27:codex-protocol = { workspace = true }`
- `codex-rs/core-api/src/lib.rs:6:pub use codex_app_server_protocol::ServerNotification;`
- `codex-rs/core-api/src/lib.rs:7:pub use codex_app_server_protocol::item_event_to_server_notification;`
- `codex-rs/core-api/src/lib.rs:32:pub use codex_login::AuthManager;`
- `codex-rs/core-api/src/lib.rs:33:pub use codex_login::default_client::set_default_originator;`

Candidate owner: do not expand `codex-core-api` as a compatibility re-export
shim. Split its facade by responsibility, or keep it sample-only until it can
re-export the new domain/API crates instead of app-server protocol and login.

### Newly introduced domain/API crates not yet carrying the boundary

Observed prepared crates:

- `codex-rs/runtime-domain/auth-api/Cargo.toml:4:name = "codex-auth-api"`
- `codex-rs/runtime-domain/model-client-api/Cargo.toml:4:name = "codex-model-client-api"`
- `codex-rs/mcp/elicitation-api/Cargo.toml:4:name = "codex-mcp-elicitation-api"`
- `codex-rs/thread/thread-projection-api/Cargo.toml:4:name = "codex-thread-projection-api"`

`codex-auth-api` is wired in the workspace:

- `codex-rs/Cargo.toml:111:"runtime-domain/auth-api",`
- `codex-rs/Cargo.toml:194:codex-auth-api = { path = "runtime-domain/auth-api" }`

`codex-mcp-elicitation-api` and `codex-thread-projection-api` have manifests and
source files, but no workspace references were found in `codex-rs/Cargo.toml`.
Searches also found no `codex_auth_api`, `codex_mcp_elicitation_api`, or
`codex_thread_projection_api` imports in `codex-core`, `app-server-protocol`,
`app-server`, or `codex-mcp`.

## Likely Transitive Dependency Leaks

- `codex-core -> codex-login -> codex-app-server-protocol`: auth mode/wire DTO
  coupling survives direct import cleanup unless `codex-login` moves to
  `codex-auth-api`.
- `codex-core -> codex-mcp -> codex-api/codex-login/codex-protocol`:
  `codex-mcp/Cargo.toml` depends on `codex-api` at line 19, `codex-login` at
  line 22, and `codex-protocol` at line 26. Concrete MCP manager imports inside
  core therefore pull runtime/auth/model concerns through the MCP crate.
- `codex-app-server -> codex-core` plus direct `codex-app-server-protocol`,
  `codex-login`, and `codex-mcp` dependencies in `app-server/Cargo.toml` lines
  38, 57, 60, 63, and 64 make app-server the proper adapter owner. Core should
  not import app-server protocol to satisfy app-server read/projection needs.
- `codex-core-api -> codex-app-server-protocol/codex-login` is not a core leak
  today, but it is a ready-made leak if future slices route boundary cleanup
  through the facade instead of responsibility-specific API crates.

## Recommended Implementation Order

1. **AuthMode boundary cleanup**

   Replace core and login usage of `codex_app_server_protocol::AuthMode` with
   `codex_auth_api::AuthMode`. Keep app-server protocol wire compatibility via
   conversion in `app-server-protocol`, not via a re-export.

   Likely owned files:

   - `codex-rs/runtime-domain/auth-api/src/lib.rs`
   - `codex-rs/login/Cargo.toml`
   - `codex-rs/login/src/**`
   - `codex-rs/app-server-protocol/src/protocol/common.rs`
   - `codex-rs/core/Cargo.toml`
   - `codex-rs/core/src/client.rs`
   - `codex-rs/core/src/client_tests.rs`
   - `codex-rs/core/src/compact_remote.rs`
   - `codex-rs/core/src/realtime_conversation.rs`

2. **MCP elicitation DTO cleanup**

   Wire `codex-mcp-elicitation-api` into `codex-rs/Cargo.toml` and use it from
   core and app-server protocol. App-server protocol should own wire conversion,
   schema/TS exports, and compatibility with v2 payload names.

   Likely owned files:

   - `codex-rs/Cargo.toml`
   - `codex-rs/mcp/elicitation-api/Cargo.toml`
   - `codex-rs/mcp/elicitation-api/src/lib.rs`
   - `codex-rs/app-server-protocol/Cargo.toml`
   - `codex-rs/app-server-protocol/src/protocol/v2/mcp.rs`
   - `codex-rs/core/Cargo.toml`
   - `codex-rs/core/src/mcp_tool_call.rs`
   - `codex-rs/core/src/session/mod.rs`
   - `codex-rs/core/src/session/tests.rs`

3. **Thread projection / history boundary**

   Wire `codex-thread-projection-api` into the workspace and migrate
   `TurnStatus` plus projection-owned history types there. Defer the full
   `ThreadHistoryBuilder` migration until the item DTO and `CodexErrorInfo`
   conversion path are explicit.

   Likely owned files:

   - `codex-rs/Cargo.toml`
   - `codex-rs/thread/thread-projection-api/Cargo.toml`
   - `codex-rs/thread/thread-projection-api/src/lib.rs`
   - `codex-rs/thread/thread-projection-api/src/turn.rs`
   - `codex-rs/app-server-protocol/Cargo.toml`
   - `codex-rs/app-server-protocol/src/protocol/thread_history.rs`
   - `codex-rs/app-server-protocol/src/protocol/v2/turn.rs`
   - `codex-rs/core/Cargo.toml`
   - `codex-rs/core/src/thread_manager.rs`

4. **Connector/MCP orchestration split**

   After DTO cleanup, reduce `codex-core` direct reliance on concrete
   `codex-connectors` and `codex-mcp` manager/helper APIs. This is larger than
   the DTO slices because it crosses plugin injection, tool search, MCP tool
   call handling, and connector directory caching.

   Likely owned files:

   - `codex-rs/core/src/connectors.rs`
   - `codex-rs/core/src/connectors_tests.rs`
   - `codex-rs/core/src/session/turn.rs`
   - `codex-rs/core/src/mcp_tool_call.rs`
   - `codex-rs/core/src/plugins/injection.rs`
   - `codex-rs/core/src/plugins/mentions.rs`
   - `codex-rs/core/src/tools/handlers/mcp.rs`
   - `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
   - `codex-rs/core/src/tools/handlers/tool_search.rs`
   - `codex-rs/codex-mcp/src/**`
   - `codex-rs/connectors/src/**`

5. **Model client abstraction**

   Use `codex-model-client-api` only after the smaller DTO boundaries are green.
   The current `client.rs` usage of `codex-api` is broad and concrete; moving it
   too early risks a large, hard-to-review client refactor.

   Likely owned files:

   - `codex-rs/runtime-domain/model-client-api/src/lib.rs`
   - `codex-rs/codex-api/src/**`
   - `codex-rs/core/src/client.rs`
   - `codex-rs/core/src/mcp_openai_file.rs`
   - `codex-rs/core/tests/suite/client_websockets.rs`

## Commit Readiness Notes

- This scout made no implementation changes and did not commit.
- `git status --short` already shows a large dirty tree (120 paths in the
  reduced status output), including manifest, lockfile, app-server, protocol,
  connector, and many Rust source changes. Future implementation workers must
  read their owned files before editing and must not revert unrelated dirty work.
- The first commit-ready implementation slice should be the `AuthMode` cleanup:
  it is small, already has a wired owner crate (`codex-auth-api`), and removes
  both direct app-server protocol imports from core auth call sites and the
  `core -> login -> app-server-protocol` transitive pressure when login follows.
- The MCP elicitation and thread projection crates are prepared but not wired
  into the workspace. Any slice that wires them must update `Cargo.toml` /
  `Cargo.lock` coherently and then, outside this scout, run the required
  workspace lock/schema/test lanes for that implementation.
- Do not mark the app-server protocol boundary green until searches for
  `codex_app_server_protocol` in `codex-rs/core/src` return no DTO imports and
  `codex-rs/core/Cargo.toml` still has no `codex-app-server-protocol`
  dependency.
