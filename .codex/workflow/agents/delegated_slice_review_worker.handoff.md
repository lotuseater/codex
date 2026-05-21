# delegated_slice_review_worker Handoff

Status: complete.

Scope:
- Reviewed committed slice with `git diff 556654f05d..HEAD` because the requested full anchor `556654f05d030847c2d1ec371d9632cfc23b938b` is not present locally.
- Inspected the named dirty-tree source files and workflow artifacts, especially `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs`, `codex-rs/app-server/Cargo.toml`, `.codex/workflow/solid-refactor-handoff.md`, and `.codex/workflow/agents/*.handoff.md`.
- Did not fix code or run build/test/schema/lockfile commands.

## Findings

1. `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs:367` drops active permission profile modifications when converting the v2 payload back to the core type.

   `ActivePermissionProfile` in the v2 protocol currently exposes only `id` and `extends` (`codex-rs/app-server-protocol/src/protocol/v2/permissions.rs:330`), while the core type carries `modifications` for bounded runtime overlays such as additional writable roots (`codex-rs/permission-types/src/lib.rs:3698`). Core config already constructs active profiles with `ActivePermissionProfileModification::AdditionalWritableRoot` when legacy requested writable roots exist (`codex-rs/core/src/config/mod.rs:2656`). The new v2 conversion reconstructs the core profile with `modifications: Vec::new()`, so any app-server round trip through this v2 type silently loses those roots.

   Exact patch suggestion:
   - Add a v2 `ActivePermissionProfileModification` tagged union and a `pub modifications: Vec<ActivePermissionProfileModification>` field to v2 `ActivePermissionProfile`.
   - Map `value.modifications` in both `From<CoreActivePermissionProfile> for ActivePermissionProfile` and `From<ActivePermissionProfile> for CoreActivePermissionProfile` instead of using `Vec::new()`.
   - Add coverage that config-derived additional writable roots survive through the app-server v2 boundary, preferably in the relevant app-server v2 thread/turn-context tests.

   Deferred verification:
   - `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server-protocol`
   - `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server -Filter permission`

2. `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs:183` changes the v2 API shape by adding `FileSystemAccessMode::None`, but no generated app-server schema/TypeScript fixture changes are present in the inspected status.

   The repo guidance requires regenerating app-server schemas when v2 API shapes change. Leaving the generated artifacts stale can make schema checks fail and leaves clients without the new `none` variant even though the Rust protocol accepts it.

   Exact patch suggestion:
   - Run `cd codex-rs; just write-app-server-schema` and include the generated schema/TypeScript fixture diffs.
   - Then run protocol verification with `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server-protocol`.

3. `codex-rs/app-server/Cargo.toml:34` and `codex-rs/app-server/Cargo.toml:73` add Rust dependencies, while the inspected dirty status shows `codex-rs/Cargo.lock` changed but no `MODULE.bazel.lock` update.

   The repo rule for Rust dependency changes requires `just bazel-lock-update` and `just bazel-lock-check`. This may be a no-op for workspace-internal dependencies, but it should be proven before the dependency slice is considered ready.

   Exact patch suggestion:
   - From repo root after dependency edits settle, run `cd codex-rs; just bazel-lock-update` and include `MODULE.bazel.lock` if it changes.
   - Run `cd codex-rs; just bazel-lock-check`.

## Non-Findings / Notes

- I did not find a high-confidence issue in the committed workflow/doc slice from `556654f05d..HEAD`; its substantive code change is the Windows gate restoration in `codex-rs/core/tests/permissions.rs`.
- The anchor mismatch is documented here so root can decide whether to re-run against a refreshed full SHA if needed.
