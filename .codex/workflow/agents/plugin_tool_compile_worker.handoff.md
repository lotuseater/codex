# plugin_tool_compile_worker Handoff

Date: 2026-05-20

## Files Read

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/agents/compile_plugin_tool_scout.handoff.md`
- `.codex/workflow/agents/dab_availability_worker.handoff.md`
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install_spec.rs`
- `codex-rs/core/src/tools/handlers/mod.rs`
- `codex-rs/tools/src/request_plugin_install.rs`
- `codex-rs/tools/src/tool_discovery.rs`
- `codex-rs/tools/src/lib.rs`
- `codex-rs/core/src/tools/spec_plan.rs`

## Files Changed

- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/core/src/tools/handlers/mod.rs`
- `.codex/workflow/agents/plugin_tool_compile_worker.handoff.md`

`codex-rs/core/src/tools/handlers/request_plugin_install_spec.rs` was read but not changed. It is no longer compiled from `handlers/mod.rs`; leaving it untouched avoids editing the retired local spec tests in this constrained slice.

## Stale Symbols Removed Or Replaced

- Removed compiled import of `codex_tools::LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME` from `request_plugin_install.rs`.
- Removed compiled import of `crate::tools::handlers::request_plugin_install_spec::create_request_plugin_install_tool`.
- Replaced the local core spec source with `codex_tools::create_request_plugin_install_tool`.
- Added `codex_tools::collect_request_plugin_install_entries` and `codex_tools::RequestPluginInstallEntry`.
- Added `RequestPluginInstallHandler::new(discoverable_tools: Vec<DiscoverableTool>)` to match the existing `spec_plan.rs` call site and precompute request-plugin-install entries for the tool spec.
- Changed the invalid `tool_id` message to point at `REQUEST_PLUGIN_INSTALL_TOOL_NAME` instead of the removed list-available plugin tool.
- Stopped compiling/exporting removed list-available plugin modules by deleting these `handlers/mod.rs` entries:
  - `mod list_available_plugins_to_install;`
  - `pub(crate) mod list_available_plugins_to_install_spec;`
  - `pub use list_available_plugins_to_install::ListAvailablePluginsToInstallHandler;`
- Stopped compiling the stale local request-plugin spec module by deleting `pub(crate) mod request_plugin_install_spec;` from `handlers/mod.rs`.

## Verification Commands

- `rg -n "LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME|ListAvailablePluginsToInstall|list_available_plugins_to_install|request_plugin_install_spec::create_request_plugin_install_tool|create_request_plugin_install_tool\\(\\)" codex-rs\\core\\src\\tools\\handlers\\request_plugin_install.rs codex-rs\\core\\src\\tools\\handlers\\mod.rs`
  - Exit 1: no stale matches in the compiled handler/module files.
- `rg -n "mod list_available_plugins_to_install|list_available_plugins_to_install_spec|pub use list_available_plugins_to_install|request_plugin_install_spec" codex-rs\\core\\src\\tools\\handlers\\mod.rs`
  - Exit 1: removed modules are not compiled/exported from `handlers/mod.rs`.
- `rg -n "RequestPluginInstallHandler::new|RequestPluginInstallHandler" codex-rs\\core\\src\\tools`
  - Confirmed `spec_plan.rs` calls `RequestPluginInstallHandler::new(discoverable_tools)` and the handler now provides it.
- `rg -n "create_request_plugin_install_tool|collect_request_plugin_install_entries|RequestPluginInstallEntry|REQUEST_PLUGIN_INSTALL_TOOL_NAME|LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME" codex-rs\\core\\src\\tools\\handlers\\request_plugin_install.rs`
  - Confirmed the compiled handler uses the current request-plugin-install source of truth and no longer imports the list-available constant.
- `rg -n "LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME|ListAvailablePluginsToInstallResult|ListAvailablePluginsToInstallHandler|request_plugin_install_spec::create_request_plugin_install_tool" codex-rs\\core\\src\\tools\\handlers\\request_plugin_install.rs codex-rs\\core\\src\\tools\\handlers\\mod.rs codex-rs\\core\\src\\tools\\spec_plan.rs`
  - Exit 1: no stale compiled references in the handler, module tree, or call site.
- `rg -n "LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME|ListAvailablePluginsToInstallResult|ListAvailablePluginsToInstallHandler|list_available_plugins_to_install" codex-rs\\tools\\src\\lib.rs codex-rs\\tools\\src\\tool_discovery.rs`
  - Exit 1: `codex-tools` source of truth no longer exports those removed list-available symbols.
- `rg -n "LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME|ListAvailablePluginsToInstallResult|ListAvailablePluginsToInstallHandler|list_available_plugins_to_install" codex-rs\\core\\src\\tools\\handlers`
  - Remaining matches are only in retired/uncompiled local files: `list_available_plugins_to_install.rs`, `list_available_plugins_to_install_spec.rs`, and `request_plugin_install_spec.rs`.
- `git diff --stat -- codex-rs\\core\\src\\tools\\handlers\\request_plugin_install.rs codex-rs\\core\\src\\tools\\handlers\\mod.rs codex-rs\\core\\src\\tools\\handlers\\request_plugin_install_spec.rs`
  - Confirmed only `request_plugin_install.rs` and `mod.rs` changed among the owned source files.

No Cargo, Just, formatter, staging, commit, or broad build lane was run per worker rules.

## Remaining Compile Blockers Outside This Slice

From the DAB availability worker handoff, remaining broad compile blockers include unresolved imports around `hook_runtime`, `project_roots_glob_pattern`, and skill dependency functions, plus missing `Session::input_queue`, `Op::UserInput.thread_settings`, and undeclared `LocalThreadStore`.

The old list-available plugin handler/spec files still exist on disk with removed symbols, but this slice removed them from the compiled `handlers/mod.rs` module tree. They are not compile blockers unless re-exported or re-added.

## Commit Pathspec

If root verification accepts this slice, the pathspec for this worker's files is:

```powershell
git add -- codex-rs/core/src/tools/handlers/request_plugin_install.rs codex-rs/core/src/tools/handlers/mod.rs .codex/workflow/agents/plugin_tool_compile_worker.handoff.md
```

Note: these source files already contained same-file edits from other sessions, including DAB handler registration in `handlers/mod.rs` and the `AppInfo` import change in `request_plugin_install.rs`. Use hunk staging instead of the file pathspec if the plugin compile slice must be committed separately from those pre-existing edits.
