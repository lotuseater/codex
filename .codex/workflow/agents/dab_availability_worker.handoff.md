# DAB Availability Worker Handoff

## Scope

Fixed the internal Codex desktop automation registration path. This does not use or depend on external Wizard_Erasmus DAB/MCP wiring.

## Changed Paths

- `codex-rs/core/src/tools/handlers/mod.rs`
- `codex-rs/core/src/tools/handlers/desktop_automation.rs`
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/spec_plan_tests.rs`
- `codex-rs/core/src/tools/registry.rs`
- `.codex/workflow/agents/dab_availability_worker.handoff.md`

## Read Paths

- `codex-rs/tools/src/desktop_automation.rs`
- `codex-rs/tools/src/tool_registry_plan.rs`
- `codex-rs/tools/src/tool_registry_plan_tests.rs`
- `codex-rs/core/src/tools/registry.rs`
- `scripts/test-local-codex-release.ps1`
- `logs/test-local-release-codex-core-desktop_automation_tools_follow_desktop_automation_config-20260520-193346.log`
- `logs/test-local-release-codex-tools-desktop_automation_tools_respect_config-20260520-193528.log`

## Exact Cause

The internal DAB tool specs and native implementation already existed, and `codex-rs/tools/src/tool_registry_plan.rs` already registered `ToolHandlerKind::DesktopAutomation` when `desktop_automation_enabled` was true. The active core `build_specs` executor path had no matching desktop automation executor registration, so DAB tools could be model-visible in the plan path while `codex-core` had no local handler for `dab_find_window` and the other `dab_*` tools.

## Fix

- Exported `DesktopAutomationHandler` from the core handlers module.
- Converted/kept `DesktopAutomationHandler` on the `ToolExecutor<ToolInvocation>` + `CoreToolRuntime` path and made it own the model-visible `ToolSpec` it executes.
- Registered one `DesktopAutomationHandler` per `create_desktop_automation_tools(config.desktop_automation_allow_input)` result in `codex-rs/core/src/tools/spec_plan.rs`, gated by `config.desktop_automation_enabled`.
- Added a Windows-only focused core canary proving:
  - `dab_find_window` is absent from model specs when desktop automation is disabled.
  - the core registry has no `dab_find_window` handler when disabled.
  - `dab_find_window` is model-visible when enabled.
  - the core registry has a `dab_find_window` handler when enabled.
- Added a `#[cfg(test)] ToolRegistry::has_handler` helper so tests can assert executor registration directly.

## Verification

- `just fmt` from `codex-rs`: passed.
- `git diff --check -- codex-rs/core/src/tools/handlers/desktop_automation.rs codex-rs/core/src/tools/handlers/mod.rs codex-rs/core/src/tools/registry.rs codex-rs/core/src/tools/spec_plan.rs codex-rs/core/src/tools/spec_plan_tests.rs`: passed; only Git CRLF conversion warnings were emitted.
- `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-tools -Filter desktop_automation_tools_respect_config`: passed. Log: `logs/test-local-release-codex-tools-desktop_automation_tools_respect_config-20260520-193528.log`; result was 1 passed, 121 filtered out, release build finished in 27m 20s.
- `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Filter desktop_automation_tools_follow_desktop_automation_config -Lib -AllowBroadCoreLibUnitTests`: failed before running the DAB canary because this dirty tree does not compile `codex-core`. Log: `logs/test-local-release-codex-core-desktop_automation_tools_follow_desktop_automation_config-20260520-193346.log`.

## Remaining Blockers

`codex-core` release unit tests are blocked by unrelated compile failures in the current dirty tree. The first errors are unresolved imports such as `crate::hook_runtime::PendingInputHookDisposition`, `crate::hook_runtime::run_user_prompt_submit_hooks`, `codex_protocol::permissions::project_roots_glob_pattern`, `skills::collect_env_var_dependencies`, `skills::resolve_skill_dependencies_for_turn`, plus unrelated API mismatches such as missing `Session::input_queue`, `Op::UserInput.thread_settings`, and undeclared `LocalThreadStore`.

No Cargo/rustc process from the verification lane remains running.
