# protocol_schema_scout Handoff

Status: completed read-only scout on 2026-05-20.

## Scope Read

- Read `.codex/workflow/solid-refactor-handoff.md`.
- The requested `codex-rs/app-server-protocol/src/protocol/v2.rs` does not exist in this tree. The current v2 protocol root is `codex-rs/app-server-protocol/src/protocol/v2/mod.rs`, which re-exports directory modules.
- Read `codex-rs/app-server-protocol/src/protocol/v2/mod.rs`.
- Read `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs`.
- Read `codex-rs/app-server-protocol/src/protocol/v2/shared.rs` and `codex-rs/app-server-protocol/src/protocol/v2/config.rs` for adjacent v2 schema-bearing ownership.
- Read `codex-rs/config-types/src/lib.rs`.
- Spot-checked `codex-rs/config/src/config_toml.rs` because config schema generation is driven by `ConfigToml`, which consumes the moved config/permission types.

## Current Impact Summary

- The refactor does affect protocol ownership: config-layer protocol data is already domain-owned by `codex_config_types` and re-exported from app-server v2; approval policy config data is domain-owned by `codex_permission_types`.
- I did not find a required app-server v2 wire shape change from the refactor itself. The safer implementation target is ownership/import/conversion changes while preserving v2 structs, enum names, serde representation, `JsonSchema`, and `TS` annotations.
- Schema generation risk exists because several moved/domain-owned types are still schema-bearing v2 API types or config schema inputs. Moving a type is safe only if its exported TypeScript name/path and serde shape remain byte-for-byte compatible.

## Exact Protocol Types Affected

### v2 permissions API

`codex-rs/app-server-protocol/src/protocol/v2/permissions.rs` is the main API adapter for permission/refactor ownership. It currently maps v2 wire types to domain/core types imported from `codex_protocol::approvals`, `codex_protocol::models`, `codex_protocol::permissions`, and `codex_protocol::request_permissions`.

Schema-bearing v2 types to preserve if their domain owners move:

- `NetworkApprovalProtocol` via `v2_enum_from_core!`
- `NetworkApprovalContext`
- `AdditionalFileSystemPermissions`
- `FileSystemAccessMode` via `v2_enum_from_core!`
- `FileSystemSpecialPath`
- `FileSystemPath`
- `FileSystemSandboxEntry`
- `PermissionProfileListParams`
- `PermissionProfileSummary`
- `PermissionProfileListResponse`
- `ActivePermissionProfile`
- `AdditionalPermissionProfile`
- `GrantedPermissionProfile`
- `NetworkAccess`
- `SandboxPolicy`
- `ExecPolicyAmendment`
- `NetworkPolicyRuleAction` via `v2_enum_from_core!`
- `NetworkPolicyAmendment`
- `PermissionsRequestApprovalParams`
- `PermissionGrantScope` via `v2_enum_from_core!`
- `PermissionsRequestApprovalResponse`

Notable stable wire constraints:

- Most v2 permissions types use `#[serde(rename_all = "camelCase")]` and `#[ts(export_to = "v2/")]`.
- `FileSystemSpecialPath` and `FileSystemPath` are tagged with `kind`; moving them must preserve tags, variant names, field names, and `ts-rs` tags.
- `ExecPolicyAmendment` is `#[serde(transparent)]` and `#[ts(type = "Array<string>", export_to = "v2/")]`; do not replace it with a differently named/generated object.
- `SandboxPolicy` has custom deserialize compatibility logic and rejects old `workspaceWrite.readOnlyAccess` with a compatibility message. Preserve that behavior if ownership moves.
- `PermissionsRequestApprovalParams` fields are `thread_id`, `turn_id`, `item_id`, `started_at_ms`, `cwd`, `reason`, and `permissions`, with camelCase wire output and `startedAtMs` typed as number for TS.
- `PermissionsRequestApprovalResponse` fields are `permissions`, `scope`, and optional `strict_auto_review`; `scope` has serde default and `strict_auto_review` has serde default/skip plus `#[ts(optional)]`.
- `protocol/common.rs` exposes `item/permissions/requestApproval` using `v2::PermissionsRequestApprovalParams` and `v2::PermissionsRequestApprovalResponse`; the method name and payload names should stay stable.

### v2 shared config/approval/sandbox mirrors

`codex-rs/app-server-protocol/src/protocol/v2/shared.rs` owns v2 mirror types that are easy to confuse with domain config types:

- `AskForApproval`
  - `#[serde(rename_all = "kebab-case")]`
  - `#[ts(rename_all = "kebab-case", export_to = "v2/")]`
  - `UnlessTrusted` is explicitly `#[serde(rename = "untrusted")]` and `#[ts(rename = "untrusted")]`
  - `Granular` is experimental and contains `sandbox_approval`, `rules`, `skill_approval`, `request_permissions`, and `mcp_elicitations`
- `ApprovalsReviewer`
  - TS type is the literal union `"user" | "auto_review" | "guardian_subagent"` exported to `v2/`
  - serde accepts `guardian_subagent` with alias `auto_review`
  - custom `JsonSchema` allows `user`, `auto_review`, and `guardian_subagent`
- `SandboxMode`
  - `#[serde(rename_all = "kebab-case")]`
  - `#[ts(rename_all = "kebab-case", export_to = "v2/")]`
  - variants are `ReadOnly`, `WorkspaceWrite`, `DangerFullAccess`

Do not replace these v2 mirrors with domain types unless the generated v2 TS/schema remains identical. The domain `codex_permission_types::AskForApproval` currently has no `export_to = "v2/"` annotation, so directly exposing it through v2 would likely change TS ownership/output.

### v2 config API

