# solid_refactor_area_review_retry_core_api_worker handoff

Status: review completed read-only. No source edits, staging, commits, pushes, or verification commands were run.

## Findings

1. Commit blocker: `codex-rs/Cargo.lock` is stale for the new `codex-core-api -> codex-core-domain-types` dependency.

Evidence: `codex-rs/core-api/Cargo.toml:21` adds `codex-core-domain-types = { workspace = true }`. `codex-rs/core-api/src/identifiers.rs:3-6` re-exports `SessionId`, `ThreadId`, `ToolCallId`, and `TurnId` from `codex_core_domain_types`, and `codex-rs/core-api/src/lib.rs:5,62-65` publishes that module plus the root aliases. The `codex-core-api` package entry in `codex-rs/Cargo.lock:2778-2794` still lists dependencies through `codex-utils-absolute-path` and omits `codex-core-domain-types`; the adjacent `codex-core-domain-types` package at `codex-rs/Cargo.lock:2796-2801` only has its own `serde` dependency. This means the manifest/source boundary moved, but the lockfile did not record the new edge.

Exact root-owned next action: regenerate/reconcile `codex-rs/Cargo.lock` from `codex-rs` until the `codex-core-api` package entry contains `"codex-core-domain-types"`, then run the Bazel lock follow-up (`just bazel-lock-update`, then `just bazel-lock-check`).

2. Commit blocker: two new boundary files exist only on disk and must be intentionally included by root before any integration commit.

Evidence: `git show HEAD:codex-rs/core-api/src/identifiers.rs` reports that the file exists on disk but not in `HEAD`, while `codex-rs/core-api/src/lib.rs:5` declares `pub mod identifiers;` and `codex-rs/core-api/src/identifiers.rs:3-6` contains the public identifier re-exports. `git show HEAD:codex-rs/core-domain/types/BUILD.bazel` reports the same disk-only state for the Bazel target file; `codex-rs/core-domain/types/BUILD.bazel:1-5` defines the `codex_core_domain_types` crate target used by the new boundary dependency.

Exact root-owned next action: include `codex-rs/core-api/src/identifiers.rs` and `codex-rs/core-domain/types/BUILD.bazel` in the root integration state before committing; do not commit only the tracked diffs.

## Checked Scope

- No in-repo Rust consumer imports of `codex_core_api::ThreadId`, `codex_core_api::ProtocolThreadId`, `codex_core_api::SessionId`, `codex_core_api::TurnId`, `codex_core_api::ToolCallId`, grouped imports containing those names, or `codex_core_api::identifiers` were found. The only `codex_core_api::...` consumer matches are the existing `thread-manager-sample` facade imports for other symbols.
- Re-export boundary looks coherent in source: `codex_core_api::ProtocolThreadId` remains the protocol-backed thread id (`codex-rs/core-api/src/lib.rs:40`), while root `SessionId`, `ThreadId`, `ToolCallId`, and `TurnId` are domain-owned identifier exports (`codex-rs/core-api/src/lib.rs:62-65`; `codex-rs/core-api/src/identifiers.rs:3-6`).
- No app-server schema fixture change appears directly required by this core-api identifier move.
- No cargo/rustc/just/Bazel/schema commands were run because this review worker was command-banned from those commands.

## Exact Root-Owned Verification Path

1. Include the disk-only boundary files: `codex-rs/core-api/src/identifiers.rs` and `codex-rs/core-domain/types/BUILD.bazel`.
2. From `codex-rs`, run the narrow release-profile core-api verification/update lane, e.g. `cargo check --release -p codex-core-api`, and verify `codex-rs/Cargo.lock` records `"codex-core-domain-types"` under the `codex-core-api` package.
3. Run `just bazel-lock-update`, `just bazel-lock-check`, `just fmt`, and `just fix -p codex-core-api`.
4. Run the consumer smoke check already identified by the source workers: `cargo check --release -p codex-thread-manager-sample`.
