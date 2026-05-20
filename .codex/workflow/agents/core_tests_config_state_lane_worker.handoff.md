# core_tests_config_state_lane_worker Handoff

Date: 2026-05-20
Status: complete; release check blocked by non-owned shared-lib compile errors

## Scope

Owned edit paths:

- `codex-rs/core/tests/config.rs`
- `codex-rs/core/tests/state.rs`
- `.codex/workflow/agents/core_tests_config_state_lane_worker.handoff.md`

The worker prompt also listed suite files as owned for read/possible inclusion,
but this pass did not edit any suite files.

## Changes

Created self-contained top-level integration wrappers:

- `config.rs`
  - `code_mode`
  - `deprecation_notice`
  - `image_rollout`
  - `model_overrides`
  - `model_visible_layout`
  - `prompt_caching`
  - `rollout_list_find`
  - `safety_check_downgrade`
  - `unstable_features_warning`
- `state.rs`
  - `json_result`
  - `request_compression`
  - `truncation`
  - `turn_state`
  - `user_notification`

Each wrapper starts with `mod support;` and uses explicit `#[path =
"suite/<module>.rs"] mod <module>;` declarations so the suite modules compile as
top-level integration-test modules without recreating `suite/mod.rs`.

## Prompt-Listed Modules Not Included

These prompt-listed suite files do not exist in the current checkout, so this
worker did not create wrapper entries for them:

- `model_provider_info`
- `mode_apply_patch`
- `project_doc`
- `rollout`
- `conversation_history`
- `session_manager`
- `task_started_completed`

## Boundary Notes

The preexisting untracked wrappers included additional modules such as
`models_cache_ttl`, `models_etag_responses`, `model_switching`,
`override_updates`, `personality`, `personality_migration`, `prompt_debug_tests`,
`quota_exceeded`, `remote_env`, `remote_models`, `items`, `otel`,
`sqlite_state`, `stream_error_allows_next_turn`, and `stream_no_completed`.
Those modules were not listed in this worker prompt, so this worker left them
for their owning lanes instead of keeping them in `config.rs` or `state.rs`.

## Verification

Completed:

- `just fmt` from `codex-rs`
- wrapper path sanity check: all `#[path]` targets referenced by `config.rs`
  and `state.rs` exist

Attempted:

```powershell
$extra = @('--test','config','--test','state')
& .\scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs $extra
```

Result: failed before the `config` or `state` integration-test targets could run
because the shared `codex-core` library currently does not compile in this
worktree. Latest log inspected:
`logs/test-local-release-codex-core-all-20260521-002522.log`.

Representative non-owned compile blockers from that log:

- unresolved imports including `run_user_prompt_submit_hooks`,
  `project_roots_glob_pattern`, `build_tool_router`, `spec_plan_types`,
  `codex_app_server_protocol`, and `ToolHandler`
- missing/changed thread-store symbols such as `LocalThreadStoreConfig` and
  `LocalThreadStore`
- function arity mismatches unrelated to the new test wrappers
