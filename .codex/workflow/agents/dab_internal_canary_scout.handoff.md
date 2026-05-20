# dab_internal_canary_scout Handoff

Status: completed read-only scout on 2026-05-20.

## Scope

Read-only inspection of the DAB availability worker's source changes to answer:

- whether the wiring uses internal Codex desktop automation rather than external Wizard DAB/MCP wiring
- the smallest canary root should run once unrelated compile blockers are cleared
- whether the slice is ready to commit

No source, manifest, lockfile, Bazel, generated, test, or snapshot files were edited. No Cargo, Just, formatter, staging, commit, broad build lane, or external Wizard DAB command was run.

## Files Read

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/dab_availability_worker.handoff.md`
- `codex-rs/core/src/tools/handlers/mod.rs`
- `codex-rs/core/src/tools/handlers/desktop_automation.rs`
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/spec_plan_tests.rs`
- `codex-rs/core/src/tools/registry.rs`
- `.codex/prototypes/check-core-boundaries.ps1`
- focused read-only diffs/searches for related Cargo/tool registry paths

## Findings

- `.codex/workflow/solid-refactor-handoff.md` identifies the intended fix as internal Codex DAB availability and explicitly warns not to rely on external `Wizard_Erasmus` DAB.
- `.codex/workflow/agents/dab_availability_worker.handoff.md` states the worker fixed the internal Codex desktop automation registration path and did not depend on external Wizard DAB/MCP wiring.
- `codex-rs/core/src/tools/handlers/mod.rs` now exposes `DesktopAutomationHandler`.
- `codex-rs/core/src/tools/spec_plan.rs` imports `DesktopAutomationHandler` and `codex_tools::create_desktop_automation_tools`, then registers `DesktopAutomationHandler::new(tool)` for each generated DAB tool when `config.desktop_automation_enabled` is true.
- `codex-rs/core/src/tools/handlers/desktop_automation.rs` calls `codex_desktop_automation::execute_tool` and `codex_desktop_automation::text_output_value`.
- `codex-rs/core/src/tools/spec_plan_tests.rs` adds the focused core canary `desktop_automation_tools_follow_desktop_automation_config`, using `codex_desktop_automation::DAB_FIND_WINDOW_TOOL` to assert `dab_find_window` is absent/handlerless when disabled and present/handled when enabled.
- `codex-rs/core/src/tools/registry.rs` adds a test-only `has_handler` helper used by the canary.
- `.codex/prototypes/check-core-boundaries.ps1` is a dependency/boundary scan script and does not change the DAB wiring conclusion.

## Internal-Only DAB Wiring

The DAB wiring appears internal-only.

Evidence:

- Core registration is driven by `config.desktop_automation_enabled` and `codex_tools::create_desktop_automation_tools`.
- Core execution is routed through the internal Rust crate `codex_desktop_automation`.
- Focused searches of the worker-touched core wiring found no source dependency on `Wizard`, `Wizard_Erasmus`, `mcp__wizard`, or external Wizard DAB tools.
- The only Wizard references observed were workflow notes warning workers not to use external Wizard DAB for this fix.

## Smallest Canary For Root

After the unrelated compile blockers are cleared, root should run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Filter desktop_automation_tools_follow_desktop_automation_config -Lib -AllowBroadCoreLibUnitTests
```

Rationale: this is the focused core canary for the new registration path. The worker handoff reports the separate `codex-tools` canary `desktop_automation_tools_respect_config` already passed; the remaining value is proving core exposes and handles the generated internal DAB tool.

## Blockers

The DAB canary is currently blocked by unrelated broader refactor compile errors reported in the worker handoff, including unresolved imports around hook runtime, permission globbing, skill dependency resolution, and unrelated API mismatches such as `Session::input_queue`, `Op::UserInput.thread_settings`, and `LocalThreadStore`.

No DAB-specific source blocker was found by this read-only scout.

## Commit Readiness

Not independently commit-ready yet. The DAB slice appears conceptually correct and internally wired, but it should wait for root to clear the unrelated compile blockers and run the focused core canary above green. After that, root should review the affected diff and commit the coherent DAB slice with any required manifest/lockfile/Bazel ownership handled at root level.
