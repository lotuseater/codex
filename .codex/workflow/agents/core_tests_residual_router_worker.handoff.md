# core_tests_residual_router_worker Handoff

## Status

Complete. All residual `codex-rs/core/tests/suite/*.rs` modules are routed into split integration-test binaries.

## Files Changed

- `codex-rs/core/Cargo.toml`
- `codex-rs/core/tests/client.rs`
- `codex-rs/core/tests/config.rs`
- `codex-rs/core/tests/exec.rs`
- `codex-rs/core/tests/permissions.rs`
- `codex-rs/core/tests/state.rs`
- `codex-rs/core/tests/telemetry.rs`

## Residual Modules Routed

- `hooks_mcp.rs` -> `permissions`
- `permissions_messages.rs` -> `permissions`
- `request_permissions.rs` -> `permissions`
- `items.rs` -> `state`
- `model_switching.rs` -> `config`
- `models_cache_ttl.rs` -> `config`
- `models_etag_responses.rs` -> `config`
- `override_updates.rs` -> `config`
- `personality.rs` -> `config`
- `personality_migration.rs` -> `config`
- `prompt_debug_tests.rs` -> `config`
- `quota_exceeded.rs` -> `config`
- `remote_models.rs` -> `config`
- `remote_env.rs` -> `exec`
- `sqlite_state.rs` -> `state`
- `stream_error_allows_next_turn.rs` -> `client`
- `stream_no_completed.rs` -> `client`
- `otel.rs` -> new narrow `telemetry` split binary

## Verification

- Ran a non-build route comparison: `suite 75`, `routed 75`, `missing []`, `extra []`.
- Intentionally skipped focused release `--no-run` verification because root already recorded unrelated shared `codex-core` compile blockers in `logs/test-local-release-codex-core-all-20260521-003300.log`.
- Did not run `cargo`, `just`, Bazel, build scripts, or tests.

## Commit

- Routing commit: `d0a3390511`
