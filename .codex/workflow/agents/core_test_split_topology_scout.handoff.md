# core_test_split_topology_scout Handoff

Status: complete read-only topology scout.

Date: 2026-05-20

## Scope

Map the `codex-core` integration test harness topology and identify a safe
mechanical split path for smaller release test lanes. I did not edit source
files and did not run Cargo, Just, formatters, staging, commits, or broad build
lanes. The only file edited by this scout is this handoff.

Important concurrency note: the harness changed while this scout was running.
At first read, `codex-rs/core/tests/all.rs` and
`codex-rs/core/tests/suite/mod.rs` existed and formed the single large suite
binary. Later in the same scout, another session deleted those two files and
added top-level category wrapper binaries plus `tests/support/mod.rs`. The
summary below records both states and treats the latest filesystem snapshot as
the current topology.

## Files Read

Required first reads:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/test_surface_scout.handoff.md`
- `codex-rs/core/tests/all.rs` as it existed at first read
- `codex-rs/core/tests/suite/mod.rs` as it existed at first read
- `codex-rs/core/Cargo.toml`

Representative modules inspected:

- `codex-rs/core/tests/suite/hooks.rs`
- `codex-rs/core/tests/suite/compact.rs`
- `codex-rs/core/tests/suite/compact_remote.rs`
- `codex-rs/core/tests/suite/client.rs`
- `codex-rs/core/tests/suite/realtime_conversation.rs`
- `codex-rs/core/tests/suite/unified_exec.rs`
- `codex-rs/core/tests/suite/apply_patch_cli.rs`
- `codex-rs/core/tests/suite/shell_serialization.rs`
- `codex-rs/core/tests/suite/compact_resume_fork.rs`
- `codex-rs/core/tests/suite/window_headers.rs`

Current split files read after the concurrent edit:

- `codex-rs/core/tests/support/mod.rs`
- `codex-rs/core/tests/agents.rs`
- `codex-rs/core/tests/client.rs`
- `codex-rs/core/tests/compact.rs`
- `codex-rs/core/tests/config.rs`
- `codex-rs/core/tests/exec.rs`
- `codex-rs/core/tests/permissions.rs`
- `codex-rs/core/tests/state.rs`
- `codex-rs/core/tests/tools.rs`
- `codex-rs/core/tests/responses_headers.rs`
- `scripts/test-local-codex-release.ps1`

## Searches Run

- `first_moves_predict` for this scout prompt.
- `rg -n "^mod |^pub ...|^fn ..."` over the original `suite/mod.rs`.
- `rg -n "super::|crate::suite|crate::error|use crate::|use super"` over
  `codex-rs/core/tests/suite`.
- Line/test count scans over `codex-rs/core/tests/suite/*.rs`.
- Focused dependency scans for `COMPACT_WARNING_MESSAGE`, `FIRST_REPLY`,
  `SUMMARY_TEXT`, `apply_patch_cli::`, `compact_remote::`, `crate::error`,
  and `codex_protocol::error`.
- Wrapper coverage scan comparing every `tests/suite/*.rs` file to every
  `#[path = "suite/..."]` reference in the new top-level wrappers.
- Focused `git status --short` over the affected test harness paths.

## Initial Topology

Initial read showed the classic aggregated integration binary:

- `codex-rs/core/tests/all.rs` was the only large suite entry point.
- `all.rs` re-exported `codex_protocol::error`, then included
  `mod suite_bootstrap;` and `mod suite;`.
- `codex-rs/core/tests/suite_bootstrap.rs` registered test-binary alias
  dispatch with `ctor` and `codex_test_binary::configure_test_binary_dispatch`.
  It special-cased the apply-patch, fs-helper, and Linux sandbox helper aliases.
- `codex-rs/core/tests/suite/mod.rs` was a pure module aggregation file after
  the bootstrap extraction. It no longer carried shared imports.
- `codex-rs/core/Cargo.toml` has no explicit `[[test]]` targets for this suite;
  Cargo auto-discovers top-level `tests/*.rs` integration binaries.
- `core_test_support` is already a dev dependency and owns most shared helpers,
  so most suite modules do not need local shared modules.

This topology meant that `cargo test -p codex-core --release --test all <filter>`
still had to compile all modules in `all.rs` before applying the runtime test
filter.

## Current Topology

Current filesystem state no longer has `all.rs` or `suite/mod.rs`. Instead,
the suite has been split into eight top-level category wrappers plus the
existing `responses_headers.rs` integration binary:

- `agents.rs`: 9 modules, about 3,428 source lines, 45 tests.
- `client.rs`: 8 modules, about 11,450 source lines, 139 tests.
- `compact.rs`: 8 modules, about 9,213 source lines, 74 tests.
- `config.rs`: 19 modules, about 9,995 source lines, 144 tests.
- `exec.rs`: 10 modules, about 7,114 source lines, 166 tests.
- `permissions.rs`: 8 modules, about 11,094 source lines, 88 tests.
- `state.rs`: 10 modules, about 4,275 source lines, 63 tests.
- `tools.rs`: 12 modules, about 6,174 source lines, 67 tests.

Each wrapper starts with `mod support;` and then includes suite modules with
`#[path = "suite/<module>.rs"] mod <module>;`.

`codex-rs/core/tests/support/mod.rs` is the renamed/current shared bootstrap
module. It contains the alias dispatch `ctor` logic that was previously observed
as `suite_bootstrap.rs`.

Read-only coverage scan result for the current split:

- Suite files found: 84.
- `#[path = "suite/..."]` references found in wrappers: 84.
- Missing suite files: none.
- Duplicate suite references: none.

Focused status at the end of the scout showed these source changes were not
mine:

- Deleted: `codex-rs/core/tests/all.rs`
- Deleted: `codex-rs/core/tests/suite/mod.rs`
- Modified: `codex-rs/core/tests/suite/shell_serialization.rs`
- Untracked: `codex-rs/core/tests/agents.rs`
- Untracked: `codex-rs/core/tests/client.rs`
- Untracked: `codex-rs/core/tests/compact.rs`
- Untracked: `codex-rs/core/tests/config.rs`
- Untracked: `codex-rs/core/tests/exec.rs`
- Untracked: `codex-rs/core/tests/permissions.rs`
- Untracked: `codex-rs/core/tests/state.rs`
- Untracked: `codex-rs/core/tests/support/mod.rs`
- Untracked: `codex-rs/core/tests/tools.rs`

## Module Coupling

Most suite modules are now independent of the old `crate::suite` namespace.
The remaining intentional sibling dependencies are small and grouped in the
same wrapper:

- `compact_resume_fork.rs` imports from `super::compact`:
  `FIRST_REPLY`, `SUMMARY_TEXT`, and `COMPACT_WARNING_MESSAGE`.
- `window_headers.rs` imports `super::compact::COMPACT_WARNING_MESSAGE`.
- `shell_serialization.rs` now imports from `super::apply_patch_cli`:
  `apply_patch_harness` and `mount_apply_patch`.
- `compact.rs` exposes the three compact constants as `pub(super)`.
- `apply_patch_cli.rs` exposes `apply_patch_harness` and `mount_apply_patch`
  as `pub async fn`.

No current suite module uses `crate::suite`. A search for `crate::error` in the
suite found no uses; the old `pub use codex_protocol::error` from `all.rs`
appears unnecessary for the current suite files.

Representative module observations:

- `hooks.rs` is large, self-contained, and had no `super::`, `crate::suite`,
  or `use crate::` dependency at inspection time. It is gated under
  `#[cfg(not(target_os = "windows"))]` in the current `permissions.rs` wrapper.
- `compact.rs` is self-contained except for the `pub(super)` constants consumed
  by compact resume/window modules.
- `compact_remote.rs` had no observed `super::` or `crate::suite` dependency.
- `client.rs`, `realtime_conversation.rs`, and `unified_exec.rs` rely heavily
  on `core_test_support` helpers but had no observed dependency on other suite
  modules.
- `apply_patch_cli.rs` is a high-test-count module, but it is coupled to
  `shell_serialization.rs` through shared helper functions.

## Recommended Split Shape

If starting from the original one-binary topology, the first safest mechanical
split would be a standalone `hooks` lane:

```rust
mod support;

#[cfg(not(target_os = "windows"))]
#[path = "suite/hooks.rs"]
mod hooks;
```

Then remove `mod hooks;` from the old `suite/mod.rs`. This avoids duplicated
tests, preserves the helper alias bootstrap, and splits a 3,434-line,
37-test self-contained module out of the monolithic `all` binary.

Given the current worktree, that minimal first split has already been
superseded by a broader topic split. The current wrapper shape is coherent
because it:

- references all 84 suite modules exactly once;
- keeps compact dependents in `compact.rs`;
- keeps `apply_patch_cli` and `shell_serialization` together in `exec.rs`;
- preserves OS gating for `abort_tasks`, `windows_sandbox`, and the
  non-Windows permission/hook modules;
- centralizes the test-binary alias bootstrap in `tests/support/mod.rs`.

Root should treat the current category split as the central implementation
slice to own, rather than adding a second split layer.

Exact central files root must own before commit:

- `codex-rs/core/tests/all.rs`: deletion or replacement must be intentional.
- `codex-rs/core/tests/suite/mod.rs`: deletion or replacement must be
  intentional.
- `codex-rs/core/tests/support/mod.rs`: shared bootstrap for every new wrapper.
- `codex-rs/core/tests/agents.rs`
- `codex-rs/core/tests/client.rs`
- `codex-rs/core/tests/compact.rs`
- `codex-rs/core/tests/config.rs`
- `codex-rs/core/tests/exec.rs`
- `codex-rs/core/tests/permissions.rs`
- `codex-rs/core/tests/state.rs`
- `codex-rs/core/tests/tools.rs`
- `codex-rs/core/tests/suite/shell_serialization.rs`: import was changed from
  the old `crate::suite::apply_patch_cli` shape to sibling `super::apply_patch_cli`.

## Risks

- Concurrency risk is real: the harness changed during the scout. Re-read these
  central files before editing or committing.
- The split is unverified locally. The coverage scan proves no missing or
  duplicate path references, but it does not prove the wrappers compile.
- `mod support;` currently creates an unused module in wrappers unless the
  `ctor` side effect is enough to suppress lint concerns. Verify with the
  release lane.
- Test names and target names changed. CI or local scripts that hard-code
  `--test all` must move to the new topic target names.
- `responses_headers.rs` remains a separate existing integration binary and
  does not include `mod support;`; that is probably fine because it does not use
  the alias-dispatch helpers, but it is a separate pattern.
- Full-suite wall-clock may not decrease automatically because multiple
  integration binaries can add per-binary overhead. The main win is focused
  compile/run lanes: `--test exec`, `--test client`, etc. no longer compile the
  whole former `all` suite.

## Verification Lane

No verification was run by this scout.

The local release test wrapper refuses broad `codex-core` integration targets
unless `-AllowIntegrationTargets` is supplied. It supports explicit Cargo target
args through `-ExtraCargoArgs`.

Recommended first verification after source ownership is settled:

```powershell
just fmt
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs @('--test','exec')
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs @('--test','compact')
```

Then run the remaining topic lanes, at least:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs @('--test','client')
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs @('--test','permissions')
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs @('--test','config')
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs @('--test','agents')
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs @('--test','state')
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs @('--test','tools')
```

If time is constrained, start with `exec` and `compact` because they cover the
two remaining sibling-dependency clusters.

## Commit Readiness

Not commit-ready from this scout alone. The current source split looks
mechanically consistent by path coverage and dependency grouping, but it needs:

- root confirmation that the concurrent deletion of `all.rs` and `suite/mod.rs`
  is intended;
- `just fmt`;
- focused release verification for the new topic targets;
- a final status review to ensure only the intended split files and this
  handoff are included.
