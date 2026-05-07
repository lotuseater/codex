# Local Codex Fork Action Plan

## Rollout

Use `scripts/build-local-codex.ps1` as the canonical build + deploy entrypoint
for this repo. (The legacy `clean-fast-release-build.ps1`,
`clean-fast-release-local.ps1`, and `install-local-codex-fork.ps1` were
removed 2026-05-04 — superseded by build-local-codex's mode-and-action
matrix and disk-space defenses.)

Default install (build + deploy at FastRelease):

```powershell
.\scripts\build-local-codex.ps1 -Mode FastRelease
```

Verify current install (no build, no deploy):

```powershell
.\scripts\build-local-codex.ps1 -Mode Status
```

Re-deploy from an existing release exe (skip build):

```powershell
.\scripts\build-local-codex.ps1 -Mode DeployOnly
```

Rollback:

```powershell
.\scripts\build-local-codex.ps1 -Mode Rollback
```

Build only, no deploy (e.g. when testing the binary directly):

```powershell
.\scripts\build-local-codex.ps1 -Mode FastRelease -SkipDeploy
```

The script builds `codex-rs\target\release\codex.exe`, creates a timestamped
backup under `C:\Users\Oleh\.codex\binary-backups`, updates
`WIZARD_CODEX_REAL_EXE`, and verifies that the user-facing `codex` command
still enters through `C:\Users\Oleh\.codex\system-wrapper`.

Available build modes (smaller to larger memory + time):

| Mode | When to use |
|---|---|
| `LowMemRelease` | same shared release profile, lower RAM/disk-aware job count |
| `FastRelease` | same shared release profile, default deploy build |
| `FullRelease` | compatibility alias for the shared release profile |

All local modes reuse the same release profile from `.cargo/config.toml`.
`DevRelease`/`dev-small` is intentionally blocked in this checkout with
`Build only release!` because switching profiles creates another large target
tree and breaks cache reuse. The script also runs a disk-space pre-check,
evicts known-regeneratable artifacts when below 8 GB free, and aborts before
starting if still below 5 GB so we don't lose 30+ minutes to a doomed disk-out
condition mid-link.

## Implementation Priorities

1. Token-cost visibility: add lightweight reports around existing token usage, compaction, deferred tool loading, and output truncation. Keep the first slice read-only or reporting-focused.
2. Automation seams: prefer app-server v2 APIs for Team App and Wizard orchestration. Extend app-server only when an external controller cannot observe or drive a needed state.
3. MCP/cache behavior: keep tool metadata and cache work inside `codex-rs/codex-mcp` and related tests. Avoid routing Wizard-specific policy into `codex-core`.
4. PowerShell robustness: extend `codex-rs/shell-command` and unified exec tests for Windows command extraction, UTF-8 output, long output, and wrapper-launched behavior.

## Verification Matrix

- Install path: `Get-Command codex -All` shows `C:\Users\Oleh\.codex\system-wrapper` first.
- Binary target: `system.codex-wrapper.env.json` points `WIZARD_CODEX_REAL_EXE` at this repo's `codex-rs\target\release\codex.exe`.
- Startup: `codex --version` exits 0 through the wrapper.
- Interactive session header: the first Codex header line includes `Wizard_Codex_April_29_2_49`.
- Optional model smoke: build the binary then run `& $env:USERPROFILE\.codex\local-builds\codex-custom-*\codex.exe --version`.
- Rollback: `.\scripts\build-local-codex.ps1 -Mode Rollback` restores the prior wrapper env and reruns the same verification.

## Guardrails

- Do not edit npm or dotnet shims directly.
- Do not remove Wizard wrapper hooks while installing the fork.
- Do not commit machine-local backup artifacts.
- Keep local Wizard policy outside this repo; Codex changes should expose neutral APIs or diagnostics.
