# core_test_split_cargo_bazel_scout Handoff

Status: complete on 2026-05-20.

## Scope

Read-only scout for Cargo/Bazel wiring needed to split `codex-core`
integration tests. I did not edit source, run Cargo/Just/Bazel/formatters, stage,
or commit. The only write is this handoff file.

## Sources inspected

- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/core/Cargo.toml`
- `codex-rs/core/BUILD.bazel`
- `codex-rs/core/tests/all.rs`
- `codex-rs/core/tests/common/BUILD.bazel`
- `codex-rs/core/tests/common/Cargo.toml`
- `codex-rs/Cargo.toml`
- `defs.bzl`
- targeted `rg` searches for `core/tests/all.rs`, `codex_core`,
  `core_test_support`, `test_shard_counts`, `rust_test`, and current core test
  file stems

## Cargo behavior

`codex-rs/core/Cargo.toml` does not set `autotests = false` and has no
explicit `[[test]]` entries. Cargo therefore auto-discovers every top-level
`codex-rs/core/tests/*.rs` file as a separate integration test target.

Expected Cargo test target names are the top-level file stems:

- `codex-rs/core/tests/all.rs` -> `--test all`
- `codex-rs/core/tests/responses_headers.rs` -> `--test responses_headers`
- `codex-rs/core/tests/suite_bootstrap.rs`, if left top-level -> `--test suite_bootstrap`

On Windows release builds, the concrete executables are hash-suffixed under
`codex-rs/target/release/deps`, for example `all-<hash>.exe`,
`responses_headers-<hash>.exe`, and, if the helper remains top-level,
`suite_bootstrap-<hash>.exe`.

Adding more top-level `tests/*.rs` integration binaries does not require
`codex-rs/core/Cargo.toml` changes. Manifest churn is only needed if root wants
to disable auto-discovery with `autotests = false`, add explicit custom
`[[test]]` entries, change dependencies, or use nonstandard harness settings.
That is not required for the split.

Important trap: helper-only modules must not live as `codex-rs/core/tests/*.rs`
unless root intentionally wants a standalone test binary. The current
`suite_bootstrap.rs` shape is useful as a shared module, but as a top-level file
Cargo and Bazel both discover it as its own mostly-empty integration test. Move
helper-only bootstrap code under a subdirectory such as
`codex-rs/core/tests/support/suite_bootstrap.rs` and include it with `#[path =
"support/suite_bootstrap.rs"] mod suite_bootstrap;` from each real integration
test binary that needs the alias dispatcher.

## Bazel impact

`defs.bzl` already mirrors Cargo discovery. Inside `codex_rust_crate`, it loops
over `native.glob(["tests/*.rs"], allow_empty = True)`, derives:

- `test_file_stem = test.removeprefix("tests/").removesuffix(".rs")`
- `test_crate_name = test_file_stem.replace("-", "_")`
- `test_name = name + "-" + test_file_stem.replace("/", "-")`
- appends `-test` unless the name already ends in `-test`

For `codex-rs/core/BUILD.bazel` where `name = "core"`, expected Bazel labels
are:

- `all.rs` -> `//codex-rs/core:core-all-test`
- `responses_headers.rs` -> `//codex-rs/core:core-responses_headers-test`
- top-level `suite_bootstrap.rs`, if left there -> `//codex-rs/core:core-suite_bootstrap-test`

Each generated target also has a manual binary target with `-bin`, and Windows
cross targets with `-windows-cross` / `-windows-cross-bin`.

No Bazel target stanza is needed just to create a new integration test target.
`codex-rs/core/BUILD.bazel` must be edited only when root needs to adjust
per-target behavior:

- add or rebalance `test_shard_counts` for split targets;
- update `test_data_extra` if a split target needs new runtime files;
- update `integration_compile_data_extra` if a split target introduces new
  compile-time source-tree reads such as `include_str!`;
- change timeout/tags if the split changes runtime characteristics.

Current `codex-rs/core/BUILD.bazel` has shard counts only for:

- `"core-all-test": 16`
- `"core-unit-tests": 8`

Therefore `core-responses_headers-test` and `core-suite_bootstrap-test` would
exist automatically but run unsharded unless root adds entries. `defs.bzl` does
not need changes for this split. `codex-rs/core/tests/common/BUILD.bazel` does
not need changes unless root adds dependencies or files to the
`core_test_support` helper crate.

## Minimal first split

Minimal split that avoids manifest churn:

1. Keep `codex-rs/core/Cargo.toml` unchanged.
2. Keep `all.rs` as the aggregate target for the remaining suite.
3. Put shared bootstrap-only code under a non-top-level path such as
   `tests/support/suite_bootstrap.rs`, not `tests/suite_bootstrap.rs`.
4. Add one real top-level integration binary, for example
   `tests/responses_headers.rs`, and include the shared bootstrap module there
   only if that target needs test-binary alias dispatch.
5. Optionally edit only `codex-rs/core/BUILD.bazel` to add a shard count for the
   new heavy target, for example `"core-responses_headers-test": <count>`.

This gives Cargo target `responses_headers` and Bazel target
`//codex-rs/core:core-responses_headers-test` without changing manifests or
lockfiles.

## Lock, schema, and generated artifacts

Pure test-file splitting does not require lock/schema/generated updates:

- no `codex-rs/Cargo.lock` update;
- no `MODULE.bazel.lock` update;
- no `codex-rs/core/config.schema.json` update;
- no app-server schema fixture update.

Root must own these artifacts only if the implementation crosses their normal
triggers:

- Cargo dependency or workspace dependency changes: update `codex-rs/Cargo.lock`
  as needed and run/include `just bazel-lock-update`, then `just
  bazel-lock-check`, which may update `MODULE.bazel.lock`.
- `ConfigToml` or nested config type changes: run/include `just
  write-config-schema`, which may update `codex-rs/core/config.schema.json`.
- app-server protocol shape changes: run/include `just write-app-server-schema`
  and, if experimental fixtures are affected, `just write-app-server-schema
  --experimental`.
- moving tests that use `insta`: expect snapshot path/name churn because the
  integration test target name is part of snapshot identity. Root must review
  pending `*.snap.new` files and accept only intended snapshot changes.

## Verification commands after structure changes

From repo root:

```powershell
cd codex-rs
just fmt
cd ..
```

For one split Cargo target through the local release wrapper:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs '--test=responses_headers'
```

For the remaining aggregate target:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs '--test=all'
```

For Bazel target discovery and the matching generated target:

```powershell
bazel query //codex-rs/core:core-responses_headers-test
bazel test //codex-rs/core:core-responses_headers-test
```

If root adds a shard entry or wants to compare both sides of the first split:

```powershell
bazel test //codex-rs/core:core-all-test //codex-rs/core:core-responses_headers-test
```

If moved tests use snapshots:

```powershell
cd codex-rs
cargo insta pending-snapshots -p codex-core
```
