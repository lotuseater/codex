# SOLID Refactor Wave 20 Dependency Boundary Checker Scout Handoff

Classification: read-only scout / accepted
Date: 2026-05-22

## Scope

- Read-only inspection only. No source edits, builds, Cargo/Rust tests, formatters, schema generation, lock refresh, deploy, activation, staging, or commits were run.
- First reads covered the SOLID orchestration handoff, current SOLID plan/review docs, and the fresh wave19 handoffs.

## Allowed Verification Run

- `scripts/check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json`
  - Result: passed with exit 0 and emitted no violations.
  - Important: this only proves the currently encoded checker policies pass. It does not prove `codex-core -> codex-tools` is clean yet, because `codex-rs/core/Cargo.toml` still contains `codex-tools = { workspace = true }`.
- `rg -n "codex_tools" codex-rs/core/src codex-rs/core/tests`
  - Result: exit 0 with remaining matches.
  - Count from the current tree: 191 matching lines across 53 files.
  - Split: 36 source files and 17 test files; 32 of the files are under `codex-rs/core/src/tools/handlers` (24 source, 8 tests).

## Fresh Wave 19 Context

- `solid_refactor_wave19_core_tools_client_goals_boundary_worker.handoff.md`: classification `repair-needed`; notes `codex-rs/core/Cargo.toml` still has direct `codex-tools` because other core modules/tests still reference `codex_tools`.
- `solid_refactor_wave19_shell_unified_exec_boundary_worker.handoff.md`: classification `accepted`; shell/unified-exec slice moved some execution types, but explicitly leaves non-shell handler specs outside scope.
- `solid_refactor_wave19_agents_runtime_split_worker.handoff.md` and `solid_refactor_wave19_code_mode_tests_split_worker.handoff.md`: accepted slices, with commit/staging deferred to root or a clean commit pass.
- `solid_refactor_wave19_core_tests_support_dependency_worker.handoff.md`, `solid_refactor_wave19_rmcp_client_tests_split_worker.handoff.md`, and `solid_refactor_wave19_search_tool_tests_split_worker.handoff.md`: root-wiring-needed slices.
- `solid_refactor_wave19_commit_integrity_worker.handoff.md`: partial; keep generated/schema/lock/deploy/activation work deferred until source ownership and verification stabilize.

## Remaining Boundary Gaps

### 1. Core Runtime And Tool-Orchestration Imports

Needs: source worker first.

Exact modules still importing `codex_tools` outside handler specs:

- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/unified_exec/process_manager.rs`
- `codex-rs/core/src/tools/context.rs`
- `codex-rs/core/src/tools/mod.rs`
- `codex-rs/core/src/tools/registry.rs`
- `codex-rs/core/src/tools/runtimes/shell/zsh_fork_backend.rs`
- `codex-rs/core/src/tools/runtimes/unified_exec.rs`
- `codex-rs/core/src/tools/spec.rs`
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/spec_plan_types.rs`
- `codex-rs/core/src/tools/tool_family/shell.rs`
- `codex-rs/core/src/tools/tool_search_entry.rs`

Observed symbols include `ToolName`, `ShellCommandBackendConfig`, `UnifiedExecShellMode`, `ZshForkConfig`, `ToolExecutor`, `ToolExposure`, `LoadableToolSpec`, `ResponsesApiNamespaceTool`, `ToolUserShellType`, `ToolEnvironmentMode`, `ToolsConfig`, `ToolSearchSourceInfo`, `TOOL_SEARCH_TOOL_NAME`, and `default_namespace_description`.

Some already have domain API homes:

- `ToolName`, `ShellCommandBackendConfig`, `UnifiedExecShellMode`, `ZshForkConfig`, `JsonToolOutput`, and `FunctionCallError` are available from `codex-tool-execution-api`.
- `LoadableToolSpec`, `ResponsesApiTool`, `ResponsesApiNamespace`, `ResponsesApiNamespaceTool`, `JsonSchema`, `ToolSpec`, and `ToolExposure` are available from `codex-tool-registry-api`.

Remaining symbols such as `ToolsConfig`, `ToolUserShellType`, `ToolEnvironmentMode`, `ToolSearchSourceInfo`, `RequestPluginInstallEntry`, tool-name constants, and plugin-install discovery helpers appear to still live only in `codex-tools`; these need either promotion into a domain API crate or a new narrow tools-definition/config API before `codex-core` can drop `codex-tools`.

### 2. Core Tool Handler/Spec Imports

Needs: source worker, probably split by handler family.

Handler/spec source files still importing `codex_tools`:

