# Compile Plugin Tool Scout Handoff

Date: 2026-05-20

## Scope

Read-only scout for current compile blockers around plugin install/list tool symbols after the `codex-tools` split. I did not edit source files, run Cargo, run Just, format, stage, or commit.

## Sources Inspected

- `.codex/workflow/agents/dab_availability_worker.handoff.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/tools/src/lib.rs`
- `codex-rs/tools/src/tool_discovery.rs`
- `codex-rs/tools/src/request_plugin_install.rs`
- `codex-rs/tools/src/tool_registry_plan.rs`
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/handlers/mod.rs`
- `codex-rs/core/src/tools/handlers/list_available_plugins_to_install.rs`
- `codex-rs/core/src/tools/handlers/list_available_plugins_to_install_spec.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install_spec.rs`
- Targeted test references in `codex-rs/core/tests/suite/request_plugin_install.rs` and `codex-rs/core/src/tools/spec_plan_tests.rs`

## Exact Missing/Stale Symbols

1. `codex_tools::LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME`
   - Still imported/used in core:
     - `codex-rs/core/src/tools/handlers/list_available_plugins_to_install.rs:1,60,80,87`
     - `codex-rs/core/src/tools/handlers/list_available_plugins_to_install_spec.rs:2,13`
     - `codex-rs/core/src/tools/handlers/request_plugin_install.rs:11,132`
     - `codex-rs/core/src/tools/handlers/request_plugin_install_spec.rs:2,35`
   - Not present anywhere under `codex-rs/tools/src`.

2. `codex_tools::ListAvailablePluginsToInstallResult`
   - Still imported/constructed in `codex-rs/core/src/tools/handlers/list_available_plugins_to_install.rs:2,32,33`.
   - Not present anywhere under `codex-rs/tools/src`.

3. `crate::tools::handlers::request_plugin_install_spec::create_request_plugin_install_tool`
   - Core-local stale spec factory still used by `RequestPluginInstallHandler::spec()` at `codex-rs/core/src/tools/handlers/request_plugin_install.rs:36,48`.
   - This spec imports the missing list-tool constant and describes a two-tool flow that no longer matches the `codex-tools` source of truth.

4. `RequestPluginInstallHandler::new(discoverable_tools)`
   - `codex-rs/core/src/tools/spec_plan.rs:343` calls `RequestPluginInstallHandler::new(discoverable_tools)`.
   - `codex-rs/core/src/tools/handlers/request_plugin_install.rs:40` currently defines `pub struct RequestPluginInstallHandler;` and has no `new` implementation.
   - This is not a `codex-tools` export, but it is part of the same plugin-install compile surface.

## Current Replacement / Source Of Truth

The current `codex-tools` source of truth is a single `request_plugin_install` tool, not a separate `list_available_plugins_to_install` tool.

- `codex-rs/tools/src/lib.rs:153-168` re-exports:
  - `DiscoverableTool`
  - `DiscoverableToolAction`
  - `DiscoverableToolType`
  - `REQUEST_PLUGIN_INSTALL_TOOL_NAME`
  - `RequestPluginInstallEntry`
  - `collect_request_plugin_install_entries`
  - `create_request_plugin_install_tool`
  - `filter_request_plugin_install_discoverable_tools_for_client`

- `codex-rs/tools/src/tool_discovery.rs:18` defines `REQUEST_PLUGIN_INSTALL_TOOL_NAME` as `"request_plugin_install"`.
- `codex-rs/tools/src/tool_discovery.rs:278` defines `create_request_plugin_install_tool(&[RequestPluginInstallEntry])`.
- `codex-rs/tools/src/tool_discovery.rs:306-327` builds the request tool description/parameters and embeds the known installable plugin/connector candidates in that tool description.
- `codex-rs/tools/src/tool_discovery.rs:330-376` converts `DiscoverableTool` values into `RequestPluginInstallEntry` values and formats them for the model-visible tool description.
- `codex-rs/tools/src/tool_registry_plan.rs:329-343` registers only `REQUEST_PLUGIN_INSTALL_TOOL_NAME` when `tool_suggest` is enabled and discoverable tools are present.

`codex-rs/tools/src/request_plugin_install.rs` remains the source for request payload/result/elicitation types:

- `RequestPluginInstallArgs`
- `RequestPluginInstallResult`
- `RequestPluginInstallMeta`
- `build_request_plugin_install_elicitation_request`
- connector install completion helpers

## Recommended Fix

Do not reintroduce `LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME` or `ListAvailablePluginsToInstallResult` into `codex-tools`. That would restore an obsolete two-tool API and duplicate the installable-candidate list that now belongs in `codex_tools::create_request_plugin_install_tool(&[RequestPluginInstallEntry])`.

Recommended implementation slice:

1. In `codex-rs/core/src/tools/handlers/request_plugin_install.rs`:
   - Remove the `LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME` import.
   - Remove the import of core-local `request_plugin_install_spec::create_request_plugin_install_tool`.
   - Import/use `codex_tools::collect_request_plugin_install_entries` and `codex_tools::create_request_plugin_install_tool` instead.
   - Change `RequestPluginInstallHandler` from a unit struct into a small struct holding the discoverable tools needed for its spec, or holding precomputed `RequestPluginInstallEntry` values.
   - Add `RequestPluginInstallHandler::new(discoverable_tools)` to match `spec_plan.rs`.
   - Make `spec()` return `codex_tools::create_request_plugin_install_tool(&entries)`.
   - Update the invalid `tool_id` error string to reference `REQUEST_PLUGIN_INSTALL_TOOL_NAME` or "the installable tools listed in the request_plugin_install tool description" instead of the removed list tool.

2. In `codex-rs/core/src/tools/handlers/mod.rs`:
   - Stop compiling/exporting `list_available_plugins_to_install` and `list_available_plugins_to_install_spec`.
   - Stop compiling/exporting `request_plugin_install_spec` if `RequestPluginInstallHandler::spec()` uses the `codex-tools` factory directly.

3. Remove or leave unreferenced for follow-up deletion:
   - `codex-rs/core/src/tools/handlers/list_available_plugins_to_install.rs`
   - `codex-rs/core/src/tools/handlers/list_available_plugins_to_install_spec.rs`
   - `codex-rs/core/src/tools/handlers/request_plugin_install_spec.rs`

4. Update affected tests:
   - `codex-rs/core/tests/suite/request_plugin_install.rs` currently defines/expect-checks `LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME` and the old two-tool flow.
   - `codex-rs/core/src/tools/spec_plan_tests.rs` has request-plugin-install spec expectations.
   - Any tests inside the removed core-local spec/list files should be deleted with those files or moved to `codex-rs/tools/src/tool_discovery_tests.rs` only if they still cover the current single-tool API.

## Files Likely Touched

Primary implementation files:

- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/core/src/tools/handlers/mod.rs`
- `codex-rs/core/src/tools/handlers/list_available_plugins_to_install.rs`
- `codex-rs/core/src/tools/handlers/list_available_plugins_to_install_spec.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install_spec.rs`

Likely test files:

- `codex-rs/core/tests/suite/request_plugin_install.rs`
- `codex-rs/core/src/tools/spec_plan_tests.rs`

Probably not touched unless the implementation exposes a mismatch:

- `codex-rs/tools/src/tool_discovery.rs`
- `codex-rs/tools/src/request_plugin_install.rs`
- `codex-rs/tools/src/lib.rs`
- `codex-rs/tools/src/tool_registry_plan.rs`

## Delegation Safety

Safe to delegate as a focused implementation slice.

Suggested ownership boundary for a worker:

- Own only the core plugin install handler/spec cleanup and the two affected test files.
- Treat `codex-rs/tools/src/tool_discovery.rs` as the source of truth and avoid changing it unless a test reveals a real API gap.
- Do not broaden into unrelated compile blockers from the handoffs (`Session::input_queue`, `Op::UserInput.thread_settings`, `LocalThreadStore`, DAB registration, etc.).

Suggested verification after implementation, not run by this scout:

- `just fmt` from `codex-rs`
- Focused release test lane for `codex-core`, with a request/plugin filter via `scripts\test-local-codex-release.ps1`, following this checkout's release-only build rules.
