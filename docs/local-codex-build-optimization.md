# Local Codex Build Optimization Notes

Date: 2026-04-30

## Current Observations

- The local build target is scoped to the binary with `cargo build --release -p codex-cli --bin codex`.
- `scripts/build-local-codex.ps1 -Mode FastRelease` reuses the shared release profile from `codex-rs\.cargo\config.toml`:
  - `lto = "off"`
  - `codegen-units = 64`
  - `opt-level = 2`
  - `debug = 0`
  - `strip = "symbols"`
  - `incremental = false`
- Keeping these settings in Cargo config, instead of per-command environment overrides, makes all local release builds/tests/checks reuse one artifact shape.
- `build-local-codex.ps1` records a release-profile stamp under
  `codex-rs\target\release\.codex-local-release-profile.json` after a
  successful local release build. Future build-script runs compare the current
  `.cargo\config.toml` hash and `rustc -Vv` output against that stamp.
- If the release profile or toolchain changed, the build script stops before
  Cargo so it does not silently create another large `target\release`
  generation. Use `-ResetReleaseCacheOnProfileChange` when intentionally
  accepting a one-time clean release rebuild.
- During recent builds, long-running rustc children were observed compiling crates such as `codex_app_server_protocol`, `codex_config`, `aws_sdk_sts`, `tokio`, and `rustls` at release optimization.
- `--jobs 1` reduces peak memory but also serializes the whole dependency graph. It is appropriate when rustc is failing with memory allocation errors, but it makes a cold or partly cold release build very long.
- The high-cost mistake was mixing build lanes. For this workflow, keep the release cache hot and avoid actions that force debug/test rebuilds.
- Another high-cost mistake is overlapping release commands. The build script
  now detects Codex Cargo command lines such as `cargo check -p codex-core` even
  when the command line omits the repo path, and refuses to start a second build.
- The release-only rustc wrapper can now chain an optional inner wrapper. If
  `sccache` is installed, `build-local-codex.ps1 -UseSccache` preserves the
  release-only guard while invoking `sccache rustc ...` underneath it.
- On this Windows machine, `sccache` 0.15.0 is installed through winget as
  `Mozilla.sccache`; the command alias resolves through
  `%LOCALAPPDATA%\Microsoft\WinGet\Links\sccache.exe`.
- Because only C: is available locally and free space is tight, the build script
  caps `SCCACHE_CACHE_SIZE` at `2G` when `-UseSccache` is passed and no explicit
  user or machine cache size is configured. The current user environment also
  has `SCCACHE_CACHE_SIZE=2G` set, so manually-started sccache servers use the
  same cap.
- Local generated-artifact recipes must use `cargo run --release`; a debug `just write-app-server-schema` run created 3.44 GB of `target/debug` artifacts on 2026-05-07.
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
- The script keeps the release target directory and uses the shared release
  profile from `codex-rs\.cargo\config.toml`, so it does not create a second
  release artifact shape.

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

### Safe Disk Cleanup

```powershell
.\scripts\build-local-codex.ps1 -Mode CleanSafe
```

What it removes:

- `codex-rs\target\debug`
- `codex-rs\target\dev-small`
- `codex-rs\target\release\incremental`
- release PDB files

When C: is still under pressure, add `-CleanTestArtifacts` to remove disposable
release test executables under `target\release\deps`. Do not delete release
`.rlib`, `.rmeta`, `.d`, `build`, `gn_out`, or `.fingerprint` files by hand;
Cargo fingerprints can still reference them.

### Release-Like Local Verification

Use this only when a release-profile binary is specifically needed:

```powershell
.\scripts\build-local-codex.ps1 -Mode FastRelease -SkipDeploy -Jobs 1
```

Notes:

- This reuses the same release profile as deploy builds, so it preserves cache compatibility.
- It is still more expensive than the feature harness because it may compile the CLI and all required release dependencies.
- If memory allows, consider `-Jobs 2` after checking no other large build is active. On this machine, `-Jobs 1` is safer when rustc previously failed with allocation errors.
- If the command stops with a release-profile/toolchain mismatch, do not delete
  random files under `target\release\deps`. Re-run the same command with
  `-ResetReleaseCacheOnProfileChange` only when you intentionally want to drop
  the old release cache and rebuild one clean generation.
- If a clean or partly cold rebuild is expected, add `-UseSccache`. Keep it
  opt-in until this checkout has enough measured hit-rate data to make it the
  default.
- If the command may exceed the tool timeout, run it as a detached process with
  stdout/stderr redirected to `logs/` and monitor the process separately. A
  timed-out parent shell can otherwise leave Cargo running but lose final
  pass/fail output.

## Logging Rule

Every long build should save output before it starts. A safe PowerShell pattern is:

```powershell
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$log = Join-Path (Resolve-Path ..).Path "logs\fast-release-build-$stamp.log"
New-Item -ItemType Directory -Force -Path (Split-Path $log) | Out-Null

& cargo build -p codex-cli --release --bin codex --jobs 1 *> $log
$exit = $LASTEXITCODE
Get-Content -LiteralPath $log -Tail 80
exit $exit
```

When running from the repo root instead of `codex-rs`, set `$log` under `logs\...` directly and run cargo from `codex-rs`.

## Cleanup Policy

- For this workflow, optimize around `target/release`; do not spend time rebuilding debug artifacts unless explicitly needed.
- Use `CleanSafe` for release verification cleanup so completed release crates remain reusable.
- If disk cleanup is unavoidable, inspect active `cargo`, `rustc`, and `link` processes first and only remove generated directories under the intended `codex-rs\target` root.
- Prefer deleting narrow stale outputs over whole profiles:
  - old `.snap.new` files after review
  - stale logs
  - disposable release test executables via `CleanSafe -CleanTestArtifacts`
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

6. If `target/debug` exists or C: is low, run `build-local-codex.ps1 -Mode CleanSafe`; add `-CleanTestArtifacts` only under disk pressure.
7. Run the produced `codex.exe --version`.
8. For interactive TUI verification, run that same binary with a temporary `log_dir`.
9. Only run `FastRelease` when release-profile behavior itself matters, and always capture a build log.
