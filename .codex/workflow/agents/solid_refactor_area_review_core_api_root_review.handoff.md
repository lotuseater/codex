# solid_refactor_area_review_core_api_root_review handoff

Status: root-authored narrow review on 2026-05-21 because the visible core-api retry workers were still running without a handoff. No builds, tests, Cargo, just, Bazel, schema generation, staging, commits, or pushes were run for this source slice.

## Findings

### P2 - Core-api identifier source boundary looks coherent, but lock/Bazel verification remains required

Evidence:
- `codex-rs/core-api/src/lib.rs:5` exposes `pub mod identifiers`.
- `codex-rs/core-api/src/lib.rs:40` preserves the protocol type as `ProtocolThreadId`.
- `codex-rs/core-api/src/lib.rs:62-65` re-export `SessionId`, `ThreadId`, `ToolCallId`, and `TurnId` from `identifiers`.
- `codex-rs/core-api/src/identifiers.rs` re-exports those IDs from `codex_core_domain_types`.
- `codex-rs/core-api/Cargo.toml:21` adds `codex-core-domain-types`.
- `codex-rs/core-domain/types/Cargo.toml:16` adds `serde` derive support, and `codex-rs/Cargo.lock:2797` reflects the new `codex-core-domain-types` dependency.

Focused `rg` found no direct `codex_core_api::ThreadId`, `codex_core_api::{... ThreadId ...}`, or `codex_core_api::identifiers` source consumer outside the core-api boundary. `ProtocolThreadId` currently appears only in `codex-rs/core-api/src/lib.rs`.

Root-owned next action: after source blockers in other areas are fixed, run the core-api release verification and required dependency-lock follow-up:

```powershell
Push-Location codex-rs
cargo check --release -p codex-core-api
cargo check --release -p codex-thread-manager-sample
just bazel-lock-update
just bazel-lock-check
just fmt
just fix -p codex-core-api
Pop-Location
```

### P2 - App-server schema JSON is not part of the core-api identifier commit

Evidence:
- `git diff --name-only -- codex-rs/core-api codex-rs/core-domain/types codex-rs/Cargo.lock MODULE.bazel.lock codex-rs/app-server-protocol/schema/json` shows the core-api/domain source files and `codex-rs/Cargo.lock`, plus many `codex-rs/app-server-protocol/schema/json/*.json` files.
- The core-api identifier move does not itself explain the app-server schema JSON drift.

Root-owned next action: keep app-server schema JSON out of the core-api identifier commit unless the DTO/source change that caused those schema updates is included and verified in the same slice.

## Commit Boundary

Potential core-api source slice after blockers are fixed and verified:

- `codex-rs/core-api/Cargo.toml`
- `codex-rs/core-api/src/lib.rs`
- `codex-rs/core-api/src/identifiers.rs`
- `codex-rs/core-domain/types/Cargo.toml`
- `codex-rs/core-domain/types/src/lib.rs`
- `codex-rs/Cargo.lock`
- Bazel lock/build-file updates produced by the required lock refresh, if any

Exclude app-server schema JSON unless paired with the owning app-server protocol source change.
