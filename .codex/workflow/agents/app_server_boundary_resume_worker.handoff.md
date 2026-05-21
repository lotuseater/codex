# app_server_boundary_resume_worker handoff

## Status

Source-only resume completed. The requested `codex-rs/app-server-protocol/src/protocol/v2.rs`
path does not exist in this checkout; v2 is the directory module
`codex-rs/app-server-protocol/src/protocol/v2/mod.rs`.

Continued the boundary cleanup without changing API payload shapes: app/plugin v2 wire
type imports now live in the leaf request processors that own those conversions instead
of the central `request_processors.rs` import surface. Existing dirty app-server/protocol
work from the interrupted worker was preserved.

## Files changed

Changed by this resume pass:

- `codex-rs/app-server/src/request_processors.rs`
- `codex-rs/app-server/src/request_processors/apps_processor.rs`
- `codex-rs/app-server/src/request_processors/plugins.rs`
- `.codex/workflow/agents/app_server_boundary_resume_worker.handoff.md`

Pre-existing dirty owned files preserved in the current tree:

- `codex-rs/app-server/Cargo.toml`
- `codex-rs/app-server/src/app_catalog_protocol.rs`
- `codex-rs/app-server/src/bespoke_event_handling.rs`
- `codex-rs/app-server/src/lib.rs`
- `codex-rs/app-server/src/mcp_refresh.rs`
- `codex-rs/app-server/src/message_processor.rs`
- `codex-rs/app-server/src/request_processors/config_processor.rs`
- `codex-rs/app-server/src/request_processors/external_agent_config_processor.rs`
- `codex-rs/app-server/src/request_processors/thread_processor.rs`
- `codex-rs/app-server/src/request_processors/thread_processor_tests.rs`
- `codex-rs/app-server/tests/suite/conversation_summary.rs`
- `codex-rs/app-server/tests/suite/v2/thread_read.rs`
- `codex-rs/app-server/tests/suite/v2/thread_unarchive.rs`
- `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs`

## Verification

Build/test/fmt/schema verification deliberately skipped because this is the no-build wave
and the worker hard stop forbids `cargo`, `rustc`, `just`, build scripts, tests, schema
generation, release/build/check commands, and commits.

Source-only checks run:

- `rg` confirmed app/plugin protocol imports moved out of central `request_processors.rs`.
- `git diff --check -- codex-rs/app-server/src/request_processors.rs codex-rs/app-server/src/request_processors/apps_processor.rs codex-rs/app-server/src/request_processors/plugins.rs`

## Later verification commands for root

From repo root:

```powershell
Push-Location codex-rs
just fmt
just write-app-server-schema
just write-app-server-schema --experimental
just fix -p codex-app-server
just fix -p codex-app-server-protocol
Pop-Location

powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server-protocol
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server

Push-Location codex-rs
just bazel-lock-check
Pop-Location
```

If `just bazel-lock-check` reports drift from the dirty `codex-rs/app-server/Cargo.toml`
dependency additions, root/build owner should run:

```powershell
Push-Location codex-rs
just bazel-lock-update
just bazel-lock-check
Pop-Location
```

## Blockers / next owner

No required source edit outside this worker's ownership was identified. Root/endgame owns
the skipped verification, schema fixture refresh, and any root lockfile updates.
