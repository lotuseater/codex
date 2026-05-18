# Native Feature Canary Work

This parks the C++ canary experiment before returning to prompt reduction.

## Scope

- Added a workspace crate `codex-native-feature-canary` under `codex-rs/native-feature-canary`.
- The crate builds a small C++ FFI library through `cc` and exposes a safe Rust wrapper.
- The native layer provides stable FNV-1a hashing and deterministic feature decisions, including `force-on:` and `force-off:` key prefixes for tests and canaries.
- Cargo workspace metadata was updated in `codex-rs/Cargo.toml` and `codex-rs/Cargo.lock`.
- Bazel metadata was sketched with a `cc_library` plus Rust crate target in `BUILD.bazel`.

## Verification Already Run

- `just fmt` in `codex-rs`: passed.
- `scripts\test-local-codex-release.ps1 -Package codex-native-feature-canary`: passed.
- Warm-cache release test with `-NoCleanup`: passed; second run did not recompile Rust crate work.
- C++-only edit release test: passed.
- `just bazel-lock-update`: passed with existing crate annotation warnings.
- `just bazel-lock-check`: passed.
- `just fix -p codex-native-feature-canary`: passed.

## Known Blocker

Direct Bazel test of the new target did not reach the crate because the checkout currently fails earlier in existing V8 setup: `unknown repo 'v8_python_deps' requested from @@v8+`, triggered by the repo `.bazelrc` V8 pointer-compression config. Treat that as a pre-existing Bazel/V8 setup blocker, not a canary crate failure.

## Patch

The applyable patch is saved at `native-feature-canary.patch`. It contains only:

- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- `codex-rs/native-feature-canary/**`

The worktree has been reverted back to the prompt-reduction task after saving this note and patch.
