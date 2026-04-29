# Local Codex Fork Action Plan

## Rollout

Use `scripts/install-local-codex-fork.ps1` as the only system-wide installer for this repo.

Default install:

```powershell
.\scripts\install-local-codex-fork.ps1
```

Verify current install:

```powershell
.\scripts\install-local-codex-fork.ps1 -Action Verify
```

Rollback:

```powershell
.\scripts\install-local-codex-fork.ps1 -Action Rollback
```

The installer builds `codex-rs\target\release\codex.exe`, creates a timestamped backup under `C:\Users\Oleh\.codex\binary-backups`, updates `WIZARD_CODEX_REAL_EXE`, and verifies that the user-facing `codex` command still enters through `C:\Users\Oleh\.codex\system-wrapper`.

The default build mode is `FastRelease`: it still writes `codex-rs\target\release\codex.exe`, but disables the repo's fat LTO/single-codegen-unit release settings through Cargo profile environment overrides so local rollout rebuilds are practical on this PC. Use `-BuildMode FullRelease` only when intentionally testing the slow upstream release profile.

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
- Optional model smoke: `.\scripts\install-local-codex-fork.ps1 -Action Verify -RunSmoke`.
- Rollback: `-Action Rollback` restores the prior wrapper env and then reruns the same verification.

## Guardrails

- Do not edit npm or dotnet shims directly.
- Do not remove Wizard wrapper hooks while installing the fork.
- Do not commit machine-local backup artifacts.
- Keep local Wizard policy outside this repo; Codex changes should expose neutral APIs or diagnostics.
