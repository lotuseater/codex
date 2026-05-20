# core_test_split_lane_plan_scout Handoff

Status: complete read-only lane plan, written 2026-05-20.

## Scope

- Goal: propose a small-commit implementation sequence for splitting the large
  `codex-core` integration test aggregate into faster, targeted test binaries.
- Boundary: this scout did not edit source files, did not run Cargo/Just,
  did not run formatters, did not stage/commit/push, and did not start broad
  build lanes.
- One handoff file was written: this file.

## Sources inspected

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/test_surface_scout.handoff.md`
- Existing split-scout placeholders under `.codex/workflow/agents/core_test_split_*.handoff.md`
- `.codex/prototypes/plan-core-test-split.ps1` and its read-only inventory output
- `codex-rs/core/Cargo.toml`
- `codex-rs/core/BUILD.bazel`
- `codex-rs/core/tests/all.rs`
- `codex-rs/core/tests/suite/mod.rs`
- `codex-rs/core/tests/suite_bootstrap.rs`
- `codex-rs/core/tests/responses_headers.rs`
- `codex-rs/core/tests/common/Cargo.toml`
- `codex-rs/core/tests/common/lib.rs`
- Top largest current physical-line files in `codex-rs/core/tests/suite/`:
  - `hooks.rs` - 3799 lines
  - `compact.rs` - 3767 lines
  - `realtime_conversation.rs` - 3698 lines
  - `compact_remote.rs` - 3367 lines
  - `approvals.rs` - 3280 lines
  - `unified_exec.rs` - 3220 lines
  - `client.rs` - 3142 lines
  - `code_mode.rs` - 2920 lines
  - `rmcp_client.rs` - 2502 lines
  - `client_websockets.rs` - 2137 lines

## Current facts

- `codex-rs/core/tests/all.rs` is still the aggregate integration binary. It
  now contains `pub use codex_protocol::error;`, `mod suite_bootstrap;`, and
  `mod suite;`.
- `codex-rs/core/tests/suite/mod.rs` is now only the module registry. The
  dispatch/alias `ctor` setup has already been extracted out of it.
- `codex-rs/core/tests/suite_bootstrap.rs` currently holds the extracted
  `configure_test_binary_dispatch("codex-core-tests", ...)` setup.
- `codex-rs/core/Cargo.toml` does not disable Cargo integration-test
  auto-discovery. A root-level `tests/suite_bootstrap.rs` is therefore likely
  auto-discovered as its own zero-test integration binary. Fixing that should be
  the first harness cleanup before adding more wrappers.
- `codex-rs/core/tests/responses_headers.rs` already exists as a standalone
  top-level integration test binary. Treat it as the existing split precedent.
- Static coupling checks found these known split blockers:
  - `suite/window_headers.rs` imports `super::compact::COMPACT_WARNING_MESSAGE`.
  - `suite/compact_resume_fork.rs` imports `super::compact::{FIRST_REPLY, SUMMARY_TEXT}`.
  - `suite/shell_serialization.rs` imports `crate::suite::apply_patch_cli::{apply_patch_harness, mount_apply_patch}`.
- `hooks.rs`, `approvals.rs`, `abort_tasks.rs`, `request_permissions.rs`, and
  `request_permissions_tool.rs` are non-Windows-gated in `suite/mod.rs`.
  `windows_sandbox.rs` is Windows-gated.
- Prior verification handoff says broad `codex-core` and workspace release
  tests should wait because current compile blockers exist elsewhere in the
  active SOLID refactor.

## Proposed commit sequence

### Commit 1: finish the shared test-binary bootstrap shape

Owner: harness/root worker.

Files:

- `codex-rs/core/tests/all.rs`
- `codex-rs/core/tests/suite_bootstrap.rs`
- a new non-auto-discovered support module path such as
  `codex-rs/core/tests/support/mod.rs` plus
  `codex-rs/core/tests/support/suite_bootstrap.rs`

Implementation intent:

- Move the existing bootstrap code out of root `tests/suite_bootstrap.rs` so
  Cargo does not auto-discover it as a standalone integration binary.
- Wire `all.rs` through `mod support;` or an equivalent `#[path = "..."]`
  include that can be reused by future top-level test binaries.
