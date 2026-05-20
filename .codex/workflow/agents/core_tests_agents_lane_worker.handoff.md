# core_tests_agents_lane_worker Handoff

Status: implemented owned agents/delegation test lane split.

Date: 2026-05-20

## Owned Changes

- Added `codex-rs/core/tests/agents.rs` as the `codex-core` integration-test wrapper for agents/delegation coverage.
- Moved the owned modules from `codex-rs/core/tests/suite/` to `codex-rs/core/tests/agents/`:
  - `agent_jobs.rs`
  - `agent_websocket.rs`
  - `agents_md.rs`
  - `codex_delegate.rs`
  - `collaboration_instructions.rs`
  - `hierarchical_agents.rs`
  - `spawn_agent_description.rs`
  - `subagent_notifications.rs`
  - `tool_parallelism.rs`
- Simplified `agents.rs` to normal sibling module declarations so the modules resolve from `tests/agents/*.rs`.

## Checks

- Confirmed no remaining `#[path = "suite/..."]` references for the moved agents/delegation modules under `codex-rs/core/tests`.
- Ran targeted formatting on `agents.rs` and the nine moved `tests/agents/*.rs` modules with `rustfmt --edition 2024`; it completed successfully. Rustfmt printed the existing stable-toolchain warning for `imports_granularity = Item`.
- Attempted targeted release verification:
  `.\scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs @('--test','agents')`.
  The first invocation used an invalid PowerShell array form and failed before Cargo by binding `agents` as `RepoRoot`.
- Retried with an explicit `$extra = @('--test','agents')`; the script initially refused to continue because repo-local Cargo activity was already active. The active process was `cargo fmt -- --config imports_granularity=Item` with process id `27628`, started `2026-05-21 00:17:14`.
- After that process cleared, reran the same targeted release lane. Cargo reached `codex-core` compilation and failed before running the `agents` integration test binary due to existing shared-library compile errors, including unresolved imports in `core/src/session/turn.rs`, `core/src/config/permissions.rs`, `core/src/tools/router.rs`, `core/src/tools/spec_plan.rs`, missing `codex_app_server_protocol`, and trait/signature mismatches. Log: `logs/test-local-release-codex-core-all-20260521-002215.log`.

## Notes For Root

- This slice intentionally does not edit unrelated dirty split files such as `client.rs`, `compact.rs`, `config.rs`, `exec.rs`, `permissions.rs`, `state.rs`, `tools.rs`, `common/`, or remaining `suite/` modules.
- Once the shared `codex-core` compile errors are repaired by their owning lanes, rerun the targeted release lane for `--test agents` before broader verification.
