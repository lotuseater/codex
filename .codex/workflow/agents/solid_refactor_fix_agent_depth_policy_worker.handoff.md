# solid_refactor_fix_agent_depth_policy_worker Handoff

## Findings

- Fixed the P2 policy-boundary leak in recursive persisted-descendant resume.
- `codex-rs/core/src/agent/control.rs` no longer computes child depth with `parent_depth + 1`; it now calls `crate::agent::policy::next_thread_spawn_depth(parent_depth)`, keeping `codex-core` as the adapter over `codex-agent-policy`.
- `codex-rs/agent-policy/src/lib.rs` already contains focused owner-crate coverage for normal depth increments, saturating `i32::MAX`, and max-depth comparison.

## Files Changed

- `codex-rs/core/src/agent/control.rs`
- `.codex/workflow/agents/solid_refactor_fix_agent_depth_policy_worker.handoff.md`

Relevant pre-existing dirty files for this slice, not authored here:

- `codex-rs/agent-policy/src/lib.rs`
- `codex-rs/core/src/agent/policy.rs`
- `codex-rs/core/src/agent/mod.rs`
- `codex-rs/core/src/agent/registry.rs`
- `codex-rs/core/src/agent/graph_store.rs`

## Verification

- `just fmt` from `codex-rs`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-agent-policy`: passed.
- `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Filter multi_agent_v2`: blocked by wrapper guard requiring `-Lib`.
- `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter multi_agent_v2`: blocked by wrapper guard requiring `-AllowBroadCoreLibUnitTests`.
- `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter multi_agent_v2 -AllowBroadCoreLibUnitTests`: `codex-core` compiled, then failed in `core_test_support` because `codex-rs/core/tests/common/test_codex.rs` imports `codex_thread_store` and `codex_thread_store_api` while `codex-rs/core/tests/common/Cargo.toml` does not declare those dependencies. Log: `logs/test-local-release-codex-core-multi_agent_v2-20260521-125357.log`.

## Commit / Push

- No commit or push was made because focused core verification is blocked by the dirty `core_test_support` dependency issue outside this worker's ownership.

## Exact Next Action

- The owner of `codex-rs/core/tests/common/test_codex.rs` / `codex-rs/core/tests/common/Cargo.toml` should reconcile the missing `codex-thread-store` and `codex-thread-store-api` dependencies, then rerun:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter multi_agent_v2 -AllowBroadCoreLibUnitTests
```
