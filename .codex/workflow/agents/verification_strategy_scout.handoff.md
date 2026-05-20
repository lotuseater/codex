# verification_strategy_scout Handoff

Status: completed read-only strategy pass on 2026-05-20.

## Scope

This handoff builds the smallest practical verification ladder for the next root
slice in the current moving tree. I did not run Cargo, Just, rustfmt, git
staging, or commits.

## Sources inspected

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/dab_availability_worker.handoff.md`
- `.codex/workflow/agents/canary_observer.handoff.md`
- `scripts/test-local-codex-release.ps1`
- `.codex/prototypes/check-core-boundaries.ps1`
- Focused read-only searches for `ExecCommandSessionEvent`, `CallBegin`, and
  `CallEnd`
- Existing logs under `logs\fast-release*.log`

## Current blocker accounting

- Prior blocker from the solid-refactor and canary handoffs: release compile
  failed in `codex-rs/core/src/client.rs` on removed
  `ExecCommandSessionEvent::CallBegin` / `CallEnd` variants.
- Current tree check: exact references to
  `ExecCommandSessionEvent::CallBegin` / `CallEnd` no longer match under
  `codex-rs/core/src`, and `ExecCommandSessionEvent` itself did not match in
  the focused core source search. Treat the prior blocker as likely edited, but
  not verified until a release compile passes.
- Current saved build artifacts also show non-compile blockers to watch:
  - `logs\fast-release-build-hotfix-20260520-043307.err.log`: build script
    refused to start while repo-local Cargo/rustc processes were active.
  - `logs\fast-release-build-hotfix-20260520-043640.err.log`: build script hit
    `Get-FileHash` not recognized in that run. `Get-FileHash` is available in
    this scout shell, so re-check the log and environment before treating this
    as a code issue.
  - `logs\fast-release-build-hotfix-20260520-043950.combined.log`: only shows
    the FastRelease disk pre-check/reclaim path so far.
- At scout time, no `cargo`, `rustc`, `link`, `rust-lld`, or `sccache` processes
  were visible, and `codex-rs\target\debug` was absent.
- `logs\release-tests` was absent; the release test script should create it on
  the first successful or failed test run.

## Recommended verification order

1. Preflight the Windows release lane.
   - Confirm there is no active repo-local Cargo/rustc/link process.
   - Confirm `codex-rs\target\debug` is still absent.
   - Check free disk before starting the release build. The latest FastRelease
     log saw only 5.21 GB free, below the 8 GB warning threshold.

2. Run the architecture boundary canary first. This is cheap and does not need
   Cargo:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .codex\prototypes\check-core-boundaries.ps1
   ```

3. Format Rust changes before verification:

   ```powershell
   Push-Location codex-rs
   just fmt
   Pop-Location
   ```

4. Run the narrow release test lane for the extracted core tool-spec-plan slice:

   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter tools::spec_plan_tests -CleanCoreLibTestArtifactsOnSuccess
   ```

   Notes:
   - Use the script, not direct `cargo test`, so release-only profile settings,
     logging, and cleanup behavior stay consistent.
   - The current script supports `-Lib`; use it for filtered codex-core lib
     tests.
   - Keep `-CleanCoreLibTestArtifactsOnSuccess` unless the root agent
     intentionally wants to inspect the generated core lib test executable.

5. Run the release build canary and capture a repo-local combined log:

   ```powershell
   $ts = Get-Date -Format yyyyMMdd-HHmmss
   $log = "logs\fast-release-build-solid-refactor-$ts.combined.log"
   powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode FastRelease *> $log
   if ($LASTEXITCODE -ne 0) {
       Get-Content $log -Tail 160
       exit $LASTEXITCODE
   }
   ```

   Use `-Mode LowMemRelease` only if FastRelease fails because of memory
   pressure. It uses the same release profile/cache with lower parallelism.

6. If the compile-blocker fix touched broader `codex-core` behavior outside the
   spec-plan extraction, add one focused release test for that behavior before
   the build canary. Do not jump straight to the full release suite unless the
   root agent explicitly approves it.

7. Before finalizing a large `codex-core` slice, run the scoped linter fix lane:

   ```powershell
   Push-Location codex-rs
   just fix -p codex-core
   Pop-Location
   ```

   Per repo guidance, do not rerun tests only because `fmt` or `fix` ran; inspect
   the resulting diff for unintended semantic edits.

## Commands to avoid for now

- Any debug-profile Cargo lane, including:

  ```powershell
  cargo test -p codex-core
  cargo test -p codex-cli
  cargo test -p codex-exec
  ```

- Direct filtered Cargo invocations that bypass the local release wrapper, for
  example:

  ```powershell
  cargo test -p codex-core --release --no-run tools::spec_plan_tests
  ```

  The earlier handoff observed this was the wrong shape for the intended local
  verification and still failed before tests.

- Workspace-wide release tests or `just test` until the targeted release lane and
  FastRelease build are green and the root agent explicitly approves the full
  suite.
- `--all-features` for routine local verification.
- Starting another FastRelease/LowMemRelease while repo-local Cargo/rustc/link
  processes are active. Inspect the existing process/log first.
- Manual deletion of release artifacts. Prefer:

  ```powershell
  powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode CleanSafe
  ```

  Add `-CleanTestArtifacts` only when disposable release test executables need
  cleanup. Do not use destructive git cleanup for build artifacts.

## Artifact and log paths to inspect

- `logs\fast-release-20260520-015544.log`: canary observer's original
  `ExecCommandSessionEvent::CallBegin` failure.
- `logs\fast-release-build-hotfix-20260520-043307.err.log`: active-build guard
  failure.
- `logs\fast-release-build-hotfix-20260520-043640.err.log`: `Get-FileHash` shell
  failure.
- `logs\fast-release-build-hotfix-20260520-043950.combined.log`: latest
  FastRelease pre-check/reclaim trace.
- `logs\release-tests\*.log`: release test logs generated by
  `scripts\test-local-codex-release.ps1`.
- `codex-rs\target\debug`: should remain absent.
- `codex-rs\target\release\deps`: inspect only if release test cleanup reports
  leftover large test executables.
- `.codex/workflow/agents/canary_observer.exec.marker`: marker for the earlier
  canary observer run.

## DAB and visual verification

No DAB-dependent verification is required for this non-visual core slice.

If a later root slice needs live GUI verification, prefer an app-native harness
or browser automation first. The DAB availability worker found `.mcp.json`
wired to the Wizard bridge and verified these external DAB tools through the MCP
inspector: `dab_automation_timeline`, `dab_click`, `dab_eval`, `dab_key`,
`dab_screenshot`, `dab_type`, and `dab_windows`. The current Codex session did
not expose `dab_*` tools, so do not block this commit on in-session DAB
availability.

## Acceptance criteria for the next commit

- The prior `client.rs` release compile blocker is either proven fixed by the
  release build, or replaced by a newly diagnosed concrete blocker with a saved
  log.
- `.codex\prototypes\check-core-boundaries.ps1` passes.
- `scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter tools::spec_plan_tests -CleanCoreLibTestArtifactsOnSuccess` passes and writes a release test log.
- `scripts\build-local-codex.ps1 -Mode FastRelease` passes with a non-empty
  saved combined log inspected after completion.
- No `codex-rs\target\debug` directory is created.
- No large disposable release test executables are left behind unexpectedly.
- `just fmt` has been run after Rust edits; `just fix -p codex-core` has been run
  if the final slice is large enough to warrant the scoped lint lane.
- The commit contains only the coherent verified slice and does not stage
  unrelated dirty work from the moving tree.