- Keep the bootstrap code local to integration-test wrappers for now. Do not
  push it into `core_test_support` in this commit unless the dependency impact
  has been explicitly reviewed.

Verify:

- Static: confirm `codex-rs/core/tests/suite_bootstrap.rs` no longer exists as
  a root-level `tests/*.rs` file.
- Static: confirm `all.rs` still includes the bootstrap module before `mod suite;`.
- Do not run broad `--test all` just to validate this cleanup while known compile
  blockers remain.

### Commit 2: add the first canary split binary for `client_websockets`

Owner: websocket/client worker for the new wrapper and feature file; harness/root
worker serializes the `suite/mod.rs` registry removal.

Files:

- `codex-rs/core/tests/client_websockets.rs` (new wrapper)
- `codex-rs/core/tests/suite/client_websockets.rs`
- `codex-rs/core/tests/suite/mod.rs` (single-line removal, harness/root only)
- shared support module from commit 1, only if wrapper syntax needs a tiny fix

Implementation intent:

- Create a top-level wrapper that re-exports `codex_protocol::error`, includes
  the shared bootstrap support, and includes
  `suite/client_websockets.rs` with `#[path = "suite/client_websockets.rs"]`.
- Remove `mod client_websockets;` from `suite/mod.rs` in the same commit so the
  tests are not compiled and run twice.
- Pick this before heavier lanes because it has no observed `super::`/`crate::`
  dependency on other suite modules and exercises websocket/client dependencies
  without platform gating.

Verify:

- Compile first, using the smallest release-profile lane:
  `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -ExtraCargoArgs @('--test','client_websockets','--no-run')`
- If that compiles and the current refactor blockers are not hit, run only that
  binary:
  `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -ExtraCargoArgs @('--test','client_websockets')`
- Do not run `cargo test --release -p codex-core` without `--test`.

### Commit 3: split the RMCP lane

Owner: RMCP worker for wrapper and feature file; harness/root serializes the
registry removal.

Files:

- `codex-rs/core/tests/rmcp_client.rs` (new wrapper)
- `codex-rs/core/tests/suite/rmcp_client.rs`
- `codex-rs/core/tests/suite/mod.rs` (single-line removal, harness/root only)

Implementation intent:

- Repeat the canary wrapper pattern for `rmcp_client.rs`.
- This validates a lane that uses process/server helpers and resource lookup
  without mixing in the large client or compaction files.

Verify:

- First: release `--no-run` for `--test rmcp_client`.
- Then: run `--test rmcp_client` only if compile succeeds.
- If resource lookup fails, fix the wrapper/support path before expanding to
  other lanes.

### Commit 4: split protocol/client conversation lanes one at a time

Owner: conversation/client worker. Do not combine with compaction or exec work.

Candidate order:

1. `realtime_conversation.rs`
2. `client.rs`
3. `code_mode.rs`

Files:

- one new top-level wrapper per module
- the matching `codex-rs/core/tests/suite/*.rs` file
- `suite/mod.rs` registry line removal, performed by harness/root only

Implementation intent:

- Keep each large module as its own binary unless a compile error proves it
  needs a partner module.
- Do not assign `client.rs` and `client_websockets.rs` to different workers at
  the same time unless the wrapper ownership is already finalized, because their
  conceptual surface overlaps.

Verify:

- For each binary, run release `--no-run` first.
- Run the split binary only after it compiles.
- Stop at the first compile error and repair the wrapper/support dependency
  instead of falling back to the aggregate `all` binary.

### Commit 5: split compaction as a coordinated lane

