# config_connectors_boundary_worker Handoff

Date: 2026-05-21

## Status

- Completed owned source refactor slice.
- Commit: not created. A clean commit is blocked by incomplete focused verification, existing unrelated dirty owned paths, and a manifest-owned connector dependency change.

## Files Changed

Changes made in this worker:

- `codex-rs/config/src/state.rs`
  - Added `UserConfigLayerSource` so user config layer identity is carried as a named config-domain value instead of a positional `Option<ProfileV2Name>`.
  - Replaced `with_user_config_profile(..., Option<...>, ...)` with `with_user_config_layer(..., UserConfigLayerSource, ...)`.
- `codex-rs/config/src/loader/mod.rs`
  - Loads user layers through `UserConfigLayerSource::unprofiled` / `::profiled`.
- `codex-rs/config/src/lib.rs`
  - Re-exported `UserConfigLayerSource`.
- `codex-rs/connectors/src/lib.rs`
  - Added `ConnectorDirectoryFetchPolicy::{UseCache, ForceRefresh}`.
  - Added `list_all_connectors_with_fetch_policy`.
  - Kept `list_all_connectors_with_options(..., force_refetch: bool, ...)` as a compatibility bridge for app-server/chatgpt callsites outside this worker's ownership.
  - Updated in-crate connector tests to call the enum-based API.

Pre-existing dirty owned files preserved and not owned by this worker:

- `codex-rs/config-types/src/lib.rs` (`HookEventName::SubagentStart`)
- `codex-rs/config/src/config_toml.rs` (`model_compact_percentage`)
- `codex-rs/config/src/lib.rs` (`ProfileV2Name` re-export; this worker added only the `UserConfigLayerSource` re-export)
- `codex-rs/connectors/Cargo.toml` and connector source imports moving `AppInfo` from `codex_app_server_protocol` to `codex_app_catalog_types`

## Verification

- Ran: `cargo fmt -p codex-config -p codex-connectors` from `codex-rs`.
- Ran: `git diff --check -- codex-rs/config codex-rs/config-types codex-rs/connectors` and it passed, with only CRLF warnings.
- Attempted: `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-config`.
  - It did not start because the wrapper detected an active repo Cargo lane.
  - Active process observed: `cargo:7344` running `cargo test -p codex-app-server-protocol --release -j 1`, with `rustc:28968`; log `logs/test-local-release-codex-app-server-protocol-all-20260521-022651.log` showed it still running after 800s.
- `codex-connectors` focused release test was deliberately not started for the same reason.

## Blockers / Next Owners

- Root/build owner: wait for the active `codex-app-server-protocol` release test lane to finish, then run focused release tests:
  - `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-config`
  - `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-connectors`
- Manifest/root owner: `codex-rs/connectors/Cargo.toml` is dirty and outside this worker's allowed edit scope, but the connector source import move to `codex_app_catalog_types` depends on that manifest change.
- App-server/chatgpt owners: migrate remaining external `list_all_connectors_with_options(..., /*force_refetch*/ bool, ...)` callsites to `list_all_connectors_with_fetch_policy(..., ConnectorDirectoryFetchPolicy::..., ...)`.
- Core/manifest owner: per `config_provenance_boundary_worker`, if core should stop using the `codex_config` re-export for config provenance types, add the direct dependency/wiring in the root-owned core manifest slice rather than adding core compatibility re-exports.

## Suggested Commit Scope

Do not stage all owned dirty files blindly. A clean scoped commit, after focused verification, should include only:

- This worker's hunks in `codex-rs/config/src/state.rs`, `codex-rs/config/src/loader/mod.rs`, `codex-rs/config/src/lib.rs`, and `codex-rs/connectors/src/lib.rs`
- Plus the manifest/source import changes only if the manifest/root owner confirms they are part of the same connector boundary slice.
