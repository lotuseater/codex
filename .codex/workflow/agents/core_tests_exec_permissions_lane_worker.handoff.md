# core_tests_exec_permissions_lane_worker Handoff

Status: source edit complete for owned exec/permissions wrapper lane.

Date: 2026-05-20

## Scope

Owned wrapper files:

- `codex-rs/core/tests/exec.rs`
- `codex-rs/core/tests/permissions.rs`

Owned suite files observed in this lane:

- `abort_tasks.rs`
- `apply_patch_cli.rs`
- `exec.rs`
- `exec_policy.rs`
- `shell_command.rs`
- `shell_serialization.rs`
- `shell_snapshot.rs`
- `unified_exec.rs`
- `user_shell_cmd.rs`
- `windows_sandbox.rs`
- `approvals.rs`
- `hooks.rs`
- `request_permissions_tool.rs`
- `review.rs`
- `tool_harness.rs`

The prompt also listed `codex-rs/core/tests/suite/permissions.rs`, but that file
does not exist in the current checkout and is not tracked by Git.

## Edits

- Kept `codex-rs/core/tests/exec.rs` as the top-level binary for the owned
  exec/sandbox modules.
- Kept the `shell_serialization.rs` dependency on `apply_patch_cli.rs` inside
  the same `exec` binary by using the existing `super::apply_patch_cli` imports.
- Removed unowned `hooks_mcp`, `permissions_messages`, and
  `request_permissions` module references from `codex-rs/core/tests/permissions.rs`
  so the wrapper now includes only the owned permissions-side modules.

## Follow-Up Ownership Notes

- `hooks_mcp.rs`, `permissions_messages.rs`, and `request_permissions.rs` still
  need to be assigned to another wrapper/worker; they are intentionally not kept
  in this lane because they were not in the owned edit path list.
- Full wrapper coverage is therefore not expected to be complete from this lane
  alone until the non-owned permission modules are routed elsewhere.

## Verification

Completed:

```powershell
just fmt
```

Result: passed, 64 files left unchanged.

```powershell
# Structural wrapper check: compare observed #[path = "suite/*.rs"] refs
# against the owned exec and permissions module lists.
```

Result: passed.

Attempted release no-run check:

```powershell
& .\scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs @('--test','exec','--no-run')
```

Result: blocked before the `exec` test binary compiled because the shared
`codex-core` library currently has unrelated compile errors outside this owned
lane, including missing imports such as
`crate::hook_runtime::run_user_prompt_submit_hooks`,
`codex_protocol::permissions::project_roots_glob_pattern`,
`crate::tools::spec_plan::build_tool_router`, and missing
`codex_app_server_protocol` linkage.

Log:

- `logs/test-local-release-codex-core-all-20260521-001749.log`

Skipped:

- The matching `permissions --no-run` check, because it would compile the same
  failing shared `codex-core` library first.
