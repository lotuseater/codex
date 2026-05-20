# Compile Hook Skill Scout Handoff

Date: 2026-05-20

## Scope

Read-only scout for current `codex-core` compile blockers involving hook runtime and skill dependency symbols. I did not edit source files, run Cargo, run Just, run formatters, or stage/commit Git changes. This file is the only write.

Required prior handoffs read:

- `.codex/workflow/agents/dab_availability_worker.handoff.md`
- `.codex/workflow/solid-refactor-handoff.md`

Targeted source paths inspected:

- `codex-rs/core/src/hook_runtime.rs`
- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/tasks/mod.rs`
- `codex-rs/core/src/lib.rs`
- `codex-rs/core/src/skills.rs`
- `codex-rs/core-skills/src/lib.rs`
- `codex-rs/core-skills/src/env_var_dependencies.rs`
- `codex-rs/core/src/mcp_skill_dependencies.rs`
- `codex-rs/hooks/src/registry.rs`
- `codex-rs/hooks/src/events/user_prompt_submit.rs`
- `codex-rs/config-types/src/lib.rs`
- `codex-rs/features/src/lib.rs`

## Exact Missing Symbols

### `crate::hook_runtime::PendingInputHookDisposition`

References:

- `codex-rs/core/src/session/turn.rs:27` imports it.
- `codex-rs/core/src/session/turn.rs:441` expects `Accepted(pending_input)`.
- `codex-rs/core/src/session/turn.rs:444` expects `Blocked { additional_contexts }`.
- `codex-rs/core/src/tasks/mod.rs:29` imports it.
- `codex-rs/core/src/tasks/mod.rs:623` expects `Accepted(pending_input)`.
- `codex-rs/core/src/tasks/mod.rs:626` expects `Blocked { additional_contexts }`.

No definition exists in `codex-rs` for this symbol.

Current source of truth appears to be `HookRuntimeOutcome`, not the old disposition enum:

- `codex-rs/core/src/hook_runtime.rs:47-50` defines `HookRuntimeOutcome { pub should_stop: bool, pub additional_contexts: Vec<String> }`.
- `codex-rs/core/src/hook_runtime.rs:443-447` defines `inspect_pending_input(...) -> HookRuntimeOutcome`.
- `codex-rs/core/src/hook_runtime.rs:450-467` already builds a `UserPromptSubmitRequest`, previews `user_prompt_submit`, and runs `hooks.run_user_prompt_submit(request)` for pending user input.
- `codex-rs/core/src/hook_runtime.rs:470-473` returns `should_stop: false` for `TurnInput::ResponseInputItem(_)`.

Related same-slice mismatch:

- `codex-rs/core/src/hook_runtime.rs:477-482` defines `record_pending_input(..., pending_input: TurnInput, additional_contexts: Vec<String>)`.
- The stale callsites still call it with no `additional_contexts` argument at `codex-rs/core/src/session/turn.rs:462` and `codex-rs/core/src/tasks/mod.rs:624`.

Recommendation: do not restore `PendingInputHookDisposition` unless the implementer intentionally wants a compatibility shim. The cleaner fix is to remove the stale enum import/match and use `HookRuntimeOutcome` directly:

- `outcome.should_stop == false` maps to accepted pending input.
- `outcome.should_stop == true` maps to blocked pending input.
- `outcome.additional_contexts` must be passed to `record_pending_input` when accepted, or to `record_additional_contexts` when blocked.

### `crate::hook_runtime::run_user_prompt_submit_hooks`

References:

- `codex-rs/core/src/session/turn.rs:33` imports it.
- `codex-rs/core/src/session/turn.rs:348` calls `run_user_prompt_submit_hooks(&sess, &turn_context, prompt).await`.
- `codex-rs/core/src/session/turn.rs:349-365` expects the return value to have `additional_contexts` and `should_stop` fields, so the return type should be `HookRuntimeOutcome`.

No function with this name exists in `codex-rs/core/src/hook_runtime.rs`.

Current source of truth is already present underneath:

- `codex-rs/hooks/src/events/user_prompt_submit.rs:22-30` defines `UserPromptSubmitRequest`.
- `codex-rs/hooks/src/events/user_prompt_submit.rs:33-37` defines `UserPromptSubmitOutcome` with `should_stop` and `additional_contexts`.
- `codex-rs/hooks/src/registry.rs:183-195` exposes `preview_user_prompt_submit` and `run_user_prompt_submit`.
- `codex-rs/config-types/src/lib.rs:755-765` includes `HookEventName::UserPromptSubmit`.
- `codex-rs/core/src/hook_runtime.rs:80-96` already maps `UserPromptSubmitOutcome` into `ContextInjectingHookOutcome`.
- `codex-rs/core/src/hook_runtime.rs:450-467` already contains the request construction and run path used for pending user input.

Recommendation: add `run_user_prompt_submit_hooks(sess, turn_context, prompt: &str) -> HookRuntimeOutcome` in `codex-rs/core/src/hook_runtime.rs`, using the same request fields and `run_context_injecting_hook` path already used by `inspect_pending_input`.

### `skills::collect_env_var_dependencies`

References:

- `codex-rs/core/src/lib.rs:92` re-exports `skills::collect_env_var_dependencies`.
- `codex-rs/core/src/session/turn.rs:12` imports `crate::collect_env_var_dependencies`.
- `codex-rs/core/src/session/turn.rs:256` calls `collect_env_var_dependencies(&mentioned_skills)`.

Current source of truth exists but is not wired:

- `codex-rs/core-skills/src/env_var_dependencies.rs:4-8` defines `SkillDependencyInfo`.
- `codex-rs/core-skills/src/env_var_dependencies.rs:10-30` defines `pub fn collect_env_var_dependencies(mentioned_skills: &[SkillMetadata]) -> Vec<SkillDependencyInfo>`.
- `codex-rs/core-skills/src/lib.rs:1-10` declares public modules for config, injection, loader, manager, model, remote, render, and system, but does not declare/export `env_var_dependencies`.
- `codex-rs/core/src/skills.rs:11-34` re-exports many `codex_core_skills` items, but not `collect_env_var_dependencies` or `SkillDependencyInfo`.

Recommendation: expose the existing owner instead of duplicating the collector:

1. In `codex-rs/core-skills/src/lib.rs`, add a module/export for `env_var_dependencies` and re-export `collect_env_var_dependencies` plus `SkillDependencyInfo`.
2. In `codex-rs/core/src/skills.rs`, re-export those symbols from `codex_core_skills` so `codex-rs/core/src/lib.rs:92` resolves.

### `skills::resolve_skill_dependencies_for_turn`

References:

- `codex-rs/core/src/lib.rs:98` re-exports `skills::resolve_skill_dependencies_for_turn`.
- `codex-rs/core/src/session/turn.rs:46` imports `crate::resolve_skill_dependencies_for_turn`.
- `codex-rs/core/src/session/turn.rs:257` calls `resolve_skill_dependencies_for_turn(&sess, &turn_context, &env_var_dependencies).await`.

No definition exists anywhere in the repo.

Likely current owner boundary:

- `codex-rs/core/src/session/turn.rs:252-258` gates this path behind `Feature::SkillEnvVarDependencyPrompt`.
- `codex-rs/features/src/lib.rs:1045-1049` defines `skill_env_var_dependency_prompt` as under development and disabled by default.
- `codex-rs/core-skills/src/env_var_dependencies.rs` owns the pure metadata extraction for `type == "env_var"` tool dependencies.
- `codex-rs/core/src/mcp_skill_dependencies.rs:34-82` owns the active prompt/install flow for `type == "mcp"` dependencies.
- `codex-rs/core/src/mcp_skill_dependencies.rs:431-432` explicitly filters only MCP tool dependencies, so it is not already the env-var resolver.
- `codex-rs/core/src/mcp_skill_dependencies.rs:218-276` is the closest existing request-user-input pattern if the env-var resolver also needs to prompt the user.

Recommendation: implement the env-var turn resolver in core, not core-skills, because it needs `Session`, `TurnContext`, and probably `request_user_input`/event behavior. Best ownership options:

- Preferred: add a dedicated `codex-rs/core/src/skill_env_var_dependencies.rs` module and re-export through `core/src/skills.rs` or `core/src/lib.rs`.
- Acceptable small fix: implement `pub(crate) async fn resolve_skill_dependencies_for_turn(...)` in `codex-rs/core/src/skills.rs` if the function stays thin.
- Avoid folding env-var behavior into `mcp_skill_dependencies.rs` unless shared prompt helpers are extracted, because that file currently owns MCP install behavior specifically.

## Recommended Fix Order

1. Add `run_user_prompt_submit_hooks` in `codex-rs/core/src/hook_runtime.rs` returning `HookRuntimeOutcome`. This uses already-present `UserPromptSubmitRequest`, preview, run, and outcome conversion code.
2. Remove stale `PendingInputHookDisposition` usage in `codex-rs/core/src/session/turn.rs` and `codex-rs/core/src/tasks/mod.rs`. Treat `inspect_pending_input(...).await.should_stop` as the block decision and pass `additional_contexts` through to either `record_pending_input` or `record_additional_contexts`.
3. Wire `collect_env_var_dependencies` from `codex-rs/core-skills/src/env_var_dependencies.rs` through `codex-rs/core-skills/src/lib.rs` and `codex-rs/core/src/skills.rs`.
4. Restore `resolve_skill_dependencies_for_turn` as the env-var dependency prompt/resolution layer. Keep pure dependency extraction in `core-skills`; keep session/prompt side effects in `codex-core`.
5. Only after these four symbols are fixed, re-run the relevant release compile/test lane. This scout did not run verification by instruction.

## Files Likely Touched For Implementation

High confidence:

- `codex-rs/core/src/hook_runtime.rs`
- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/tasks/mod.rs`
- `codex-rs/core-skills/src/lib.rs`
- `codex-rs/core/src/skills.rs`

