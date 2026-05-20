# app_server_boundary_scout Handoff

Status: complete, read-only scout.
Date: 2026-05-20

## Scope

This pass inspected the current `codex-rs/app-server` boundary after the
in-flight SOLID refactor handoffs. It did not edit source files, manifests,
lockfiles, generated files, or any other handoff.

## Sources Inspected

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/app_catalog_followup.handoff.md`
- `.codex/workflow/agents/thread_store_boundary.handoff.md`
- `.codex/workflow/agents/thread_projection_boundary.handoff.md`
- `codex-rs/app-server/Cargo.toml`
- `codex-rs/app-server/src/app_catalog_protocol.rs`
- `codex-rs/app-server/src/request_processors.rs`
- `codex-rs/app-server/src/request_processors/apps_processor.rs`
- `codex-rs/app-server/src/request_processors/config_processor.rs`
- `codex-rs/app-server/src/request_processors/external_agent_config_processor.rs`
- `codex-rs/app-server/src/request_processors/plugins.rs`
- `codex-rs/app-server/src/request_processors/thread_processor.rs`
- `codex-rs/app-server/src/mcp_refresh.rs`
- `codex-rs/app-server/src/message_processor.rs`
- `codex-rs/app-server/src/thread_state.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/{apps.rs,config.rs,thread.rs}`
- `codex-rs/app/app-catalog-api/src/lib.rs`
- `codex-rs/app/app-catalog-types/src/lib.rs`
- `codex-rs/thread/thread-store-api/src/{store.rs,types.rs}`
- `codex-rs/thread/thread-projection-api/src/{lib.rs,page.rs,turn.rs}`

## Findings

### 1. App catalog listing policy still lives in app-server

App-server file/functions:

- `codex-rs/app-server/src/request_processors/apps_processor.rs`
  - `AppsRequestProcessor::apps_list`
  - `AppsRequestProcessor::apps_list_inner`
  - `AppsRequestProcessor::apps_list_task`
  - `AppsRequestProcessor::apps_list_response`
  - `AppsRequestProcessor::load_thread`
  - `AppsRequestProcessor::load_latest_config`
  - `merge_loaded_apps`
  - `should_send_app_list_updated_notification`
  - `paginate_apps`
  - `send_app_list_updated_notification`

Policy currently owned here:

- Backend/auth gating for whether apps are enabled.
- Selection of the runtime config/thread context used to load app catalogs.
- Timeout/retry behavior for partial catalog loads.
- Merge semantics for accessible apps, full catalog apps, enabled state, and
  "ready enough to notify" behavior.
- Cursor validation and pagination limits for `apps/list`.

Proposed destination:

- Move list orchestration, merge policy, notification readiness, and pagination
  to `codex-rs/app/app-catalog-api`, for example a `listing` module with an
  `AppCatalogListService`, `AppCatalogListRequest`, `AppCatalogListPage`, and
  `AppCatalogLoadState`.
- Keep app-server as the edge adapter that resolves the request id, sends the
  early/late JSON-RPC responses, and converts neutral catalog output to v2 wire
  DTOs.

Schema/test risks:

- `apps/list` pagination and `app/listUpdated` notification timing can regress
  even if type shapes do not change.
- App list tests should cover empty backend-disabled responses, invalid cursors,
  partial accessible/full catalog loads, and the no-duplicate notification rule.
- If the neutral page type becomes the source of truth, regenerate app-server
  schema fixtures only when wire DTO fields change.

### 2. App catalog protocol and plugin app-summary policy are split across app-server

App-server file/functions:

- `codex-rs/app-server/src/app_catalog_protocol.rs`
  - `app_infos_to_v2`
  - `app_info_to_v2`
  - `app_summary_from_catalog`
  - `app_branding_to_v2`
  - `app_metadata_to_v2`
  - `app_screenshot_to_v2`
- `codex-rs/app-server/src/request_processors/plugins.rs`
  - `load_plugin_app_summaries`
  - `plugin_apps_needing_auth_for_install`
  - `plugin_apps_needing_auth`
  - `start_plugin_mcp_oauth_logins`
  - `remote_marketplace_to_info`
  - `remote_plugin_summary_to_info`
  - `remote_plugin_detail_to_info`

Policy currently owned here:

- `AppSummary.needs_auth` is derived from accessible-vs-all connector sets in
  plugin code, while `app_summary_from_catalog` lives in a generic app-server
  conversion file.
- Plugin install/auth flows know how to query app catalogs, determine which app
  auth steps are still needed, and project that into protocol summaries.
- Remote plugin catalog details are also converted to protocol DTOs directly in
  app-server.

Proposed destination:

- Move app-summary/auth-state calculation to `codex-rs/app/app-catalog-api`,
  alongside the list merge policy.
- Consider a small `codex-plugin-catalog-api` or `codex-plugin-api` projection
  module for remote plugin marketplace/detail DTOs if plugin catalog projection
  needs to be shared outside app-server.
- Keep only thin `app_catalog_protocol` or protocol-crate conversions at the
  edge. Do not make domain crates depend on `codex-app-server`.

Schema/test risks:

- `AppInfo`, `AppSummary`, screenshot, metadata, and branding wire shapes are in
  `app-server-protocol/src/protocol/v2/apps.rs`; moving conversion helpers must
  preserve camelCase fields and optional/nullability behavior.
- Plugin install tests can regress if `needs_auth` is calculated before
  accessible app data is available or if first-party install semantics change.
- Watch for dependency cycles if `codex-app-catalog-api` is made to import
  `codex-app-server-protocol`; a neutral projection type plus adapter is safer.

### 3. Thread store listing, read, archive, and summary projection policy remains in `thread_processor.rs`

App-server file/functions:

- `codex-rs/app-server/src/request_processors/thread_processor.rs`
  - `thread_list_response_inner`
  - `thread_loaded_list_response_inner`
  - `list_threads_common`
  - `thread_read_response_inner`
  - `read_thread_view`
  - `load_persisted_thread_for_read`
  - `load_live_thread_view`
  - `apply_thread_read_store_fields`
  - `thread_archive_inner`
  - `thread_archive_response`
  - `prepare_thread_for_archive`
  - `thread_unarchive_inner`
  - `thread_unarchive_response`
  - `thread_store_list_error`
  - `thread_store_resume_read_error`
  - `thread_store_archive_error`
  - `summary_from_stored_thread`
  - `summary_from_state_db_metadata`
  - `summary_from_thread_metadata`
  - `preview_from_rollout_items`
  - `set_thread_name_from_title`
  - `requested_permissions_trust_project`
  - `permission_profile_trusts_project`
  - `build_thread_from_snapshot`
  - `build_thread_from_loaded_snapshot`
- `codex-rs/app-server/src/request_processors.rs`
  - `build_api_turns_from_rollout_items`
- `codex-rs/app-server/src/thread_state.rs`
  - `ThreadState`
  - `TurnSummary`
  - uses `ThreadHistoryBuilder` and protocol `Turn` directly

Policy currently owned here:

- Store paging, sort keys, cursor serialization, and cursor error mapping.
- Joining store rows with live thread state and state-db metadata.
- Thread status normalization across live, loaded, archived, and historical
  states.
- Summary/preview extraction from stored threads, rollout items, and thread
  metadata.
- Archive semantics, including removing live threads and archiving descendants.
- Permission-profile trust checks used while projecting API thread state.

Proposed destination:

- Move history/page/status projection into the new
  `codex-rs/thread/thread-projection-api` crate. That crate already exposes
  `ProjectedThread`, `ProjectedTurn`, `ProjectionPage`, `TurnListParams`, and
  `TurnItemsListParams`; it should become the neutral home for the projection
  rules now embedded in app-server.
- Move store-list/archive/read orchestration that is not wire-specific into
  `codex-rs/thread/thread-store-api` or a new adjacent `codex-thread-service-api`
  facade module. App-server should call a service method and adapt its result to
  v2 protocol types.
- Keep app-server-owned logic limited to JSON-RPC request validation, protocol
  conversion, and connection-specific live-notification side effects.

Schema/test risks:

- `thread/list` cursors preserve millisecond precision from the store; any move
  must keep existing cursor serialization stable or intentionally update schema
  fixtures/tests.
- `thread/read`, `thread/turns/list`, `thread/archive`, and `thread/unarchive`
  tests are sensitive to status, archived state, summary, descendant handling,
  and rollout fallback behavior.
- Projection code currently returns app-server protocol `Thread`, `Turn`, and
  `ConversationSummary`; neutralizing it will need adapter tests to avoid
  accidental v2 wire changes.

### 4. Thread turns and rollout-history projection are still protocol-coupled

App-server file/functions:

- `codex-rs/app-server/src/request_processors/thread_processor.rs`
  - `thread_turns_list_response_inner`
  - `load_thread_turns_list_history`
  - `paginate_thread_turns`
  - `serialize_thread_turns_cursor`
  - `parse_thread_turns_cursor`
  - `reconstruct_thread_turns_for_turns_list`
  - `normalize_thread_turns_status`
- `codex-rs/app-server/src/request_processors.rs`
  - `build_api_turns_from_rollout_items`
- `codex-rs/app-server/src/bespoke_event_handling.rs`
  - imports `populate_thread_turns_from_history` and
    `thread_from_stored_thread` from request processors

Policy currently owned here:

- Deciding which rollout items are persisted into API-visible turns.
- Reconstructing turn pages and item pages from rollout history.
- Normalizing active/completed/error turn state.
- Sharing thread projection helpers through request processor modules, which
  makes app-server request code the de facto projection API.

Proposed destination:

- Put rollout-history-to-thread projection and turns pagination in
  `codex-rs/thread/thread-projection-api`, with app-server protocol conversion
  layered on top.
- If `codex-app-server-protocol::ThreadHistoryBuilder` must remain the v2 wire
  builder, wrap it in app-server after neutral projection has completed.

Schema/test risks:

- Snapshot or fixture changes can appear in `thread/read`,
  `thread/turns/list`, conversation summary, and bespoke event tests.
- The neutral projection crate cannot import app-server protocol types without
  defeating the boundary; adapter coverage is required.

### 5. Config requirements and external-agent migration policy remain app-server-local

App-server file/functions:

- `codex-rs/app-server/src/request_processors/config_processor.rs`
  - `ConfigRequestProcessor::config_requirements_read`
  - `map_requirements_toml_to_api`
  - `map_mcp_server_requirements_to_api`
  - `map_file_permission_to_api`
  - `map_network_permission_to_api`
  - `map_network_domain_permission_to_api`
  - `map_network_http_permission_to_api`
  - `map_network_unix_socket_permission_to_api`
  - `map_error`
  - `config_write_error`
- `codex-rs/app-server/src/request_processors/external_agent_config_processor.rs`
  - `ExternalAgentConfigRequestProcessor::detect`
  - `ExternalAgentConfigRequestProcessor::import`
  - `complete_pending_plugin_import`
  - `migration_items_need_runtime_refresh`
  - `session_not_detected_error`
- `codex-rs/app-server/src/mcp_refresh.rs`
  - `queue_strict_refresh`
  - `queue_best_effort_refresh`
  - `build_refresh_config`
  - `queue_refresh`

Policy currently owned here:

- Mapping `ConfigRequirementsToml` into API requirements, including
  normalization such as ensuring disabled web search appears in the API set.
- Mapping config manager write errors into app-server JSON-RPC error data.
- External-agent migration item projection between core config types and v2
  protocol types.
- Import policy for sessions, plugins, ledger updates, runtime refresh, and MCP
  refresh after migration.
- MCP refresh config selection from live config and thread config loaders.

Proposed destination:

- Move config-requirements projection to `codex-rs/config` or a small
  `codex-config-api` crate/module that owns neutral API-facing requirements
  shapes.
- Move external-agent detect/import planning into the existing external-agent
  config domain code or a dedicated `codex-external-agent-config-api` crate.
  App-server should execute a plan and emit JSON-RPC responses/notifications.
- Keep `mcp_refresh.rs` as an edge queue only if the refresh request object is
  built by config/MCP domain code.

Schema/test risks:

- `ConfigRequirements` is v2 experimental schema surface; moving mappers can
  require `just write-app-server-schema` and protocol fixture updates if field
  shape or optionality changes.
- External-agent import touches session import, plugin import, MCP refresh, and
  runtime notifications; partial migration tests should verify each item type.
- Config write error JSON data currently includes a `config_write_error_code`;
  preserving that detail matters for clients.

### 6. Dynamic tool validation is still embedded in thread request processing

App-server file/functions:

- `codex-rs/app-server/src/request_processors/thread_processor.rs`
  - `validate_dynamic_tools`

Policy currently owned here:

- Dynamic tool name/namespace length limits.
- Identifier regex.
- Reserved tool names and reserved Responses API namespaces.
- Error wording for tool and namespace collisions.

Proposed destination:

- Move to `codex-rs/tools`, `codex-rs/protocol`, or a new
  `codex-dynamic-tools-api` module depending on ownership of dynamic tool
  semantics. App-server should call a validator and map its typed errors to
  JSON-RPC invalid request responses.

Schema/test risks:

- Error messages may be asserted by app-server tests or clients.
- Moving this into a shared crate creates an opportunity to make validation
  reusable by core, but the current hard-coded app-server error strings should
  be preserved or intentionally versioned.

## Commit Readiness Notes

- This scout changed only `.codex/workflow/agents/app_server_boundary_scout.handoff.md`.
- No Cargo, Just, formatter, schema generation, staging, commits, or source edits
  were run.
- The source tree already has app-server/config/protocol/thread changes in
  progress. The broad source slice is not commit-ready from this scout alone
  because the remaining boundary moves need root-owned manifest, lockfile, and
  possibly Bazel/schema fixture integration.
- A handoff-only commit would be coherent if root wants to checkpoint scout
  findings before assigning implementation lanes.
