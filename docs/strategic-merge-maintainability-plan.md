# Strategic Merge Maintainability Plan

Date: 2026-05-15

This note tracks maintainability issues found during the `slow-context-budget-mode` merge and the follow-up dependency refactor. The intent is to keep local feature work ahead of `main` without making future merges, builds, or verification lanes unnecessarily heavy.

## Current State

- The blackboard and `origin/main` merge has been completed; this document now tracks follow-up architecture work, not merge blockers.
- The instruction and self-review prompt wording has been updated to prefer the largest coherent verified slice when it materially improves long-term design.
- `codex-config` is now an enforced config-only boundary. It must not depend on `codex-protocol`, `codex-app-server-protocol`, `codex-api`, `codex-otel`, `codex-network-proxy`, `gix*`, `hyper*`, `prost*`, `rama-*`, `starlark*`, or `tonic*`.
- `scripts\check-cargo-dependency-boundaries.ps1 -Package codex-config` is the durable canary for that boundary.

## Completed Dependency Refactors

- Runtime network proxy ownership was split earlier: `codex-network-proxy-config` owns config/data DTOs, while `codex-network-proxy` remains the runtime crate.
- Config and protocol DTO ownership was split further: `codex-config-types` now owns lightweight config/model/realtime/hook DTOs, and `codex-protocol` re-exports them only for compatibility.
- Permission and sandbox DTO ownership was split into `codex-permission-types`; config and filesystem code import from that owner crate instead of `codex-protocol`.
- Git SHA ownership was split into `codex-git-types`; `codex-git-utils` and `codex-protocol` use/re-export that owner type.
- Remote thread-config gRPC code moved from `codex-config` into `codex-thread-config-remote`, so `tonic`, `prost`, and `hyper` are no longer compiled for config-only checks.
- The config trust path no longer depends on `codex-git-utils`; the needed filesystem-only worktree root resolver is local to the config loader, so `gix` is no longer compiled for config-only checks.
- `codex-model-provider-info` is now split between a lightweight default path and an opt-in `runtime` feature for API/app-server/header conversion helpers.
- `codex-features` no longer depends on `codex-otel` or `codex-protocol`; runtime event emission is owned by `codex-core`.
- TUI queued-input ordering policy moved into `codex-input-queue`, a lightweight zero-dependency crate. Normal queued prompts now remain separate model turns, while rejected-steer retry batches remain explicit retry batches; `codex-tui` keeps only UI-specific preview, composer restore, and command submission behavior.
- Turn-diff tracking moved into `codex-turn-diff`; `codex-core` now only adapts protocol and apply-patch file-change DTOs into the owner crate.
- Cognos operation logic moved into `codex-cognos-ops`; `codex-core` now only parses tool invocations, resolves runtime paths/state, and formats tool output.
- Blackboard session runtime moved into `codex-blackboard`; `codex-core` now only maps `Config` plus `SessionSource` into blackboard-owned session options.
- Transcript/history-cell rendering moved into `codex-tui-render`; `codex-tui` now owns app-server/config/TUI boundary adaptation while the render crate owns pure display modules and snapshot fixtures.
- Runtime metric display DTOs moved into `codex-runtime-metrics-types`, so render code no longer depends on `codex-otel`.
- `scripts\analyze-branch-conflict-surface.ps1 -BaseRef origin/main -IncludeWorkingTree` is the durable canary for tracking committed and uncommitted local churn in upstream-hot files before recurring `main` merges.

## Boundary Results

- `cargo tree -p codex-config --edges normal,build` no longer includes `codex-protocol`, `codex-app-server-protocol`, `codex-api`, `codex-otel`, `codex-network-proxy`, `rama-*`, `starlark*`, `gix`, `tonic`, `prost`, or `hyper`.
- `cargo check --release -p codex-config -j 1` completed successfully after the split and now reaches the crate quickly instead of compiling remote/protocol/runtime graphs first.
- `cargo check --release -p codex-thread-config-remote -j 1` completed successfully, proving the remote-loader behavior remains available from its new owner crate.
- `cargo check --release -p codex-protocol -p codex-app-server-protocol -p codex-file-system -p codex-git-utils -j 1` completed successfully after restoring omitted protocol-owned event/API types.
- `cargo tree -p codex-tui-render --edges normal -i codex-app-server-protocol` no longer finds a path, and the render graph no longer includes `codex-config`, `rmcp`, `codex-otel`, `codex-core`, `rama-*`, or `curve25519-dalek`.
- `cargo check --release -p codex-tui-render -j 1`, `scripts\test-local-codex-release.ps1 -Package codex-tui-render`, and `cargo check --release -p codex-tui --lib -j 1` completed successfully after the render split.

