# Local Codex Build Incidents

This note records build failures seen on this Windows checkout so future attempts do not repeat
expensive or unsafe lanes.

## Current Rules

- Use release profile only. Debug builds/tests have repeatedly exhausted C: disk.
- Preserve `codex-rs/target/release`; it is the useful shared cache.
- Do not prune individual `target/release/deps` hash generations. Cargo depinfo/fingerprints can
  still reference older hashed artifacts, and deleting only the dep file can poison later tests.
- It is safe to delete release `.pdb` files on this checkout when disk is tight. They are debug
  symbols, not inputs needed to run the deployed `codex.exe`.
- Avoid broad `cargo test -p codex-cli --release <filter>` because Cargo still builds all CLI
  integration-test targets. Prefer `cargo test -p codex-cli --release --bin codex <filter> -j 1`.
- Avoid broad `cargo test -p codex-core --release <filter>` for unit-test filters because Cargo can
  still compile integration-test dependencies. Prefer
  `cargo test -p codex-core --release --lib <filter> -j 1` for module unit tests.
- Cargo accepts only one test-name filter before `--`. For multiple focused tests, use a common
  module/prefix filter such as `session::checkpoint_policy::tests`, or run separate Cargo commands.
- Do not start a second build while `build-local-codex.ps1 -Mode Status` reports active Cargo,
  rustc, link, or cmd processes for this repo.
- Treat an active Codex Cargo command without an absolute repo path, such as
  `cargo check -p codex-core --release`, as a competing build. It still
  consumes Cargo locks, memory, and release-cache headroom.
- If `build-local-codex.ps1` reports a release-profile/toolchain stamp mismatch, either keep the
  old cache and stop, or intentionally rebuild one clean generation with
  `-ResetReleaseCacheOnProfileChange`; do not prune individual hashed files in
  `target/release/deps`.
- If `target/debug` appears, remove it with `build-local-codex.ps1 -Mode CleanSafe`; local schema
  and generated-artifact recipes must use release Cargo commands.

## Incidents

| Date | Command/log | Symptom | Action |
| --- | --- | --- | --- |
| 2026-04-30 | `logs/custom-footer-test-detached-20260430-221058.log` | Debug incremental query cache failed with `os error 112` disk full. | Do not use debug lanes for routine verification. |
| 2026-05-02 | `logs/focused-core-review-fixes-20260502-143702.log` | Debug build ran out of disk while compiling shared crates. | Keep debug target deleted on this machine. |
| 2026-05-02 | `logs/install-local-codex-fork-fastrelease-20260502.log` | Release build hit `os error 112` while writing `.rlib`/`.rmeta`. | Preflight disk and clean only non-release-cache artifacts. |
| 2026-05-05 | `logs/codex-cli-loop-release-test-20260505-144014.log` | Broad CLI release test hit paging-file error `os error 1455`, then metadata/prelude cascade. | Use `--bin codex` for CLI unit filters; avoid broad integration-test compilation. |
| 2026-05-05 | `logs/codex-tui-loop-release-test-20260505-154512.log` | TUI lib test spent 30+ minutes compiling toward `codex-core` without a useful result. | Avoid `cargo test -p codex-tui --release --lib` on this checkout unless explicitly needed. |
| 2026-05-05 | `logs/local-codex-build-lowmemrelease-20260505-161800.log` | Low-memory deploy build failed with `0xc0000409 STATUS_STACK_BUFFER_OVERRUN` while building/linking `codex-cli`. | Capture process snapshot and retry only after stale process cleanup and log review. |
| 2026-05-05 | `build-local-codex.ps1 -Mode Diagnose` | `target/release` reached 17.22 GB; `target/release/deps` had 8,179 stale hashed artifact generations reclaiming about 8.94 GB. | Added stale release-deps pruning to preflight/post-build cleanup and reduced `target/release` to 8.28 GB. |
| 2026-05-05 | `logs/local-codex-build-lowmemrelease-20260505-165637.log` | PowerShell append redirection wrote Cargo output as NUL-padded text and hid compile counts from `Progress`. | Switched build logging through `cmd.exe` append and added memory/pagefile diagnostics for future builds. |
| 2026-05-05 | `logs/test-core-checkpoint-policy-*.log` | `cargo test -p codex-core --release checkpoint_policy -j 1` failed before repo code because `target/release/deps/libthiserror-*.rmeta` had been pruned while `ts-rs` still referenced it. | Disabled release deps pruning; keep `Mode Diagnose` as reporting-only for duplicate generations. Repair with targeted `cargo clean -p thiserror -p thiserror-impl --release` if this appears again. |
| 2026-05-05 | `logs/test-core-checkpoint-policy-20260505-205832.log` | Broad `cargo test -p codex-core --release checkpoint_policy -j 1` compiled toward integration-test dependencies for 30 minutes and reduced C: free space to about 3.9 GB before timeout. | Use `--lib` for core unit-test filters. |
| 2026-05-06 | `cargo test --release -p codex-core <test-a> <test-b> ... -- --nocapture` | Cargo rejected the command with `unexpected argument` because it supports only one test filter before harness args. | Use one common prefix filter, or run each exact test filter as its own command. |
| 2026-05-07 | `just write-app-server-schema` before the release-only fix | The recipe used debug `cargo run`, creating 3.44 GB in `target/debug` while C: had about 1 GB free. | Switched generated-artifact Just recipes to `cargo run --release` and added `CleanSafe` cleanup. |
| 2026-05-07 | release test lanes after repeated focused checks | `target/release/deps` included 24 disposable test executables totaling about 2.18 GB, plus release PDBs. | `CleanSafe -CleanTestArtifacts` removes release test `.exe` files and matching PDBs only under explicit disk-pressure cleanup. |
| 2026-05-07 | `cargo check -p codex-core --release --quiet` overlapping a deploy build | A second Codex Cargo command was active while `FastRelease -Jobs 1` was compiling `codex-tui`, increasing memory and Cargo-cache pressure. | Extended `build-local-codex.ps1` process detection to catch Codex package commands even when their command line does not include the repo root. |

## Verification Lanes That Worked

- `cargo test -p codex-cli --release --bin codex loop -j 1`
  - Passed in `logs/codex-cli-loop-bin-release-test-20260505-144448.log`.
  - Passed again in `logs/codex-cli-loop-bin-release-test-20260505-153707.log`.
