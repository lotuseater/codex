# core_compile_config_permissions_worker Handoff

Date: 2026-05-21

## Status

- Repaired the owned config/permissions compile blocker caused by a stale `codex_protocol::permissions::project_roots_glob_pattern` import.
- Kept the fix inside `codex-rs/core/src/config/permissions.rs`; no app-server protocol dependency or manifest change was added.
- `codex-rs/core/src/config/mod.rs` is dirty in the worktree from another worker's unrelated model compaction change and was not edited or staged by this lane.

## Files Changed

- `codex-rs/core/src/config/permissions.rs`
- `.codex/workflow/agents/core_compile_config_permissions_worker.handoff.md`

## Compile Blockers Fixed

- Removed the unresolved `codex_protocol::permissions::project_roots_glob_pattern` import.
- Added a local `project_roots_glob_pattern` helper for config permission profile compilation so `:workspace_roots` scoped deny glob patterns continue compiling without crossing the config/protocol boundary.

## Boundary Issues Left

- No remaining app-server protocol imports were found in `codex-rs/core/src/config`, `codex-rs/config-types/src`, or `codex-rs/config/src`.
- Broader `codex-core` compile failures remain outside this lane, including a release check failure in `codex-rs/otel/src/events/session_telemetry.rs` for the unhandled `ResponseEvent::Incomplete` variant.

## Verification

- Ran `just fmt` from `codex-rs`.
- Ran `git diff --check -- codex-rs/core/src/config/permissions.rs`: passed.
- Ran static boundary scan:
  `rg -n "project_roots_glob_pattern|codex_app_server_protocol" codex-rs\\core\\src\\config codex-rs\\config-types\\src codex-rs\\config\\src`
  No matches remained.
- Attempted focused release script checks:
  - `scripts\\test-local-codex-release.ps1 -Package codex-core -Filter config` was refused by the script because `codex-core` package tests require `-Lib` to avoid integration targets.
  - `scripts\\test-local-codex-release.ps1 -Package codex-core -Lib -Filter config` was refused by the script because broad core lib unit tests require `-AllowBroadCoreLibUnitTests`, which this worker did not use.
- Ran release compile check:
  `cargo check --release -p codex-core --lib`
  Failed before `codex-core` could be checked due to the external `codex-otel` compile error above. Log: `logs/core-config-cargo-check-release-20260521-015907.log`.

## Commit

- Commit pending after this handoff is written and path-scoped staging excludes unrelated dirty files.
