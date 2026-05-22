# SOLID Refactor Wave 21 Core Lib Fallout Repair Worker Handoff

classification: root-wiring-needed

## Files changed

- `codex-rs/core/src/session/review.rs`
- `codex-rs/core/src/session/turn_context.rs`
- `codex-rs/core/src/tools/handlers/multi_agents.rs`
- `codex-rs/core/src/tools/handlers/multi_agents/close_agent.rs`
- `codex-rs/core/src/tools/handlers/multi_agents/resume_agent.rs`
- `codex-rs/core/src/tools/handlers/multi_agents/send_input.rs`
- `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`
- `codex-rs/core/src/tools/handlers/multi_agents/wait.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_v2.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_v2/close_agent.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_v2/compact_agent.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_v2/list_agents.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_v2/restart_agent.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_v2/resume_agent.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs`
- `.codex/workflow/agents/solid_refactor_wave21_core_lib_fallout_repair_worker.handoff.md`

## Repair summary

- Restored `ToolsConfig` and `ToolsConfigParams` imports in the two session files from `codex_tools`.
- Updated multi-agent `ToolOutput` implementations to match the current `codex_tool_execution_api::ToolOutput` trait shape by accepting `&dyn ToolOutputPayload` for response/code-mode output conversions.
- Added `ToolOutputPayload` imports in the multi-agent module roots so child handlers can use it through `use super::*`.

## Checks run

- `rg -n "ToolOutput|ToolOutputPayload|ToolPayload|fn payload|ToolsConfig|ToolsConfigParams" codex-rs/core/src/session/review.rs codex-rs/core/src/session/turn_context.rs codex-rs/core/src/tools/handlers/multi_agents.rs codex-rs/core/src/tools/handlers/multi_agents_spec.rs codex-rs/tools-domain/tool-execution-api/src/lib.rs`
  - Result: passed; expected imports/usages are present.
- `rg -n "fn to_response_item\(&self, call_id: &str, payload: &ToolPayload\)|fn code_mode_result\(&self, _payload: &ToolPayload\)|fn post_tool_use_input\(&self, _payload: &ToolPayload\)|fn post_tool_use_response\(" codex-rs/core/src/tools/handlers/multi_agents codex-rs/core/src/tools/handlers/multi_agents_v2`
  - Result: passed; no stale multi-agent output signatures remain.
- `git diff --check -- codex-rs/core/src/session/review.rs codex-rs/core/src/session/turn_context.rs codex-rs/core/src/tools/handlers/multi_agents.rs codex-rs/core/src/tools/handlers/multi_agents_spec.rs codex-rs/core/src/tools/handlers/multi_agents codex-rs/core/src/tools/handlers/multi_agents_v2.rs codex-rs/core/src/tools/handlers/multi_agents_v2 .codex/workflow/agents/solid_refactor_wave21_core_lib_fallout_repair_worker.handoff.md`
  - Result: passed; only existing Windows line-ending warnings were printed.
- `$log = "logs\wave21-codex-core-lib-fallout-repair.log"; cargo check --manifest-path codex-rs\Cargo.toml --release -p codex-core --lib *> $log`
  - Result: failed after the process completed; log path: `logs/wave21-codex-core-lib-fallout-repair.log`.
  - Failure is currently in excluded `codex-tool-registry-api` wave20/root-owned files before `codex-core` can be checked:
    - unresolved imports in `tools-domain/tool-registry-api/src/lib.rs` for context/first-moves/agent tool exports;
    - missing `defer_loading` in `ResponsesApiTool` initializers in `tools-domain/tool-registry-api/src/tool_discovery.rs`.

## Remaining gaps

- `codex_tools::{ToolsConfig, ToolsConfigParams}` remains a temporary architecture gap because those config types are still sourced from `codex_tools`.
- Root/tool-registry wiring is needed before the release-profile `codex-core` lib check can prove this worker's source repair end-to-end.
- Commit skipped: adjacent multi-agent handler files already had concurrent dirty hunks before this repair, and the release-profile check is not green due to excluded upstream registry errors.