`codex-rs/app-server-protocol/src/protocol/v2/config.rs` imports `AskForApproval`, `ApprovalsReviewer`, and `SandboxMode` from v2 shared and re-exports:

- `codex_config_types::ConfigLayer`
- `codex_config_types::ConfigLayerMetadata`
- `codex_config_types::ConfigLayerSource`

Affected v2 config payloads include:

- `ProfileV2`, fields `approval_policy` and `approvals_reviewer`
- `Config`, fields `approval_policy`, `approvals_reviewer`, `sandbox_mode`, `sandbox_workspace_write`, and `profiles`
- `ConfigReadResponse`, fields `config`, `origins`, and `layers`
- `OverriddenMetadata`, because it contains `ConfigLayerMetadata`

`ConfigLayerSource`, `ConfigLayerMetadata`, and `ConfigLayer` live in `codex_config_types` but still export TS to `v2/`. If these move again, preserve `#[ts(export_to = "v2/")]`, `#[serde(rename_all = "camelCase")]`, and the tagged `ConfigLayerSource` shape.

## Exact Config Types Affected

`codex-rs/config-types/src/lib.rs` contains schema-bearing config/domain types relevant to the refactor:

- `AskForApproval` is imported from `codex_permission_types` and is used by `UserSavedConfig::approval_policy` and `Profile::approval_policy`.
- `SandboxMode`
  - derives `JsonSchema` and `TS`
  - `#[serde(rename_all = "kebab-case")]`
  - explicitly serializes `ReadOnly` as `read-only`
- `ApprovalsReviewer`
  - derives `JsonSchema` via custom implementation
  - serializes `User` as `user`
  - serializes `AutoReview` as `guardian_subagent` and accepts alias `auto_review`
  - schema intentionally lists `user`, `auto_review`, and `guardian_subagent`
- `UserSavedConfig`
  - camelCase serde
  - fields include `approval_policy`, `sandbox_mode`, and `sandbox_settings`
- `Profile`
  - fields include `approval_policy`
- `SandboxSettings`
  - camelCase serde
  - fields are `writable_roots`, `network_access`, `exclude_tmpdir_env_var`, and `exclude_slash_tmp`
- `ConfigLayerSource`
  - tagged with `type`, camelCase variant fields, `#[ts(export_to = "v2/")]`
  - variants include `Mdm`, `System`, `User`, `Project`, `Profile`, `SessionFlags`, `LegacyManagedConfigTomlFromFile`, and `LegacyManagedConfigTomlFromMdm`
- `ConfigLayerMetadata`
  - camelCase serde, `#[ts(export_to = "v2/")]`
  - fields `name` and `version`
- `ConfigLayer`
  - camelCase serde, `#[ts(export_to = "v2/")]`
  - fields `name`, `version`, `config`, and optional `disabled_reason`

Downstream config schema inputs in `codex-rs/config/src/config_toml.rs` include:

- `ConfigToml::approval_policy: Option<AskForApproval>`
- `ConfigToml::approvals_reviewer: Option<ApprovalsReviewer>`
- `ConfigToml::sandbox_mode: Option<SandboxMode>`
- `ConfigToml::sandbox_workspace_write`
- `ConfigToml::default_permissions`
- `ConfigToml::permissions`

## Schema Regeneration Need

- `just write-config-schema`: needed after implementation if any commit changes `ConfigToml` or the schema/serde behavior of nested config types above, including `codex_permission_types::AskForApproval`, `SandboxMode`, `ApprovalsReviewer`, `SandboxSettings`, profile/user config structs, or permission-profile config fields. If the implementation only changes imports/conversions and leaves those shapes unchanged, it should produce no schema diff, but running it is still the right verification before committing a config-shape slice.
- `just write-app-server-schema`: needed after implementation if any commit changes app-server v2 payload structs/enums, re-exported schema-bearing types, experimental markers, `ts-rs` annotations, serde rename/tag/default/skip behavior, or method params/responses. This includes changes to `permissions.rs`, `shared.rs`, `config.rs`, or the re-exported `ConfigLayer*` types in `codex_config_types`.
- `just write-app-server-schema --experimental`: needed if implementation touches experimental v2 fields/types such as `AskForApproval::Granular`, `ProfileV2::approval_policy`, `Config::approval_policy`, `Config::profiles`, `ConfigReadResponse::config`, or `approvalsReviewer`.
- If implementation only removes core dependencies on app-server protocol by changing callers to domain-owned types, and app-server v2 payload definitions are unchanged, schema generation should be a no-op rather than a required fixture update.

## Likely Test Lanes

Do not run these in the scout task; this is for the implementing/root agent.

- Protocol-only shape/fixture work:
  - `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server-protocol`
  - `just write-app-server-schema`
  - `just write-app-server-schema --experimental` when experimental v2 fields are touched
- Config schema/domain work:
  - `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-config-types`
  - `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-config`
  - `just write-config-schema`
- Permission domain ownership work:
  - `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-permission-types`
  - rerun protocol/config lanes above if app-server v2 or config schema uses the moved type directly
- App-server behavior smoke when config RPC or permission request payloads are touched:
  - `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server`

## Commit Readiness Notes

- This scout made no source, manifest, generated schema fixture, Cargo, Just, staging, or commit changes.
- The only write is this handoff file.
- A commit that only changes ownership imports/conversions is ready from a schema standpoint when:
  - v2 public type names and method names are unchanged;
  - serde/ts/schemars attributes listed above moved with any moved type;
  - app-server schema generation is either not needed or produces no unexpected diff;
  - config schema generation is either not needed or produces no unexpected diff;
  - targeted release test lanes for touched crates are green.
- Do not start the broader app-server protocol cleanup with MCP elicitation types, `ThreadHistoryBuilder`, or `TurnStatus`; the root handoff explicitly calls those out for separate boundary review.
