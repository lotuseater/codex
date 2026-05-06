---
name: codex-rust-build
description: Use when building, testing, diagnosing, or deploying this local Windows Codex Rust checkout; especially when failures involve Cargo cache reuse, release-only builds, disk pressure, paging-file errors, rustc/link crashes, or system-wrapper deployment.
---

# Codex Rust Build

Use this skill before any build/test/deploy work in this checkout.

## Workflow

1. Inspect live state first:
   - `powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode Status`
   - Do not start another Cargo build while repo-local cargo/rustc/link/cmd processes are active.
2. Preserve useful cache:
   - keep `codex-rs\target\release`
   - do not delete `target\release\build`, `target\release\gn_out`, or `.fingerprint` unless the user explicitly accepts a slower rebuild
   - safe cleanup targets are `target\debug`, `target\dev-small`, and `target\release\incremental`
3. Pick the narrowest release lane:
   - CLI parser/unit checks: `cargo test -p codex-cli --release --bin codex <filter> -j 1`
   - Cargo accepts only one test filter before `--`. Never pass multiple full test names in one command; use a common module/prefix filter or run the filters sequentially.
   - Core unit checks on this checkout should use `--lib`, for example `cargo test -p codex-core --release --lib session::checkpoint_policy::tests -j 1 -- --nocapture`.
   - Deploy build: `powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode LowMemRelease -Jobs 1`
   - Do not use broad debug tests.
   - Avoid `cargo test -p codex-tui --release --lib` on this machine unless the task requires it; it has pulled in expensive core compilation.
4. Capture and inspect logs:
   - build logs live under `logs\local-codex-build-*.log`
   - targeted test logs should be tee'd into `logs\...`
   - check `docs\local-build-incidents.md` before retrying a failed lane.

## Failure Handling

- `os error 112`: disk full. Preserve release cache and reclaim debug/dev-small/incremental first.
- `os error 1455`: paging file too small. Stop broad test lanes and use single-job, bin-scoped checks.
- `STATUS_STACK_BUFFER_OVERRUN`: inspect the build log and active process snapshot before retrying; do not immediately rerun the same command.
- Zero-byte or truncated logs are build-system defects; improve logging before another long attempt.
