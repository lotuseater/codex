# Agent Prompt: dab_availability_worker

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are responsible only for the internal Codex DAB/native desktop automation
availability bug. Do not rely on external Wizard_Erasmus DAB or MCP tools for
the fix; inspect and fix the internal Codex code path.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/scripts/Start-CodexWorker.ps1`
- `codex-rs/desktop-automation/src/lib.rs`
- `codex-rs/desktop-automation/src/windows.rs`
- `codex-rs/tools/src/desktop_automation.rs`
- `codex-rs/tools/src/tool_registry_plan.rs`
- `codex-rs/core/src/tools/handlers/desktop_automation.rs`
- `codex-rs/core/src/tools/handlers/mod.rs`
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/spec_plan_tests.rs`

Observed root finding:

- The internal DAB implementation and tool specs exist.
- `codex-rs/tools/src/tool_registry_plan.rs` registers DAB model specs when
  `desktop_automation_enabled` is true.
- `codex-rs/core/src/tools/handlers/desktop_automation.rs` defines
  `DesktopAutomationHandler`.
- The active core executor path appears not to export/register
  `DesktopAutomationHandler`, which can leave `dab_*` tools unavailable even
  when the prompt injects desktop automation instructions.

Owned paths:

- `codex-rs/core/src/tools/handlers/mod.rs`
- `codex-rs/core/src/tools/spec_plan.rs`
- `codex-rs/core/src/tools/spec_plan_tests.rs`
- `.codex/workflow/agents/dab_availability_worker.handoff.md`

Forbidden:

- no Git staging, commits, resets, or checkouts
- no `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`, Bazel files, or lockfiles
- no broad debug Cargo builds
- no unrelated SOLID refactor files
- no external Wizard_Erasmus DAB dependency

Task:

1. Verify whether core registers all DAB handlers corresponding to
   `create_desktop_automation_tools`.
2. If missing, wire `DesktopAutomationHandler` into the core executor registry
   behind `config.desktop_automation_enabled`.
3. Add or update a focused test proving both the model-visible DAB spec and the
   core handler exist for at least `dab_find_window`, and that the tools are
   absent when desktop automation is disabled.
4. Run the narrowest feasible verification. On this Windows checkout, prefer a
   targeted release test if running Rust tests.

Prototype-first requirement:

- Add or update the smallest targeted canary/test that proves enabled internal
  desktop automation specs have registered executors for `dab_*` tool names.
- Prefer a focused `spec_plan` or tool-registry test before changing runtime
  behavior.
- If the stale interactive worker recorded in
  `.codex/workflow/agents/dab_availability_worker.marker.txt` has not written
  a worker handoff, treat it as stale and continue from this prompt.

Write `.codex/workflow/agents/dab_availability_worker.handoff.md` with:

- changed/read paths
- exact cause
- verification command and result
- remaining blockers