Owner: compaction worker only.

Files:

- `codex-rs/core/tests/suite/compact.rs`
- `codex-rs/core/tests/suite/compact_remote.rs`
- `codex-rs/core/tests/suite/compact_remote_parity.rs`
- `codex-rs/core/tests/suite/compact_resume_fork.rs`
- `codex-rs/core/tests/suite/window_headers.rs`
- new wrapper files for the chosen compaction binaries
- `suite/mod.rs` registry removals, harness/root only

Implementation intent:

- Handle compaction as one ownership lane because `compact_resume_fork.rs` and
  `window_headers.rs` import constants from `compact.rs`.
- Prefer extracting tiny shared constants/helpers into a compaction-local support
  module under `tests/support/` or grouping dependent modules in the same
  wrapper over duplicating constants.
- Do not let another worker edit these files while this lane is active.

Verify:

- Compile each new compaction test binary with `--no-run`.
- Run one narrow smoke filter per new binary before attempting all tests in the
  compaction lane.
- Do not run the old `all` aggregate as a substitute for fixing split compile
  errors.

### Commit 6: split apply-patch, shell, exec, and sandbox lanes

Owner: exec/sandbox worker only.

Files:

- `codex-rs/core/tests/suite/apply_patch_cli.rs`
- `codex-rs/core/tests/suite/shell_serialization.rs`
- `codex-rs/core/tests/suite/shell_command.rs`
- `codex-rs/core/tests/suite/shell_snapshot.rs`
- `codex-rs/core/tests/suite/unified_exec.rs`
- `codex-rs/core/tests/suite/exec.rs`
- `codex-rs/core/tests/suite/exec_policy.rs`
- `codex-rs/core/tests/suite/windows_sandbox.rs`
- other sandbox-adjacent files only after the first exec wrapper compiles
- `suite/mod.rs` registry removals, harness/root only

Implementation intent:

- Split `apply_patch_cli.rs` and `shell_serialization.rs` together or extract
  their shared helper into support first, because `shell_serialization.rs`
  imports helpers from `apply_patch_cli.rs`.
- Keep Windows-only and non-Windows-only wrappers explicitly gated.

Verify:

- Use release `--no-run` for each wrapper first.
- Run only the wrapper that just split, with platform-appropriate expectations.
- On Windows, do not claim Linux sandbox/hook coverage from a zero-test gated
  binary.

### Commit 7: split non-Windows hooks, approvals, and permission request lanes last

Owner: hooks/approval worker only.

Files:

- `codex-rs/core/tests/suite/hooks.rs`
- `codex-rs/core/tests/suite/hooks_mcp.rs`
- `codex-rs/core/tests/suite/approvals.rs`
- `codex-rs/core/tests/suite/request_permissions.rs`
- `codex-rs/core/tests/suite/request_permissions_tool.rs`
- `codex-rs/core/tests/suite/abort_tasks.rs`
- `suite/mod.rs` registry removals, harness/root only

Implementation intent:

- Leave these until the wrapper/support pattern has proven stable. They are
  large, platform-gated, and likely to consume the most iteration time.
- Use explicit `#[cfg(not(target_os = "windows"))]` at the wrapper or module
  boundary so Windows local checks do not misrepresent coverage.

Verify:

- On Windows: static compile shape only, expecting gated tests to be absent.
- On non-Windows CI or remote executor: release `--no-run`, then run each new
  split binary.

### Commit 8: shrink and validate the remaining aggregate

Owner: harness/root worker.

Files:

- `codex-rs/core/tests/all.rs`
- `codex-rs/core/tests/suite/mod.rs`
- any final wrapper files
- Cargo/Bazel metadata only if the root owner decides explicit entries are
  required

Implementation intent:

- Once enough heavy lanes are split, leave `all.rs` as a thin residual aggregate
  or remove it only when every suite module has a standalone binary and Cargo/
  Bazel behavior is understood.
