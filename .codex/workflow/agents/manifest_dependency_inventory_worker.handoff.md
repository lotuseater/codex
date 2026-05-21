# manifest_dependency_inventory_worker Handoff

Status: completed read-only manifest/dependency ownership inventory on 2026-05-21.

No source, manifest, lockfile, Bazel, or generated-schema commands were run. No files were edited except this handoff.

## Current Manifest / Lock State

- Clean in the current worktree: `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`, `MODULE.bazel`, and `MODULE.bazel.lock`.
- Root workspace wiring already includes `mcp/elicitation-api`, `thread/thread-projection-api`, the app catalog crates, session crates, turn crates, domain crates, tools-domain crates, runtime-domain crates, and thread API/store crates.
- `codex-rs/Cargo.lock` already contains entries for `codex-app-catalog-types`, `codex-mcp-elicitation-api`, `codex-thread-projection-api`, and `codex-thread-store-api`.
- `MODULE.bazel` / `MODULE.bazel.lock` do not currently show local crate identifiers for the new split crates. Bazel/BUILD ownership is therefore still likely needed before Bazel lanes are expected to pass.

## Manifest / Lock Ownership Map

Already committed foundation:

- `boundary_dependency_manifest_worker` owns the already-committed root workspace / lock foundation in commit `ed932df9565873019dbc504ebf931e3a0fedc964`.
- That foundation added root workspace and `Cargo.lock` wiring for app catalog, context/domain/runtime/session/thread/tools/turn crates, plus the later `codex-mcp-elicitation-api` and `codex-thread-projection-api` entries.
- Root remains the owner for any further `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`, `MODULE.bazel`, `MODULE.bazel.lock`, or shared BUILD/schema refresh updates.

Tracked dirty `Cargo.toml` files:

- `codex-rs/connectors/Cargo.toml`: likely `config_connectors_boundary_worker` plus `app_catalog_followup`. It replaces `codex-app-server-protocol` with `codex-app-catalog-types`; commit only with the connector source import move.
- `codex-rs/app-server/Cargo.toml`: likely `app_server_boundary_rescue`, `app_catalog_followup`, and `thread_store_integration_rescue`. It adds `codex-app-catalog-types` and `codex-thread-store-api`.
- `codex-rs/tools/Cargo.toml`: likely `app_catalog_followup` / plugin-tool lane. It adds `codex-app-catalog-types` while still retaining `codex-app-server-protocol`.
- `codex-rs/core-api/Cargo.toml`: boundary/facade cleanup lane, also called out by `bazel_lock_rescue`; it removes `codex-core` but still has other facade-risk dependencies.
- `codex-rs/ext/guardian/Cargo.toml`: extension boundary cleanup, also called out by `bazel_lock_rescue`; it removes `codex-core`.
- `codex-rs/mcp-server/Cargo.toml`: thread-store integration lane; it adds `codex-thread-store`.
- `codex-rs/thread-manager-sample/Cargo.toml`: thread-store sample/integration lane; it adds `codex-core` and `codex-thread-store`.
- `codex-rs/thread-store/Cargo.toml`: deleted old location; pair with the new `codex-rs/thread/thread-store/Cargo.toml` in one thread-store move commit.

Untracked crate manifests by likely owner:

- App catalog lane: `codex-rs/app/app-catalog-api/Cargo.toml`, `codex-rs/app/app-catalog-types/Cargo.toml`.
- Domain/foundation lane: `codex-rs/context-domain/compaction-policy/Cargo.toml`, `codex-rs/context-domain/context-budget/Cargo.toml`, `codex-rs/context-domain/history-api/Cargo.toml`, `codex-rs/context-domain/prompt-context/Cargo.toml`, `codex-rs/core-domain/types/Cargo.toml`, `codex-rs/tools-domain/tool-execution-api/Cargo.toml`, `codex-rs/tools-domain/tool-handler-api/Cargo.toml`, `codex-rs/tools-domain/tool-registry-api/Cargo.toml`, `codex-rs/runtime-domain/auth-api/Cargo.toml`, `codex-rs/runtime-domain/model-client-api/Cargo.toml`, `codex-rs/runtime-domain/runtime-ports/Cargo.toml`, `codex-rs/runtime-domain/state-db-api/Cargo.toml`, `codex-rs/runtime-domain/telemetry-api/Cargo.toml`.
- MCP protocol split lane: `codex-rs/mcp/elicitation-api/Cargo.toml`.
- Session lane: `codex-rs/session/session-api/Cargo.toml`, `codex-rs/session/session-events/Cargo.toml`, `codex-rs/session/session-factory/Cargo.toml`, `codex-rs/session/session-input/Cargo.toml`, `codex-rs/session/session-policy/Cargo.toml`, `codex-rs/session/session-runtime-api/Cargo.toml`, `codex-rs/session/session-runtime/Cargo.toml`, `codex-rs/session/session-state/Cargo.toml`.
- Thread lane: `codex-rs/thread/thread-api/Cargo.toml`, `codex-rs/thread/thread-handle-api/Cargo.toml`, `codex-rs/thread/thread-manager-api/Cargo.toml`, `codex-rs/thread/thread-projection-api/Cargo.toml`, `codex-rs/thread/thread-store-api/Cargo.toml`, `codex-rs/thread/thread-store/Cargo.toml`.
- Turn lane: `codex-rs/turn/turn-api/Cargo.toml`, `codex-rs/turn/turn-events/Cargo.toml`, `codex-rs/turn/turn-loop-api/Cargo.toml`, `codex-rs/turn/turn-loop/Cargo.toml`, `codex-rs/turn/turn-policy/Cargo.toml`, `codex-rs/turn/turn-state/Cargo.toml`, `codex-rs/turn/turn-tool-bridge/Cargo.toml`.