Medium confidence:

- `codex-rs/core/src/skill_env_var_dependencies.rs` if a new focused module is added for the env-var resolver.
- `codex-rs/core/src/lib.rs` if the new module is exported directly from core instead of through `skills.rs`.
- `codex-rs/core/src/mcp_skill_dependencies.rs` only if extracting shared request-user-input helper behavior.
- `codex-rs/core-skills/src/env_var_dependencies.rs` only if the collector needs case-insensitive matching or tests.
- `codex-rs/core/src/session/mod.rs` or `codex-rs/core/src/session/input_queue.rs` only if the implementer also resolves the adjacent pending-input type mismatch between older `ResponseInputItem` queue APIs and newer `TurnInput` hook APIs.

## Delegation Safety

Safe to delegate as implementation, but split it into two bounded slices if possible:

1. Hook runtime slice: `hook_runtime.rs`, `session/turn.rs`, `tasks/mod.rs`.
2. Skill env-var dependency slice: `core-skills/src/lib.rs`, `core/src/skills.rs`, and a small core resolver module or function.

Do not combine this with unrelated blockers from the DAB handoff such as plugin-install symbols, thread store/session API churn, or protocol permission glob symbols. The hook slice has moderate risk because pending-input queue types are mid-refactor; the skill slice is lower risk because the collector already exists and only the resolver is missing.