- Avoid a manifest-wide `autotests = false` change unless root is prepared to
  enumerate every intended integration test binary explicitly.

Verify:

- Run targeted split binaries first.
- Only after targeted lanes are green, run the remaining aggregate `--test all`
  if it still exists.
- Then consider broader release verification.

## Future worker ownership map

- Harness/root worker:
  - owns `codex-rs/core/tests/all.rs`, `codex-rs/core/tests/suite/mod.rs`,
    shared `codex-rs/core/tests/support/**`, root-level wrapper naming rules,
    Cargo/Bazel metadata, lockfiles, formatting policy, staging, commits, and
    pushes.
  - serializes all `suite/mod.rs` removals to prevent merge conflicts.
- Websocket/client worker:
  - owns `codex-rs/core/tests/suite/client_websockets.rs` and
    `codex-rs/core/tests/client_websockets.rs`.
  - does not edit `client.rs` unless reassigned.
- RMCP worker:
  - owns `codex-rs/core/tests/suite/rmcp_client.rs` and
    `codex-rs/core/tests/rmcp_client.rs`.
- Conversation/client worker:
  - owns `realtime_conversation.rs`, `client.rs`, `code_mode.rs`, and their
    wrappers, one module at a time.
- Compaction worker:
  - owns `compact*.rs`, `window_headers.rs`, and compaction wrapper/support
    files.
- Exec/sandbox worker:
  - owns `apply_patch_cli.rs`, `shell_*`, `unified_exec.rs`, `exec*.rs`,
    `windows_sandbox.rs`, and exec/sandbox wrappers.
- Hooks/approval worker:
  - owns `hooks*.rs`, `approvals.rs`, `request_permissions*.rs`,
    `abort_tasks.rs`, and non-Windows gated wrappers.
- Everyone:
  - do not create nested workspaces, new `Cargo.lock` files, `target/`
    directories, or path dependencies between sibling workspace crates.
  - do not edit another worker's feature file to remove a compile error; hand
    the error back to that owner or root.

## What to verify after each commit

- Before implementation commits: static/read-only checks only.
- After Rust source edits: run `just fmt` from `codex-rs` before committing,
  per repo rules. Keep `just fix -p codex-core` for larger finalized slices,
  not for this scout.
- For every new split binary:
  - first run release `--no-run` through `scripts\test-local-codex-release.ps1`
    with `-Package codex-core -ExtraCargoArgs @('--test','<binary>','--no-run')`.
  - then run only that split binary with `-ExtraCargoArgs @('--test','<binary>')`.
  - if runtime is high, use a narrow test filter after the binary compiles.
- After a lane with shared support extraction:
  - verify dependent wrappers compile before moving the next module out of
    `suite/mod.rs`.
- After platform-gated splits:
  - on Windows, state clearly when verification only proves the gated wrapper
    compiles or is absent.
  - use Linux/non-Windows CI or remote executor before claiming hook/approval
    behavior is green.
- After enough split lanes are green:
  - run the residual aggregate `--test all` only if it still exists and only
    after root confirms current compile blockers are resolved.

## What not to run until the split structure exists

- Do not run `cargo test -p codex-core` or `cargo test --release -p codex-core`
  without an explicit `--test <split_binary>` target.
- Do not run the old broad `--test all` lane as the first verification for a
  new wrapper.
- Do not run workspace-wide `cargo test --release`, broad `just test`, or broad
  debug-profile Cargo tests.
- Do not run `scripts\build-local-codex.ps1 -Mode FastRelease` to validate test
  splitting; it is a binary-build lane and prior handoff says it is currently
  red elsewhere.
- Do not run Bazel lock/check/update or schema generation for this split unless
  root changes dependencies, manifests, generated schema inputs, or Bazel test
  metadata.
- Do not run formatters/fixers from scout sessions or before the implementation
  worker has completed the Rust edits for its slice.
