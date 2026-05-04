# Local Codex Build Optimization Notes

Date: 2026-04-30

## Current Observations

- The local build target is already scoped to the binary with `cargo build -p codex-cli --bin codex`.
- `scripts/build-local-codex.ps1 -Mode FastRelease` sets:
  - `CARGO_PROFILE_RELEASE_LTO=off`
  - `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16`
  - `CARGO_INCREMENTAL=0` (release builds don't use incremental; setting this avoids cargo creating a multi-GB `target/release/incremental` dir of pure scratch)
- That still leaves the release profile at `opt-level=3`. During the current build, long-running rustc children were observed compiling crates such as `codex_app_server_protocol`, `codex_config`, `aws_sdk_sts`, `tokio`, and `rustls` at release optimization.
- `--jobs 1` reduces peak memory but also serializes the whole dependency graph. It is appropriate when rustc is failing with memory allocation errors, but it makes a cold or partly cold release build very long.
- The high-cost mistake was mixing build lanes. For this workflow, keep the release cache hot and avoid actions that force debug/test rebuilds unless debug artifacts are explicitly needed.
- Earlier release build output was not saved, so exact failure analysis depended on transient terminal output. Future long builds should always write stdout and stderr to a repo-local log.

## Recommended Build Lanes

### Custom Footer Smoke Check

Use this first for the local session-limit footer customization when no Rust source has changed since the last successful build:

```powershell
.\scripts\test-session-limit-footer.ps1
```

What it verifies:

- The footer plumbing and percentage calculation hooks are still present in source.
- The footer percentage formatting and its direct calculation test live in `codex-rs/tui/src/chatwidget/session_limit_footer.rs` instead of being embedded in the large chat widget module.
- The focused render snapshot lives in `codex-rs/tui/src/chatwidget/tests/session_limit_footer.rs`, separate from broader status/layout tests.
- The accepted snapshot still contains the right-aligned `tokens` and `reset` footer text.
- The side-conversation snapshot still combines the side-context label with the
  `tokens` and `reset` footer text instead of replacing the custom footer.
- The currently configured copied Codex executable starts with `--version`.
- The system-wide `codex` wrapper resolves to a working executable and its `--help` output reaches the interactive CLI entrypoint.

This does not compile Rust and is intentionally decoupled from the broad `codex-tui` test graph.

### Operation Cache Smoke Check

Use this for the custom operation-cache bridge and Codex-side interceptor:

```powershell
.\scripts\test-operation-cache.ps1
```

What it verifies:

- The Wizard Python bridge CLI can store and read cache entries.
- Codex/Claude canonicalization parity still holds for the selected read/grep bridge tests.
- The Rust `operation_cache` unit tests compile and pass through the `codex-core` lib-test lane.

Important build detail:

- Use `cargo test -p codex-core --lib operation_cache --release` for this lane.
- Do not use only `cargo test -p codex-core operation_cache --release`; Cargo still compiles the package integration test binaries before applying the test name filter, which can trigger long release LTO work unrelated to this feature.
- The script keeps the release target directory but disables release LTO for local verification unless `-StrictRelease` is passed.
- Avoid `-StrictRelease` for routine local operation-cache tests. On this machine,
  the strict release `codex-core --lib operation_cache` lane still spent roughly
  an hour in full-LTO codegen before the warmed rerun could execute the two
  filtered tests.

For a no-build end-to-end check against the currently active system wrapper,
use:

```powershell
.\scripts\test-operation-cache-runtime.ps1
```

This creates a temporary canary file under `logs\cache-canaries`, runs
`codex exec` twice, verifies the second run increments the Codex cache hit count
in the shared Wizard cache DB, checks that a failed cacheable read does not
create a cache row, asserts the failed read really exited nonzero, and then
removes the temporary canary file plus its successful cache row and exact
miss-telemetry rows. It is the preferred quick runtime check after a copied
binary has already been activated.

### Direct Release Test Harness

When `cargo test ... --release` has already produced a fresh test executable
but the parent shell timed out before the harness summary was written, do not
start another Cargo compile just to rerun the same filtered tests. Run the
release test executable directly:

```powershell
$exe = Get-ChildItem .\codex-rs\target\release\deps -Filter 'codex_tui-*.exe' |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1
& $exe.FullName session_limit_footer --nocapture
```

For the core operation-cache lane:

```powershell
$exe = Get-ChildItem .\codex-rs\target\release\deps -Filter 'codex_core-*.exe' |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1
& $exe.FullName operation_cache --nocapture
& $exe.FullName exec_command_tool_output_success_for_logging_tracks_exit_code --nocapture
```

This verifies the already-built harness and avoids rebuilding the package graph.
Use Cargo again only when Rust sources changed after that executable was built.

### Fast Functional Verification

Use this only when release-profile verification is not required and a quick local smoke binary is enough:

```powershell
.\scripts\build-local-codex.ps1 -Mode DevRelease -SkipDeploy -Jobs 1
```

Why:

- Uses the existing `dev-small` profile: `opt-level=0`, `debug=0`, `strip=true`.
- Produces `codex-rs\target\dev-small\codex.exe`.
- Avoids release optimization cost.
- Good enough for a quick local TUI smoke check, but not the preferred lane when validating release-only work.

### Release-Like Local Verification

Use this only when a release-profile binary is specifically needed:

```powershell
.\scripts\build-local-codex.ps1 -Mode FastRelease -SkipDeploy -Jobs 1
```

Notes:

- This is faster than full release because LTO is disabled and codegen units are increased.
- It is still expensive because `opt-level=3` remains in effect.
- If memory allows, consider `-Jobs 2` after checking no other large build is active. On this machine, `-Jobs 1` is safer when rustc previously failed with allocation errors.
- If the command may exceed the tool timeout, run it as a detached process with
  stdout/stderr redirected to `logs/` and monitor the process separately. A
  timed-out parent shell can otherwise leave Cargo running but lose final
  pass/fail output.

### Faster Release-Like Experiment

For local verification only, not for packaging/release parity, this should be faster than `FastRelease` while still using the release target directory:

```powershell
$env:CARGO_PROFILE_RELEASE_LTO = "off"
$env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "16"
$env:CARGO_PROFILE_RELEASE_OPT_LEVEL = "1"
cargo build -p codex-cli --release --bin codex --jobs 1
```

Tradeoff:

- This changes optimization level for the whole release profile in the current process.
- It should compile much faster than `opt-level=3`.
- The resulting binary is not equivalent to the normal release profile, so use it only for functional verification.

## Logging Rule

Every long build should save output before it starts. A safe PowerShell pattern is:

```powershell
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$log = Join-Path (Resolve-Path ..).Path "logs\fast-release-build-$stamp.log"
New-Item -ItemType Directory -Force -Path (Split-Path $log) | Out-Null

$env:RUST_MIN_STACK = "33554432"
$env:CARGO_PROFILE_RELEASE_LTO = "off"
$env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "16"

& cargo build -p codex-cli --release --bin codex --jobs 1 *> $log
$exit = $LASTEXITCODE
Get-Content -LiteralPath $log -Tail 80
exit $exit
```

When running from the repo root instead of `codex-rs`, set `$log` under `logs\...` directly and run cargo from `codex-rs`.

## Cleanup Policy

- For this workflow, optimize around `target/release`; do not spend time rebuilding debug artifacts unless explicitly needed.
- Prefer `-SkipClean` for release verification builds so completed release crates remain reusable.
- If disk cleanup is unavoidable, inspect active `cargo`, `rustc`, and `link` processes first and only remove generated directories under the intended `codex-rs\target` root.
- Prefer deleting narrow stale outputs over whole profiles:
  - old `.snap.new` files after review
  - stale logs
  - specific failed profile output, only when no build is active
- Keep `target/release` if retrying release builds; Cargo can reuse completed crate artifacts.
- If tests are needed for this workflow, prefer release-profile test commands for the touched crate instead of rebuilding the default debug test graph.

## Practical Next-Time Sequence

1. Check active build processes:

   ```powershell
   Get-Process | Where-Object { $_.ProcessName -match 'cargo|rustc|link' }
   ```

2. If the source did not change and the goal is only to confirm the custom footer lane, run:

   ```powershell
   .\scripts\test-session-limit-footer.ps1
   ```

3. If no build is active and Rust test coverage is needed, run focused release-profile tests:

   ```powershell
   cargo test -p codex-tui --lib session_limit_footer --release
   ```

4. For operation-cache verification, run:

   ```powershell
   .\scripts\test-operation-cache.ps1
   ```

5. For active-wrapper operation-cache verification without rebuilding, run:

   ```powershell
   .\scripts\test-operation-cache-runtime.ps1
   ```

6. For binary smoke verification without release requirements, build `DevSmall` with `-SkipClean`; otherwise keep using the release lane.
7. Run the produced `codex.exe --version`.
8. For interactive TUI verification, run that same binary with a temporary `log_dir`.
9. Only run `FastRelease` when release-profile behavior itself matters, and always capture a build log.