## Follow-Up Refactors

- Split more protocol-owned surfaces into owner crates only when a concrete broad consumer benefits. The remaining `codex-protocol` weight is mixed across HTTP errors, image helpers, XML serialization, ICU formatting, policy matching, schema/TS derivation, and event models.
- Continue extracting runtime-specific behavior out of broad crates. `codex-core`, app-server client paths, and full TUI tests still compile large runtime graphs and should gain narrower owner crates or pure state-machine crates.
- Keep core-owned tool/session files as adapters. If a future feature needs substantial logic behind a tool or session hook, create or extend an owner crate first and keep `codex-core` responsible for runtime wiring.
- Keep TUI unit-testable state machines outside the broad `codex-tui` test graph where possible. Queue ordering now has a lightweight owner crate; automatic prompt construction and review evidence should follow the same pattern, with full TUI tests used as final canaries.
- Continue reducing `codex-tui-render` protocol weight. The active extraction removed app-server, config, core, otel, rmcp, and rama/crypto edges, but `reqwest` still enters through `codex-protocol`; the next coherent slice should move remaining display-only protocol inputs into render-owned view models or lighter DTO crates.
- Split optional RMCP conversion shims out of `codex-app-server-protocol`. The `rmcp-conversions` feature is useful for compatibility coverage, but the protocol crate should stay focused on wire DTOs, serde, schema, and TS export; runtime/MCP adapter code should own the RMCP dependency and its slow tests.
- Keep MultiAgentV2 tool definitions, implementation handlers, tool docs, and registry specs generated from or backed by one canonical source.
- Keep the release cleanup policy dep-info-aware: prune orphaned deps and disposable test executables, classify duplicate dependency versions, but do not delete active same-name hashed variants or known unavoidable duplicate-version cases.

## Verification Plan

- Run `powershell -ExecutionPolicy Bypass -File scripts\check-cargo-dependency-boundaries.ps1 -Package codex-config`.
- Run `rg -n "codex_protocol::|codex_app_server_protocol::" codex-rs\config\src` and expect no matches.
- Run release checks for changed owner crates: `codex-config-types`, `codex-permission-types`, `codex-git-types`, `codex-features`, `codex-model-provider-info`, `codex-file-system`, `codex-git-utils`, `codex-thread-config-remote`, and `codex-config`.
- Run `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-input-queue` for queue policy, then use a narrow `codex-tui` filter only as the integration canary.
- For history-cell work, prefer `codex-tui-render` owner-crate tests once the extraction is complete; treat filtered `codex-tui` history-cell release tests as expensive final canaries and avoid rerunning them for source changes outside transcript rendering.
- Run app-server/protocol canaries after DTO moves because they preserve public wire compatibility through re-exports.
- Run `codex-app-server-protocol --features rmcp-conversions` only when touching RMCP conversion code until the conversion shims move to a narrower owner crate.
- Run owner-crate checks for recent merge-pressure extractions: `codex-turn-diff`, `codex-cognos-ops`, and `codex-blackboard`; use core release canaries only after owner-crate tests pass.
- Run `powershell -ExecutionPolicy Bypass -File scripts\analyze-branch-conflict-surface.ps1 -BaseRef origin/main -IncludeWorkingTree -Top 20` before and after a modularization pass to confirm broad-file churn is moving into owner crates before committing it.
- Run `just fmt`, scoped `just fix -p` for changed crates, `just write-config-schema`, `just bazel-lock-update`, `just bazel-lock-check`, `git diff --check`, then FastRelease build/deploy.
