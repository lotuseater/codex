# core_tests_harness_manifest_worker Handoff

Status: edits complete on 2026-05-20.

## Owned Paths

- `codex-rs/core/Cargo.toml`
- `codex-rs/core/BUILD.bazel`
- `codex-rs/core/tests/support/mod.rs`
- `.codex/workflow/agents/core_tests_harness_manifest_worker.handoff.md`

## Decisions

- Made `codex-core` integration tests explicit with `autotests = false` and
  `[[test]]` entries for the split binaries:
  `agents`, `client`, `compact`, `config`, `exec`, `permissions`, `state`, and
  `tools`.
- Kept the pre-existing standalone `responses_headers` integration test explicit
  so switching off Cargo auto-discovery does not drop coverage.
- Removed the Bazel shard-count reference to stale `core-all-test` and wired the
  split Bazel test targets, plus `core-responses_headers-test`.
- Kept the reusable test-binary dispatch bootstrap in
  `codex-rs/core/tests/support/mod.rs`, which each split top-level test imports
  via `mod support;`.
- Preserved existing dependency substitutions already present in
  `codex-rs/core/Cargo.toml` before this worker resumed:
  `codex-app-server-protocol` -> `codex-app-catalog-types` and
  `codex-thread-store` -> `codex-thread-store-api`.

## Remaining Risks

- `codex-rs/core/tests/all.rs` is already deleted in the working tree but is not
  an owned path for this worker. If that deletion is not staged by its owner,
  Bazel's `tests/*.rs` glob would still emit `core-all-test` in that separate
  commit/tree.
- Full release-profile `codex-core` coverage was intentionally not run by this
  worker because the prompt forbids broad core/workspace builds or tests.

## Verification

Passed:

```powershell
git diff --check -- codex-rs/core/Cargo.toml codex-rs/core/BUILD.bazel codex-rs/core/tests/support/mod.rs .codex/workflow/agents/core_tests_harness_manifest_worker.handoff.md
cargo metadata --manifest-path codex-rs/Cargo.toml --no-deps --format-version 1
rustfmt --check codex-rs/core/tests/support/mod.rs
```

Deferred to root after all split-lane owners land:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core
bazel query //codex-rs/core:core-agents-test //codex-rs/core:core-client-test //codex-rs/core:core-compact-test //codex-rs/core:core-config-test //codex-rs/core:core-exec-test //codex-rs/core:core-permissions-test //codex-rs/core:core-responses_headers-test //codex-rs/core:core-state-test //codex-rs/core:core-tools-test
```
