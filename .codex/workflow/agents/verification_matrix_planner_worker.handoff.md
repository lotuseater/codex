# verification_matrix_planner_worker handoff

Date: 2026-05-21

## Status

- Completed deferred verification planning only.
- No Cargo, rustc, just, Bazel, build script, test script, schema generation,
  lockfile generation, release, build, check, or verification command was run by
  this worker.
- No source files were edited. The only write from this worker is this handoff.
- Read the root SOLID refactor handoff, all available agent handoff headings/key
  verification lines, `.cargo/config.toml`, `scripts/build-local-codex.ps1`,
  `scripts/test-local-codex-release.ps1`, and the repo root `justfile`.
- `codex-rs/justfile` does not exist in this checkout. The repo root `justfile`
  sets `working-directory := "codex-rs"` and owns the just commands below.

## Global Preconditions

- Current no-build/refactor-first phase has ended.
- Source ownership is clean: root has decided which worker owns each dirty path,
  no slice depends on unstaged changes from another slice, and root has reviewed
  the missing/not-yet-written resume handoffs.
- No active `cargo.exe`, `rustc.exe`, linker, Bazel, or build-script lane is
  using this checkout.
- Root has enough disk and memory for a release-profile lane. This checkout is
  release-only locally; do not use debug-profile cargo lanes.
- `.cargo/config.toml` release profile remains the shared local profile:
  `lto=off`, `codegen-units=256`, `opt-level=1`, `debug=0`,
  `strip=symbols`, `incremental=false`, `split-debuginfo=off`.
- Schema and lockfile generation waits until API, config, manifest, and crate
  shape are stable.

## Staged Verification Matrix

### Stage 0 - Exit No-Build Safely

Prerequisites:

- Root declares source ownership clean enough to run status/preflight commands.
- No worker is still expected to edit the same files.

Commands for root:

```powershell
git status --short
Get-Process cargo,rustc,link,bazel -ErrorAction SilentlyContinue
powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode Status
```

If `codex-rs\target\debug` exists and no build process is active, cleanup should
run before release verification:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode CleanSafe -CleanTestArtifacts
```

### Stage 1 - Format and Static Shape

Prerequisites:

- Rust edit batches for the active slices have settled.
- Root has decided whether schema/lockfile paths are intentionally dirty or
  should remain untouched until Stage 3.

Commands for root:

```powershell
just fmt
git diff --check
```

If root wants a scoped static scan before tests, use path-specific `rg` checks
from the worker handoffs rather than broad source searches that restart design
discussion.

### Stage 2 - Smallest Useful Release Checks By Slice

Run these one slice at a time. Do not start a second release lane while one is
active.

#### 2A - Known Compile-Gate First: `codex-otel`

Prerequisites:

- `codex_otel_compile_followup_worker` source is integrated.
- No active release lane is running.

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-otel
```

Reason for first position: multiple core worker handoffs reported that
`codex-core` release checks did not reach owned code because `codex-otel` was
blocking the dependency graph.

#### 2B - Config and Connectors Boundary

Prerequisites:

- `config_connectors_boundary_worker` source is integrated.
- Root has decided whether manifest/root dependency edits belong to this slice
  or a separate manifest slice.

Commands:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-config
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-connectors
```

If `ConfigToml` or nested config types changed, defer schema generation to
Stage 3, then rerun `codex-config` afterward.

#### 2C - Manifest and New Boundary Crates

Prerequisites:

- `boundary_dependency_manifest_worker` changes and any root manifest decisions
  are integrated.
- Root has confirmed which prepared crates are actually wired into the workspace.

Small package checks:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core-api
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core-domain-types
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-thread-api
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-thread-handle-api
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-thread-store-api
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-thread-manager-api
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-thread-store
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-thread-projection-api
```

Only run crates that exist and are wired in root `codex-rs/Cargo.toml` after
the refactor settles.

#### 2D - Core Config/Permissions, Session/Thread, and Tools Slices

Prerequisites:

- Stage 2A is green.
- Core worker slices are integrated without cross-slice dirty dependencies.
- Root accepts that `codex-core --lib` release tests are intentionally broad
  enough to require `-AllowBroadCoreLibUnitTests`.

Smallest compile gate:

```powershell
Push-Location codex-rs
cargo check -p codex-core --release --lib
Pop-Location
```

Focused release test filters after the compile gate:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter config -AllowBroadCoreLibUnitTests
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter permissions -AllowBroadCoreLibUnitTests
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter thread -AllowBroadCoreLibUnitTests
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter session -AllowBroadCoreLibUnitTests
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter tools -AllowBroadCoreLibUnitTests
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter spec_plan -AllowBroadCoreLibUnitTests
```

Integration targets likely needed for the split:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs '--test=permissions'
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs '--test=responses_headers'
```

Before releasing the permissions lane, review the `request_permissions.rs`
platform gate called out in the root handoff, then run the `permissions`
integration target above.

#### 2E - App, Thread, App-Server, and Protocol Boundary

Prerequisites:

