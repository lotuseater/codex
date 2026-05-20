# bazel_lock_scout Handoff

Status: read-only Bazel and lockfile scout complete.

## Scope

Inspected:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/manifest_wiring_scout.handoff.md`
- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- `MODULE.bazel`
- `MODULE.bazel.lock`
- targeted new crate directories under `codex-rs/`

Note: the requested `codex-rs/MODULE.bazel.lock` path does not exist. The Bazel module lockfile in this checkout is the repo-root `MODULE.bazel.lock`.

No source, manifest, lockfile, BUILD, generated, staging, Cargo, Just, Bazel, or formatter action was run. This handoff is the only file written.

## Manifest Drift Observed

`codex-rs/Cargo.toml` is modified and currently adds 35 workspace members for the boundary/domain refactor:

- `app/app-catalog-api`
- `app/app-catalog-types`
- `context-domain/compaction-policy`
- `context-domain/context-budget`
- `context-domain/history-api`
- `context-domain/prompt-context`
- `core-domain/types`
- `runtime-domain/auth-api`
- `runtime-domain/model-client-api`
- `runtime-domain/runtime-ports`
- `runtime-domain/state-db-api`
- `runtime-domain/telemetry-api`
- `session/session-api`
- `session/session-events`
- `session/session-factory`
- `session/session-input`
- `session/session-policy`
- `session/session-runtime`
- `session/session-runtime-api`
- `session/session-state`
- `thread/thread-api`
- `thread/thread-handle-api`
- `thread/thread-manager-api`
- `thread/thread-store`
- `thread/thread-store-api`
- `tools-domain/tool-execution-api`
- `tools-domain/tool-handler-api`
- `tools-domain/tool-registry-api`
- `turn/turn-api`
- `turn/turn-events`
- `turn/turn-loop`
- `turn/turn-loop-api`
- `turn/turn-policy`
- `turn/turn-state`
- `turn/turn-tool-bridge`

The same manifest adds matching workspace dependency entries:

- `codex-app-catalog-api = { path = "app/app-catalog-api" }`
- `codex-app-catalog-types = { path = "app/app-catalog-types" }`
- `codex-auth-api = { path = "runtime-domain/auth-api" }`
- `codex-compaction-policy = { path = "context-domain/compaction-policy" }`
- `codex-context-budget = { path = "context-domain/context-budget" }`
- `codex-core-domain-types = { path = "core-domain/types" }`
- `codex-history-api = { path = "context-domain/history-api" }`
- `codex-model-client-api = { path = "runtime-domain/model-client-api" }`
- `codex-prompt-context = { path = "context-domain/prompt-context" }`
- `codex-runtime-ports = { path = "runtime-domain/runtime-ports" }`
- `codex-session-api = { path = "session/session-api" }`
- `codex-session-events = { path = "session/session-events" }`
- `codex-session-factory = { path = "session/session-factory" }`
- `codex-session-input = { path = "session/session-input" }`
- `codex-session-policy = { path = "session/session-policy" }`
- `codex-session-runtime = { path = "session/session-runtime" }`
- `codex-session-runtime-api = { path = "session/session-runtime-api" }`
- `codex-session-state = { path = "session/session-state" }`
- `codex-state-db-api = { path = "runtime-domain/state-db-api" }`
- `codex-telemetry-api = { path = "runtime-domain/telemetry-api" }`
- `codex-thread-api = { path = "thread/thread-api" }`
- `codex-thread-handle-api = { path = "thread/thread-handle-api" }`
- `codex-thread-manager-api = { path = "thread/thread-manager-api" }`
- `codex-thread-store = { path = "thread/thread-store" }`
- `codex-thread-store-api = { path = "thread/thread-store-api" }`
- `codex-tool-execution-api = { path = "tools-domain/tool-execution-api" }`
- `codex-tool-handler-api = { path = "tools-domain/tool-handler-api" }`
- `codex-tool-registry-api = { path = "tools-domain/tool-registry-api" }`
- `codex-turn-api = { path = "turn/turn-api" }`
- `codex-turn-events = { path = "turn/turn-events" }`
- `codex-turn-loop = { path = "turn/turn-loop" }`
- `codex-turn-loop-api = { path = "turn/turn-loop-api" }`
- `codex-turn-policy = { path = "turn/turn-policy" }`
- `codex-turn-state = { path = "turn/turn-state" }`
- `codex-turn-tool-bridge = { path = "turn/turn-tool-bridge" }`

The existing `codex-thread-store` workspace path moved from `thread-store` to `thread/thread-store`. This is a dependency location move, not a new lockfile package name.

Already-present current workspace crates from the agent/analytics slice are also wired in `Cargo.toml` and `Cargo.lock`:

- `codex-analytics = { path = "analytics" }`
- `codex-agent-graph-store = { path = "agent-graph-store" }`
- `codex-agent-identity = { path = "agent-identity" }`
- `codex-agent-policy = { path = "agent-policy" }`

## Lockfile Drift Observed

`codex-rs/Cargo.lock` is modified. The diff adds 34 internal `codex-*` packages and no new third-party package entries were observed in the HEAD-to-working-tree package-name comparison.

Added `Cargo.lock` packages:

- `codex-app-catalog-api`
- `codex-app-catalog-types`
- `codex-auth-api`
- `codex-compaction-policy`
- `codex-context-budget`
- `codex-core-domain-types`
- `codex-history-api`
- `codex-model-client-api`
- `codex-prompt-context`
- `codex-runtime-ports`
- `codex-session-api`
- `codex-session-events`
- `codex-session-factory`
- `codex-session-input`
- `codex-session-policy`
- `codex-session-runtime`
- `codex-session-runtime-api`
- `codex-session-state`
- `codex-state-db-api`
- `codex-telemetry-api`
- `codex-thread-api`
- `codex-thread-handle-api`
- `codex-thread-manager-api`
- `codex-thread-store-api`
- `codex-tool-execution-api`
- `codex-tool-handler-api`
- `codex-tool-registry-api`
- `codex-turn-api`
- `codex-turn-events`
- `codex-turn-loop`
- `codex-turn-loop-api`
- `codex-turn-policy`
- `codex-turn-state`
- `codex-turn-tool-bridge`

Notable first-party dependency movement visible in the lock diff:

- `codex-app-server` gained `codex-app-catalog-types`, `codex-memories-extension`, and `codex-thread-store-api`.
- `codex-connectors` moved from `codex-app-server-protocol` to `codex-app-catalog-types`.
- `codex-core` moved from `codex-app-server-protocol` to `codex-app-catalog-types`, and from `codex-thread-store` to `codex-thread-store-api`.
- `codex-core-api` dropped `codex-core`.
- `codex-thread-store` gained `codex-thread-store-api`.

Two prepared manifests called out by `manifest_wiring_scout` still exist on disk but are not wired into the root workspace manifest or lockfile:

- `codex-rs/mcp/elicitation-api/Cargo.toml`
- `codex-rs/thread/thread-projection-api/Cargo.toml`

Expected future root wiring from that handoff remains:

- `mcp/elicitation-api`
- `codex-mcp-elicitation-api = { path = "mcp/elicitation-api" }`
- `thread/thread-projection-api`
- `codex-thread-projection-api = { path = "thread/thread-projection-api" }`

## Bazel Follow-Up

`MODULE.bazel` uses the Rust crate extension against the Rust workspace files:

- `cargo_lock = "//codex-rs:Cargo.lock"`
- `cargo_toml = "//codex-rs:Cargo.toml"`

Because `Cargo.toml` and `Cargo.lock` changed substantially, the repo-root `MODULE.bazel.lock` is likely stale even though it currently has no working-tree diff. A later integration slice should refresh and check it after the manifest shape is stable.

Likely `BUILD.bazel` follow-up is required for almost all newly added workspace members. Among the 35 newly added workspace members, only `codex-rs/thread/thread-store/BUILD.bazel` was found. The other 34 added workspace members did not have a `BUILD.bazel` file in their crate directory during this scout pass.

The already-present agent/analytics crates do have `Cargo.toml` and lockfile entries; keep their Bazel package files in scope if a full crate-universe or BUILD audit is done, but the current high-risk missing-BUILD set is the newly added boundary/domain crates above.

The two not-yet-wired prepared crates also lacked `BUILD.bazel` files during this pass:

- `codex-rs/mcp/elicitation-api/BUILD.bazel`
- `codex-rs/thread/thread-projection-api/BUILD.bazel`

Targeted scans across the new boundary/domain directories did not find:

- `include_str!`
- `include_bytes!`
- `sqlx::migrate!`
- `env!("CARGO_MANIFEST_DIR")`
- `build.rs`

So no compile-time source-tree data wiring was directly observed from those patterns. The likely Bazel work is package target/dependency wiring and crate lock refresh, not `compile_data` or `build_script_data` for the scanned new crates.

## Commands For Later

Do not run these from this scout pass. Once root decides the manifest slice is stable, run from repo root:

```powershell
just bazel-lock-update
just bazel-lock-check
```

If BUILD files are added or generated for the new crates, include those BUILD updates in the same coherent Bazel wiring slice as the refreshed root `MODULE.bazel.lock`.

## Commit Readiness

Not commit-ready as a Bazel/lockfile slice yet.

Reasons:

- `Cargo.toml` and `Cargo.lock` have broad intentional refactor drift.
- Root `MODULE.bazel.lock` has not been refreshed after the manifest/lockfile changes.
- Most newly added workspace members do not yet have crate-local `BUILD.bazel` targets.
- `mcp/elicitation-api` and `thread/thread-projection-api` still have prepared manifests but remain unwired in root `Cargo.toml` / `Cargo.lock`.
- Broad Cargo/Bazel verification was intentionally out of scope for this read-only scout.

