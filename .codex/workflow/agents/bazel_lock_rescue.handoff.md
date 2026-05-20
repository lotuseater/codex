# bazel_lock_rescue Handoff

Status: completed read-only rescue pass on 2026-05-20. The only intended repo
output is this handoff; no Cargo, Just, Bazel, formatter, staging, commit, or
build command was run.

## Scope Read

Workflow inputs read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/agents/manifest_wiring_scout.handoff.md`
- `.codex/workflow/agents/protocol_schema_scout.handoff.md`

Workspace, lock, and Bazel control files read:

- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- `MODULE.bazel`
- `MODULE.bazel.lock`
- `justfile`
- `defs.bzl`
- `codex-rs/core-api/BUILD.bazel`
- `codex-rs/app-server/BUILD.bazel`
- `codex-rs/thread-store/BUILD.bazel` via git diff
- `codex-rs/thread/thread-store/BUILD.bazel`

Dirty or prepared crate manifests read:

- `codex-rs/app-server/Cargo.toml`
- `codex-rs/connectors/Cargo.toml`
- `codex-rs/core-api/Cargo.toml`
- `codex-rs/core/Cargo.toml`
- `codex-rs/ext/guardian/Cargo.toml`
- `codex-rs/mcp-server/Cargo.toml`
- `codex-rs/thread-manager-sample/Cargo.toml`
- `codex-rs/thread-store/Cargo.toml` via git diff
- `codex-rs/thread/thread-store/Cargo.toml`
- `codex-rs/app/app-catalog-api/Cargo.toml`
- `codex-rs/app/app-catalog-types/Cargo.toml`
- `codex-rs/context-domain/compaction-policy/Cargo.toml`
- `codex-rs/context-domain/context-budget/Cargo.toml`
- `codex-rs/context-domain/history-api/Cargo.toml`
- `codex-rs/context-domain/prompt-context/Cargo.toml`
- `codex-rs/core-domain/types/Cargo.toml`
- `codex-rs/mcp/elicitation-api/Cargo.toml`
- `codex-rs/runtime-domain/auth-api/Cargo.toml`
- `codex-rs/runtime-domain/model-client-api/Cargo.toml`
- `codex-rs/runtime-domain/runtime-ports/Cargo.toml`
- `codex-rs/runtime-domain/state-db-api/Cargo.toml`
- `codex-rs/runtime-domain/telemetry-api/Cargo.toml`
- `codex-rs/session/session-api/Cargo.toml`
- `codex-rs/session/session-events/Cargo.toml`
- `codex-rs/session/session-factory/Cargo.toml`
- `codex-rs/session/session-input/Cargo.toml`
- `codex-rs/session/session-policy/Cargo.toml`
- `codex-rs/session/session-runtime-api/Cargo.toml`
- `codex-rs/session/session-runtime/Cargo.toml`
- `codex-rs/session/session-state/Cargo.toml`
- `codex-rs/thread/thread-api/Cargo.toml`
- `codex-rs/thread/thread-handle-api/Cargo.toml`
- `codex-rs/thread/thread-manager-api/Cargo.toml`
- `codex-rs/thread/thread-projection-api/Cargo.toml`
- `codex-rs/thread/thread-store-api/Cargo.toml`
- `codex-rs/tools-domain/tool-execution-api/Cargo.toml`
- `codex-rs/tools-domain/tool-handler-api/Cargo.toml`
- `codex-rs/tools-domain/tool-registry-api/Cargo.toml`
- `codex-rs/turn/turn-api/Cargo.toml`
- `codex-rs/turn/turn-events/Cargo.toml`
- `codex-rs/turn/turn-loop-api/Cargo.toml`
- `codex-rs/turn/turn-loop/Cargo.toml`
- `codex-rs/turn/turn-policy/Cargo.toml`
- `codex-rs/turn/turn-state/Cargo.toml`
- `codex-rs/turn/turn-tool-bridge/Cargo.toml`

## Manifest Wiring Status

`codex-rs/Cargo.toml` is dirty and already wires most prepared boundary/domain
crates as both `[workspace].members` and `[workspace.dependencies]`. Current
root wiring includes app catalog, context domain, core domain types, runtime
domain, session, thread API/store, tools domain, and turn crates. `codex-thread-store`
has moved from the old root `thread-store` path to `thread/thread-store` while
keeping the package name `codex-thread-store`.

Two prepared boundary crates are present on disk but not yet root-wired:

- `codex-rs/mcp/elicitation-api/Cargo.toml`
  - package: `codex-mcp-elicitation-api`
  - lib crate: `codex_mcp_elicitation_api`
  - deps: `serde`, `serde_json`
  - missing root member: `"mcp/elicitation-api"`
  - missing workspace dep: `codex-mcp-elicitation-api = { path = "mcp/elicitation-api" }`
- `codex-rs/thread/thread-projection-api/Cargo.toml`
  - package: `codex-thread-projection-api`
  - lib crate: `codex_thread_projection_api`
  - deps: `codex-protocol`, `serde`
  - missing root member: `"thread/thread-projection-api"`
  - missing workspace dep: `codex-thread-projection-api = { path = "thread/thread-projection-api" }`

Current dirty downstream manifest deltas observed:

- `app-server/Cargo.toml` adds `codex-app-catalog-types` and
  `codex-thread-store-api`.
- `connectors/Cargo.toml` replaces `codex-app-server-protocol` with
  `codex-app-catalog-types`.
- `core/Cargo.toml` replaces `codex-app-server-protocol` with
  `codex-app-catalog-types`, and replaces `codex-thread-store` with
  `codex-thread-store-api`.
- `core-api/Cargo.toml` removes `codex-core`.
- `ext/guardian/Cargo.toml` removes `codex-core`.
- `mcp-server/Cargo.toml` adds `codex-thread-store`.
- `thread-manager-sample/Cargo.toml` adds `codex-core` and `codex-thread-store`
  while removing the old comment that limited it to one Codex workspace dep.
- `thread-store/Cargo.toml` and `thread-store/BUILD.bazel` are deleted in the
  working tree; matching files exist under `thread/thread-store/`.

## Lock And Bazel Impact

`codex-rs/Cargo.lock` is dirty. Its current diff appears limited to local
workspace package entries and local dependency edges; no added external
`source` or `checksum` lines were observed.

The lock already contains entries for the root-wired new crates:

- `codex-app-catalog-api`, `codex-app-catalog-types`
- `codex-compaction-policy`, `codex-context-budget`, `codex-history-api`,
  `codex-prompt-context`, `codex-core-domain-types`
- `codex-auth-api`, `codex-model-client-api`, `codex-runtime-ports`,
  `codex-state-db-api`, `codex-telemetry-api`
- `codex-session-api`, `codex-session-events`, `codex-session-factory`,
  `codex-session-input`, `codex-session-policy`, `codex-session-runtime-api`,
  `codex-session-runtime`, `codex-session-state`
- `codex-thread-api`, `codex-thread-handle-api`, `codex-thread-manager-api`,
  `codex-thread-store-api`, `codex-thread-store`
- `codex-tool-execution-api`, `codex-tool-handler-api`,
  `codex-tool-registry-api`
- `codex-turn-api`, `codex-turn-events`, `codex-turn-loop-api`,
  `codex-turn-loop`, `codex-turn-policy`, `codex-turn-state`,
  `codex-turn-tool-bridge`

The lock does not contain `codex-mcp-elicitation-api` or
`codex-thread-projection-api`, matching their missing root workspace wiring.

`MODULE.bazel` and `MODULE.bazel.lock` are clean. `MODULE.bazel` uses
`crate.from_cargo(cargo_lock = "//codex-rs:Cargo.lock", cargo_toml =
"//codex-rs:Cargo.toml", ...)`, so any finalized change to the Rust workspace
manifest or lock must be followed by a Bazel module lock refresh. A direct
search of `MODULE.bazel.lock` found no matches for the new local crate package
names checked (`codex-app-catalog-*`, `codex-thread-store*`,
`codex-mcp-elicitation-api`, `codex-thread-projection-api`).

Bazel BUILD coverage is incomplete for the new crates. Only the moved
`codex-rs/thread/thread-store/BUILD.bazel` was found among the new crate paths.
The following new crate directories currently have no `BUILD.bazel`:

- `app/app-catalog-api`, `app/app-catalog-types`
- `context-domain/compaction-policy`, `context-domain/context-budget`,
  `context-domain/history-api`, `context-domain/prompt-context`
- `core-domain/types`
- `mcp/elicitation-api`
- `runtime-domain/auth-api`, `runtime-domain/model-client-api`,
  `runtime-domain/runtime-ports`, `runtime-domain/state-db-api`,
  `runtime-domain/telemetry-api`
- `session/session-api`, `session/session-events`, `session/session-factory`,
  `session/session-input`, `session/session-policy`,
  `session/session-runtime-api`, `session/session-runtime`,
  `session/session-state`
- `thread/thread-api`, `thread/thread-handle-api`,
  `thread/thread-manager-api`, `thread/thread-projection-api`,
  `thread/thread-store-api`
- `tools-domain/tool-execution-api`, `tools-domain/tool-handler-api`,
  `tools-domain/tool-registry-api`
- `turn/turn-api`, `turn/turn-events`, `turn/turn-loop-api`,
  `turn/turn-loop`, `turn/turn-policy`, `turn/turn-state`,
  `turn/turn-tool-bridge`

## Commands Root Should Run Later

Prerequisites before running lock/schema refresh:

- Finish root wiring for `codex-mcp-elicitation-api` and
  `codex-thread-projection-api` if their source imports or downstream
  `{ workspace = true }` entries are part of the integration slice.
- Refresh `codex-rs/Cargo.lock` after the root manifest is final, so it contains
  every workspace member that will be committed.
- Add/relocate required `BUILD.bazel` files for new local crate targets before
  expecting Bazel build/test lanes to see them.
- Confirm no other Cargo/Bazel process is active before lock-refresh work.

Then from the repo root:

```powershell
just bazel-lock-update
just bazel-lock-check
```

If app-server protocol v2 wire shapes or exported TS/schema fixtures changed in
the same source slice, also run:

```powershell
just write-app-server-schema
```

`protocol_schema_scout` specifically calls out permissions/config protocol
surface as schema-sensitive and recommends schema refresh after active protocol
edits are green. If `ConfigToml` or nested config types changed, root must also
run the repo-required config schema refresh separately.

## Risks If Source Is Committed First

- Source that imports `codex-mcp-elicitation-api` or
  `codex-thread-projection-api` before root wiring lands will fail Cargo
  metadata/build with missing workspace dependency keys or missing workspace
  members.
- `MODULE.bazel.lock` can become stale relative to `codex-rs/Cargo.toml` and
  `codex-rs/Cargo.lock`, causing `just bazel-lock-check` / CI to fail even if
  Cargo metadata succeeds.
- Bazel lanes can fail to resolve local crate targets because most new crate
  directories currently lack `BUILD.bazel`.
- Protocol/schema source can compile while generated schema fixtures stay stale,
  leaving app-server protocol tests, TS consumers, or review diffs inconsistent.
- A partial commit with source changes but not the root-owned manifests and
  locks would make later workers chase dependency errors that are not owned by
  their source files.

## Commit Grouping Recommendation

Root should own one coherent manifest/lock/Bazel refresh slice before source
integration commits are treated as complete. That slice should include:

- `codex-rs/Cargo.toml`
- all crate `Cargo.toml` files whose packages are introduced, moved, or whose
  dependencies changed
- `codex-rs/Cargo.lock`
- `MODULE.bazel.lock`
- required `BUILD.bazel` additions/moves for the new local crates
- generated schema fixtures when protocol/config wire surface changes are in
  the same source slice

Prefer committing that foundation with explicit pathspec staging after refresh
commands pass. Then commit protocol/app/core source integration in smaller
verified slices. Do not stage only worker source files that depend on the new
crates while leaving the root manifest, Cargo lock, Bazel lock, BUILD files, or
schema fixtures stale.
