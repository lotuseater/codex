# core_compile_tools_worker Handoff

## Status

Implemented the tool-router/spec-plan compile-blocker repair in the tools lane. Changes are left unstaged because the single allowed release check is blocked before `codex-core` by an unrelated `codex-otel` compile error.

## Files Changed

Intentional changes by this worker:

- `codex-rs/core/src/tools/registry.rs`
- `codex-rs/core/src/tools/router.rs`
- `codex-rs/core/src/tools/mod.rs`
- `codex-rs/core/src/tools/parallel.rs`
- `codex-rs/core/src/tools/tool_dispatch_trace_tests.rs`
- `codex-rs/core/src/tools/code_mode/execute_handler.rs`
- `codex-rs/core/src/tools/code_mode/wait_handler.rs`
- `codex-rs/core/src/tools/handlers/**/*.rs` executor impls that were missing `type Output`

There were already shared dirty files under `codex-rs/core/src/tools/**` from other lanes; I did not revert them.

## Compile Blockers Fixed

- Restored `crate::tools::spec` and `crate::tools::spec_plan_types` module exports.
- Replaced the missing `build_tool_router` call with local router assembly through `collect_tool_router_parts` and `build_tool_registry_builder_from_executors`.
- Reintroduced the registry builder boundary (`ToolRegistryBuilder`) and a `RegisteredTool` object-safe adapter for router storage/dispatch.
- Kept `CoreToolRuntime` as the core metadata trait and added `ToolHandler` as a compatibility alias for handlers still using the older name.
- Added `type Output = Box<dyn crate::tools::context::ToolOutput>` to remaining boxed-output `ToolExecutor` impls under `core/src/tools`.
- Removed `async_trait` from `ToolExecutor` impls that now target the RPITIT trait shape.

## Dependency Or Manifest Need

No dependency, manifest, or lockfile update is needed from this lane.

## Verification

- `just fmt` in `codex-rs`: passed.
- Static scan: all `impl ToolExecutor<...>` blocks under `codex-rs/core/src/tools` now declare `type Output`.
- `git diff --check -- codex-rs/core/src/tools .codex/workflow/agents/core_compile_tools_worker.handoff.md`: passed, with existing CRLF warnings only.
- `cargo check -p codex-core --release --lib`: attempted once as the focused release check and failed before checking `codex-core`.

Release-check blocker:

- Log: `logs/codex-core-tools-router-release-check-20260521-021708.log`
- Error: `codex-otel` fails at `otel/src/events/session_telemetry.rs:1173` with `error[E0004]` because `ResponseEvent::Incomplete { .. }` is not covered.
- This blocker is outside the assigned tool-router/spec-plan lane.

## Commit

No commit created.

Commit blocker: verification did not reach `codex-core` because the allowed release check is blocked by the unrelated `codex-otel` exhaustive-match error above. Changes are intentionally left unstaged for root integration.
