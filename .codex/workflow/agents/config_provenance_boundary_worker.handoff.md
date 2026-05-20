# config_provenance_boundary_worker Handoff

Date: 2026-05-20

## Scope

Owned edit paths:

- `codex-rs/core/src/config/`
- `codex-rs/core/src/config.rs` (not present in this checkout; config lives under `codex-rs/core/src/config/`)
- `codex-rs/config-types/`

## Outcome

- Searched the owned paths for `codex_app_server_protocol::ConfigLayerSource` and app-server protocol provenance imports.
- No app-server protocol imports remain in the owned paths after the current refactor state.
- The domain owner type is `codex_config_types::ConfigLayerSource`, defined in `codex-rs/config-types/src/lib.rs`.
- `ConfigLayerSource::User` now carries `profile: Option<ProfileV2Name>` in the config-types owner type, with `ProfileV2Name` owned in `config-types`.
- Core config tests now use the non-protocol config-layer source type through the existing `codex_config` public API instead of importing `codex_app_server_protocol`.

## Boundary Note

`codex-core` does not currently have a direct `codex-config-types` dependency; it depends on `codex-config`. Because this worker is not allowed to edit manifests or lockfiles, I did not add a direct `codex-config-types` dependency to core. A future root-owned dependency-boundary slice can decide whether core should import `codex_config_types::ConfigLayerSource` directly by adding the manifest dependency, or keep using the existing `codex_config` re-export as the public config-domain API.

I did not add any compatibility re-export crutch in core.

## Unrelated Dirty Work Preserved

`codex-rs/config-types/src/lib.rs` also contains an unrelated dirty `HookEventName::SubagentStart` addition in the working tree. I treated it as another worker's change and did not stage it for this provenance-boundary commit.

## Verification

No cargo, just, bazel, build scripts, or test scripts were run during the refactor. Narrow static scans used for this slice:

- `rg -n "codex_app_server_protocol" codex-rs\\core\\src\\config codex-rs\\config-types`
- `rg -n "codex_app_server_protocol::ConfigLayerSource|use codex_app_server_protocol::ConfigLayerSource" codex-rs\\core\\src\\config codex-rs\\config-types`
- `Select-String -Path 'codex-rs\\core\\Cargo.toml','codex-rs\\config-types\\Cargo.toml' -Pattern 'codex-config-types|codex-config|name = "codex_config_types"'`

Recommended later package checks, once the broader refactor lane allows builds/tests:

- `powershell -ExecutionPolicy Bypass -File scripts\\test-local-codex-release.ps1 -Package codex-core -Filter config`
- If manifest ownership changes later add a direct dependency, run the focused crate check for that slice plus the repo's dependency-lock checks required by AGENTS.md.