## Suspected Dependency Edges To Resolve

- Current `codex-rs/core/src` still imports app-server protocol DTOs even though `codex-rs/core/Cargo.toml` has no `codex-app-server-protocol` dependency:
  - `codex-rs/core/src/mcp_tool_call.rs:29-32`: `McpElicitationObjectType`, `McpElicitationSchema`, `McpServerElicitationRequest`, `McpServerElicitationRequestParams`.
  - `codex-rs/core/src/session/mod.rs:54-55`: `McpServerElicitationRequest`, `McpServerElicitationRequestParams`.
  - `codex-rs/core/src/session/tests.rs:79`: `McpElicitationSchema`.
  - `codex-rs/core/src/thread_manager.rs:18-19`: `ThreadHistoryBuilder`, `TurnStatus`.
- Preferred ownership for those DTOs remains: `codex-mcp-elicitation-api` for elicitation request/schema types and `codex-thread-projection-api` for turn/projection types. Avoid fixing this by adding `codex-app-server-protocol` back to `codex-core`.
- `codex-rs/login/Cargo.toml:15` still depends on `codex-app-server-protocol`, and login source imports `AuthMode` from it. This keeps the transitive `codex-core -> codex-login -> codex-app-server-protocol` leak alive until `AuthMode` moves through `codex-auth-api`.
- `codex-rs/core/Cargo.toml` still has high-value coupling edges into `codex-api`, `codex-connectors`, `codex-login`, and `codex-mcp`. These are broader source-ownership issues from `core_dependency_map_scout`, not manifest-only fixes.
- `codex-rs/core-api/Cargo.toml` still depends on `codex-app-server-protocol` and `codex-login`. Treat it as a facade-risk dependency: do not route core cleanup through `codex-core-api` unless the facade boundary is intentionally being repaired.
- `codex-rs/tools/Cargo.toml` now has `codex-app-catalog-types` but still has `codex-app-server-protocol`. That can be valid during migration, but should not be considered clean until the app catalog/tool source owner confirms remaining protocol types are intentional.
- The new untracked domain/session/thread/turn/tools/runtime manifests did not show suspicious direct `codex-app-server-protocol`, `codex-config`, `codex-connectors`, `codex-otel`, or `codex-core` edges in the searched `Cargo.toml` files.

## Deferred Exact Commands For Root

After source ownership is clean and no Cargo/rustc process is active:

```powershell
git status --short -- 'codex-rs/Cargo.toml' 'codex-rs/Cargo.lock' 'MODULE.bazel' 'MODULE.bazel.lock' 'codex-rs/**/Cargo.toml'
git diff --check -- codex-rs/app-server/Cargo.toml codex-rs/connectors/Cargo.toml codex-rs/core-api/Cargo.toml codex-rs/ext/guardian/Cargo.toml codex-rs/mcp-server/Cargo.toml codex-rs/thread-manager-sample/Cargo.toml codex-rs/tools/Cargo.toml codex-rs/thread-store/Cargo.toml codex-rs/thread/thread-store/Cargo.toml
just bazel-lock-update
just bazel-lock-check
```

Focused verification lanes that earlier handoffs deferred:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-config
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-connectors
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-otel
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-thread-store-api
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-thread-store
```

Schema refreshes, only if the corresponding source surface is part of the final slice:

```powershell
just write-app-server-schema
just write-config-schema
```

## Overlap Risks And Recommended Sequencing

1. Keep root `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`, `MODULE.bazel`, `MODULE.bazel.lock`, BUILD files, and schema fixtures under root ownership. Workers should not independently stage those files.
2. Commit the untracked crate manifests with their owned source files and any required BUILD/lock updates. Do not stage only source that depends on a new crate while leaving that crate manifest, root lock, or Bazel wiring out.
3. Commit `codex-rs/connectors/Cargo.toml` with the connector source import move, not by itself.
4. Commit `codex-rs/app-server/Cargo.toml` only after app catalog and thread-store app-server source ownership is settled, because the file currently carries both dependency directions.
5. Commit the old `codex-rs/thread-store/*` deletion and the new `codex-rs/thread/thread-store/*` path together. Splitting them leaves package ownership ambiguous and can break local/Bazel target discovery.
6. Resolve core source imports of app-server protocol DTOs before declaring the core split clean. Adding a temporary `codex-app-server-protocol` dependency to `codex-core` would hide the mismatch and move the architecture backward.
7. Run Bazel lock refresh/check after the final manifest and BUILD set is known. Running it before local BUILD targets are added is likely churn.
