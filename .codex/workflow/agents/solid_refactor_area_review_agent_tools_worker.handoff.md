# solid_refactor_area_review_agent_tools_worker Handoff

## Findings

### P2 - Resume-descendant depth still bypasses the extracted spawn-depth policy

`codex-agent-policy` now owns saturating child-depth calculation: `codex-rs/agent-policy/src/lib.rs:45-46` implements `next_thread_spawn_depth(parent_depth)` with `saturating_add(1)`, and `codex-rs/agent-policy/src/lib.rs:49-50` owns the depth-limit comparison. The MultiAgentV2 spawn and explicit resume handlers use that boundary at `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs:73-83` and `codex-rs/core/src/tools/handlers/multi_agents_v2/resume_agent.rs:326-333`.

The recursive persisted-descendant resume path still computes depth manually: `codex-rs/core/src/agent/control.rs:526-542` queues `(thread_id, root_depth)`, then sets `let child_depth = parent_depth + 1;` at `codex-rs/core/src/agent/control.rs:541`. It immediately builds `SessionSource::SubAgent(SubAgentSource::ThreadSpawn { depth: child_depth, ... })` at `codex-rs/core/src/agent/control.rs:545-552` and passes it to `resume_single_agent_from_rollout` at `codex-rs/core/src/agent/control.rs:553-558`.

This is a missed callsite migration and can diverge from the policy boundary, including overflow behavior for an extreme or corrupted stored depth. Root-owned next action: replace the manual `parent_depth + 1` with the extracted raw-depth policy helper, for example `crate::agent::policy::next_thread_spawn_depth(parent_depth)`, and add focused coverage for the persisted descendant resume path.

### P3 - Telemetry preview ownership moved to `codex-tools`, but the owner crate still has no local policy tests

`codex-tools` owns the telemetry preview constants, policy type, and default helper at `codex-rs/tools/src/tool_output.rs:11-34`, `codex-rs/tools/src/tool_output.rs:36-77`, and `codex-rs/tools/src/tool_output.rs:283-285`; the API is re-exported from `codex-rs/tools/src/lib.rs:173-180`.

The behavioral tests remain in `codex-core`: `codex-rs/core/src/tools/context_tests.rs:394-408` covers byte truncation and `codex-rs/core/src/tools/context_tests.rs:412-422` covers line truncation. That gives integration coverage when `codex-core` is tested, but a focused `codex-tools` verification can compile the owner crate without exercising the policy it now owns.

Root-owned next action: move or duplicate the telemetry preview behavior tests into `codex-rs/tools/src/tool_output.rs` or a `codex-tools` integration test, then keep the core context tests as integration checks for core logging behavior.

### P3 - Compatibility re-exports and broad `codex-tools` dependency remain in place

The telemetry preview extraction does not reduce `codex-core` dependency fan-in yet. `codex-rs/core/Cargo.toml:135` still depends on the mixed `codex-tools` crate, while `codex-rs/tools/Cargo.toml:11-20` shows that crate still carries app-catalog, app-server-protocol, agent-policy, code-mode, protocol, tool-schema, path/pty/string utility, and RMCP dependencies.

Core compatibility re-exports remain too: `codex-rs/core/src/tools/context.rs:25-37` re-exports `ToolOutput`, `ToolPayload`, and `ToolCallSource` from `codex_tools`, and `codex-rs/core/src/tools/router.rs:26` re-exports `ToolCallSource`. Many core tool callsites still type against `crate::tools::context::ToolOutput` or `boxed_tool_output`; the read-only scan found examples at `codex-rs/core/src/tools/code_mode/execute_handler.rs:5`, `codex-rs/core/src/tools/code_mode/execute_handler.rs:92`, `codex-rs/core/src/tools/handlers/apply_patch.rs:21`, and `codex-rs/core/src/tools/handlers/apply_patch.rs:301`.

Root-owned next action: decide whether these re-exports are an intentional short-lived adapter. If the commit claims callsite migration or dependency reduction, migrate the remaining core tool callsites to the `codex_tools` API and remove the compatibility re-exports; otherwise document this as transitional in the commit boundary.

## Non-Blocking Observations

- `codex-rs/core/src/agent/policy.rs:1-23` is a private core adapter over `codex-agent-policy`, not a public compatibility re-export. That shape looks acceptable for keeping core callsites local while policy ownership moves out.
- `codex-rs/tools/src/tool_call_source.rs:1-15` is a small domain type with no new heavy dependency by itself. The broader issue is the remaining core compatibility re-export, not this enum's implementation.
- No source edits, staging, commits, pushes, Cargo, rustc, just, Bazel, scripts, schema generation, or tests were run by this worker.

## Exact Commit Boundary For This Area

Recommended split if root wants clean commits:

1. Agent policy/depth boundary commit:
   - `codex-rs/agent-policy/src/lib.rs`
   - `codex-rs/core/src/agent/policy.rs`
   - `codex-rs/core/src/agent/mod.rs`
   - `codex-rs/core/src/agent/registry.rs`
   - `codex-rs/core/src/agent/control.rs`
   - `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`
   - `codex-rs/core/src/tools/handlers/multi_agents_v2/resume_agent.rs`
   - `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`

2. Tools telemetry preview / tool-source boundary commit:
   - `codex-rs/tools/src/tool_output.rs`
   - `codex-rs/tools/src/tool_call_source.rs`
   - `codex-rs/tools/src/lib.rs`
   - `codex-rs/core/src/tools/context.rs`
   - `codex-rs/core/src/tools/context_tests.rs`
   - `codex-rs/core/src/tools/router.rs`
   - Any remaining `codex-rs/core/src/tools/handlers/multi_agents_v2/{compact_agent,restart_agent,resume_agent}.rs` one-line tool-output signature/import fallout if root keeps those with the tools API cleanup.

Do not include unrelated schema files or unrelated Cargo.lock/Cargo.toml drift in this area commit unless root confirms those manifest changes are required by these exact source changes. If `codex-rs/core/Cargo.toml` or `codex-rs/Cargo.lock` are included, root must also refresh/check Bazel lock state per repo rules.

## Root-Owned Verification Commands

After applying the depth callsite fix and adding owner-crate telemetry tests:

```powershell
Push-Location codex-rs
just fmt
Pop-Location
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-agent-policy
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-tools -Filter telemetry_preview
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Filter multi_agent_v2
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Filter telemetry_preview
```

If manifest/lock changes remain in the commit:

```powershell
Push-Location codex-rs
just bazel-lock-update
just bazel-lock-check
Pop-Location
```

Final lint/fix pass before root commits:

```powershell
Push-Location codex-rs
just fix -p codex-agent-policy
just fix -p codex-tools
just fix -p codex-core
Pop-Location
```