- App-server/protocol boundary source is integrated.
- App/thread projection crates are wired or explicitly removed from the slice.
- Schema generation has not been run yet unless Stage 3 has already stabilized
  and regenerated fixtures.

Boundary package checks:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-protocol
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-catalog-types
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-catalog-api
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server-protocol
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server
```

App-server v2/thread focused checks:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server -Filter v2
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server -Filter thread
```

If root integrates MCP elicitation, tool/plugin, hook, or skill boundary slices,
add the owning packages before broad app-server tests:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-mcp-elicitation-api
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-tool-registry-api
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-tool-handler-api
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-tool-execution-api
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-tools
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-tool-schema
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core-plugins
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-plugin
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-hooks
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-skills
```

### Stage 3 - Schema and Lockfile Generation

Prerequisites:

- Stages 2A through the relevant owning package checks are green.
- Source, API payloads, config types, and manifest shape are stable.
- Root has decided that generated fixture/lockfile changes belong in the same
  coherent slice.

Commands:

```powershell
just write-config-schema
just write-app-server-schema
just write-app-server-schema --experimental
just bazel-lock-update
just bazel-lock-check
```

Run `just write-config-schema` only if `ConfigToml` or nested config types
changed.

Run `just write-app-server-schema` when v2 app-server protocol API behavior or
fixtures changed. Add `--experimental` only when experimental API fixtures are
affected.

Run `just bazel-lock-update` only after `Cargo.toml`, `Cargo.lock`, workspace
membership, or dependency graph changes stabilize. Follow with
`just bazel-lock-check` before committing.

If hook schema behavior changes, add:

```powershell
just write-hooks-schema
```

### Stage 4 - Post-Generation Focused Reruns

Prerequisites:

- Stage 3 generated artifacts are reviewed and either accepted or reverted by
  root.
- No generated artifact is stale relative to source.

Rerun the affected package checks from Stage 2. At minimum, if schema or lock
artifacts changed:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-config
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server-protocol
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server
```

If core integration split files or Bazel BUILD files changed, then add the
Cargo integration target first:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs '--test=responses_headers'
```

Then, only if Bazel wiring changed and the release Cargo lane is green:

```powershell
bazel test //codex-rs/core:core-all-test //codex-rs/core:core-responses_headers-test
```

### Stage 5 - Broader Release Checks

Prerequisites:

- Focused owning package checks are green.
- Schema and lockfile regeneration is complete or confirmed unnecessary.
- Root has committed or otherwise isolated coherent verified slices so broad
  failures are attributable.

Commands:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -AllowBroadCoreLibUnitTests
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server-protocol
```

Only after those pass should root consider a full release workspace suite, and
only with explicit acceptance of the time/disk cost:

```powershell
Push-Location codex-rs
cargo test --release
Pop-Location
```

### Stage 6 - Final Local Build and Deploy Validation

Prerequisites:

- Stage 5 is green or root has explicitly accepted the remaining external
  blocker.
- No active build process is using `target\release`.
- Disk/memory status is acceptable.

Preferred command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode FastRelease
```

Lower-memory fallback:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode LowMemRelease
```

After deploy, confirm the installed launcher resolves to the built version:

```powershell
codex --version
```

If a build succeeds but deploy is intentionally skipped or interrupted, root can
use the script's deploy-only lane after confirming the release binary:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode DeployOnly
```

### Stage 7 - Compaction Validation Last

Prerequisites:

- Final source, generated artifacts, and deployment behavior are stable.
- Stage 6 has passed.
- Root is now validating compaction behavior rather than still changing
  protocol/core ownership boundaries.

Core compaction integration target:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs '--test=compact'
```

If new unit tests are added under core compaction modules, add the filtered
release lib lane:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter compact -AllowBroadCoreLibUnitTests
```

Keep compaction validation last because the accepted compaction plan depends on
final prompt/output behavior. Running it before schema, protocol, core, and
deployment shape stabilize risks validating a transient output path.

## Commands Intentionally Deferred Until After Refactoring

- Any `cargo`, `rustc`, `just`, Bazel, build script, test script, schema
  generation, lockfile generation, release, build, or check command during the
  current no-build phase.
- Debug-profile Cargo lanes, including:

```powershell
cargo test
cargo check
cargo test -p codex-core
cargo test -p codex-cli
```

- Broad or conflict-prone release lanes before focused checks are green:

```powershell
cargo test --release
just test
just bazel-remote-test
bazel test //...
```

- Broad TUI/core lib lanes unless root intentionally accepts the expensive
  graph:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-tui -Lib -AllowBroadTuiUnitTests
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -AllowBroadCoreLibUnitTests
```

- Schema/lockfile generation before source shape stabilizes:

```powershell
just write-config-schema
just write-app-server-schema
just write-app-server-schema --experimental
just bazel-lock-update
just bazel-lock-check
```

- Final build/deploy before package-level release checks are green:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode FastRelease
powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode LowMemRelease
```

- Compaction validation before the final output and deployment shape is stable:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs '--test=compact'
```