- `codex-rs/core/src/tools/handlers/agent_jobs_spec.rs`
- `codex-rs/core/src/tools/handlers/apply_patch_spec.rs`
- `codex-rs/core/src/tools/handlers/cognos_ops.rs`
- `codex-rs/core/src/tools/handlers/context_ops.rs`
- `codex-rs/core/src/tools/handlers/context_ops/workflow_batch.rs`
- `codex-rs/core/src/tools/handlers/dynamic.rs`
- `codex-rs/core/src/tools/handlers/extension_tools.rs`
- `codex-rs/core/src/tools/handlers/first_moves.rs`
- `codex-rs/core/src/tools/handlers/list_available_plugins_to_install.rs`
- `codex-rs/core/src/tools/handlers/list_available_plugins_to_install_spec.rs`
- `codex-rs/core/src/tools/handlers/mcp.rs`
- `codex-rs/core/src/tools/handlers/mcp_resource_spec.rs`
- `codex-rs/core/src/tools/handlers/multi_agents.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_spec.rs`
- `codex-rs/core/src/tools/handlers/plan_spec.rs`
- `codex-rs/core/src/tools/handlers/repo_context_scout.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install_spec.rs`
- `codex-rs/core/src/tools/handlers/request_user_input_spec.rs`
- `codex-rs/core/src/tools/handlers/shell_spec.rs`
- `codex-rs/core/src/tools/handlers/test_sync_spec.rs`
- `codex-rs/core/src/tools/handlers/tool_search.rs`
- `codex-rs/core/src/tools/handlers/tool_search_spec.rs`
- `codex-rs/core/src/tools/handlers/view_image_spec.rs`

Adjacent handler tests still importing `codex_tools`:

- `codex-rs/core/src/tools/handlers/agent_jobs_spec_tests.rs`
- `codex-rs/core/src/tools/handlers/dynamic_tests.rs`
- `codex-rs/core/src/tools/handlers/mcp_resource_spec_tests.rs`
- `codex-rs/core/src/tools/handlers/mcp_search_tests.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_spec_tests.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install_tests.rs`
- `codex-rs/core/src/tools/handlers/request_user_input_spec_tests.rs`
- `codex-rs/core/src/tools/handlers/test_sync_spec_tests.rs`

Most schema/Responses API imports can likely switch to `codex-tool-registry-api`. Concrete factory functions and constants such as `create_*_tool`, `*_TOOL_NAME`, and discovery helpers are the harder boundary: either move their neutral spec-building pieces out of `codex-tools`, or introduce a narrow API crate that `codex-core` can depend on without depending on the concrete `codex-tools` crate.

### 3. Core Tests

Needs: source worker after the source API direction is decided.

Non-handler test modules still importing `codex_tools`:

- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/session/tests/guardian_tests.rs`
- `codex-rs/core/src/session/tests/policy_permission_tests.rs`
- `codex-rs/core/src/session/tests/turn_flow_tests.rs`
- `codex-rs/core/src/tools/context_tests.rs`
- `codex-rs/core/src/tools/hosted_spec_tests.rs`
- `codex-rs/core/src/tools/router_tests.rs`
- `codex-rs/core/src/tools/spec_plan_tests.rs`
- `codex-rs/core/src/tools/spec_tests.rs`

Do this after the production imports move, so tests can follow the final domain API rather than locking in temporary compatibility paths.

### 4. Manifest Dependency

Needs: manifest worker after source refs are gone.

- `codex-rs/core/Cargo.toml:596` still has `codex-tools = { workspace = true }`.
- `codex-rs/core/Cargo.toml:593-594` already has `codex-tool-execution-api` and `codex-tool-registry-api`, so many replacement imports should not require adding new dependencies.
- Do not remove the `codex-tools` dependency until `rg -n "codex_tools" codex-rs/core/src codex-rs/core/tests` is zero or all remaining references are intentionally outside `codex-core`.

### 5. Checker Coverage Gap

Needs: source worker after cleanup, then manifest worker/commit worker.

The current checker passed even though `codex-core` still depends on `codex-tools`. After the source and manifest cleanup, extend `scripts/check-cargo-dependency-boundaries.ps1 -SolidRefactor` so `codex-core -> codex-tools` fails both at manifest level and source-import level (`codex_tools::`). Adding this before cleanup would intentionally fail the current tree.

## Ranked Next Actions

1. Source worker: move/promote remaining `codex-tools`-only domain-neutral symbols (`ToolsConfig`, `ToolUserShellType`, `ToolEnvironmentMode`, `ToolSearchSourceInfo`, `RequestPluginInstallEntry`, tool-name constants, and plugin-install discovery helpers) into an existing domain API crate or a new narrow API crate, then update the core runtime/tool-orchestration files listed in gap 1.
2. Source worker: clean handler/spec imports in `codex-rs/core/src/tools/handlers`, using `codex-tool-registry-api` and `codex-tool-execution-api` where the target type already exists, and moving only neutral spec-building functions/constants out of `codex-tools`.
3. Source worker: update core tests to use the final API crates and rerun the same targeted `rg` command until it returns zero for `codex-rs/core/src` and `codex-rs/core/tests`.
4. Manifest worker: remove `codex-tools` from `codex-rs/core/Cargo.toml` only after the source grep is clean. Dependency/lock verification is deferred to root because this scout was not allowed to run lock refresh or builds.
5. Source/checker worker: add the explicit checker policy that prevents `codex-core` from reintroducing `codex-tools`.
6. Commit worker: once the boundary grep is clean and the manifest is updated, stage only coherent verified slices. The tree is broadly dirty from parallel wave work, so avoid staging unrelated generated/schema/lock/deploy changes unless root explicitly includes them.

## Verification Not Run

- No Cargo/Rust builds or tests.
- No `just fmt`, `just fix`, schema generation, Bazel, lock refresh, release build, deploy, activation, staging, or commit.
