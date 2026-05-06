# Codex Capability Internalization Research

Date: 2026-05-06

## Goal

Bring the useful capabilities currently supplied by WizardErasmus, DAB, skills,
hooks, and cache sidecars into Codex in a way that Codex can discover, report,
and use them without depending on fragile launch-session luck.

## Current PC Inventory

- Global Codex home: `C:\Users\Oleh\.codex`
- Global skills: `C:\Users\Oleh\.codex\skills`
- Repo skills: `C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\skills`
- Global hook config: `C:\Users\Oleh\.codex\hooks.json`
- Codex MCP/app tools cache: `C:\Users\Oleh\.codex\cache\codex_apps_tools`
- Repo first-moves DB: `.first_moves.db`
- Wizard repo: `C:\Users\Oleh\Documents\GitHub\Wizard_Erasmus`

The current global hook config runs Wizard's fail-open hook runner for
`PostToolUse` on edit/write operations and points at Wizard's Codex edit-safety
hook. The Wizard MCP config exposes first-moves, cache stats/hotspots, memory,
project-detect, and DAB tools such as `dab_find_window`, `dab_screenshot`,
`dab_ocr`, `dab_click`, and `dab_send_keys`.

## Native Codex Capabilities Already Present

- Skills: Codex already scans system and repo skills, exposes `skills/list`,
  and has watcher-driven cache invalidation.
- Hooks: Codex already resolves configured hooks and exposes `hooks/list`.
- MCP/app cache status: app-server protocol already has experimental
  `mcp/cache/status` for the native app-tools cache.
- Operation cache bridge: `codex-rs/core/src/tools/operation_cache.rs` can call
  Wizard's cache bridge. Explicit off values in `WIZARD_CODEX_OPERATION_CACHE`
  disable it; otherwise Codex auto-discovers the Wizard bridge when present.
- Tool discovery: first-moves and cache tools are already recognized as
  deferred MCP tools through the MCP exposure/discovery path. This change adds
  native `first_moves_predict` and `first_moves_stats` tools so Codex can still
  use the opening-sweep predictor when the Wizard MCP server is not exposed.

## Gap: Loop After Self-Review

The loop bug is caused by the automatic idle loop treating automatic
self-review completion as ordinary activity. After self-review succeeds, Codex
waits for the normal loop period instead of immediately submitting the loop
message. The correct behavior is:

- automatic self-review started by Codex marks a pending loop continuation;
- manual `/review` does not mark that continuation;
- the continuation is requested only after `ExitedReviewMode` and turn
  completion;
- the app layer submits the configured loop message only if loop mode is still
  enabled and the UI has no blocker.

## Gap: System Cache Scope

System-wide cache keys and paths must not collapse unrelated repos that happen
to use the same leaf folders, such as `src`. Codex now sends additive cache
scope metadata to the operation-cache bridge:

- `repo_root`
- `repo_name`
- `system_cache_namespace`

The namespace includes a path-safe repo folder component plus a stable hash of
the resolved repo root path. Existing bridge consumers can ignore the fields,
but future system-wide cache paths should use this namespace rather than a bare
cwd leaf.

The same repo-root rule now applies to native first-moves storage. If Codex is
launched from a subfolder such as `src`, first-moves resolves the enclosing git
root and uses the repo folder name plus root hash for the system-wide cache
namespace.

## Gap: Native Desktop Automation

WizardErasmus DAB is useful, but treating it only as an external MCP provider
means Codex can miss GUI evidence when that MCP server is not exposed in the
current session. The native Codex surface should make desktop automation a
built-in capability on Windows and reserve Wizard DAB for compatibility or
Wizard-specific state.

Current implementation direction:

- built-in DAB tool names match the established `dab_*` vocabulary;
- app/repo harness detection is a first-class tool so native harnesses can win
  before generic desktop control;
- screenshots, OCR-style UI Automation text extraction, visual scans, window
  checks, element maps, named navigation, clicks, background clicks, smart
  clicks, and send-keys are exposed as Codex tools;
- input-capable tools are treated as mutating, while window/screenshot/scan
  tools remain read-only and hook/cache visible;
- a bundled `visual-app-inspector` skill makes Codex proactively inspect GUI
  state when visual context can change the answer.

## Gap: Remembered Session Settings

The TUI can change model reasoning effort, approval policy, and permission
profile during a running session, but restored/app-server sessions previously
could fall back to stale persisted config. Codex now:

- persists approval policy changes to the active profile or root config;
- persists legacy-compatible permission profile changes as `sandbox_mode` and
  clears root `default_permissions` when saving root full-access choices;
- forwards the current in-memory reasoning effort and summary into thread
  start/resume/fork config overrides so restored sessions keep the user's
  selected effort immediately.

## Gap: Plan Self-Review

Automatic self-review for implementation work should not run during planning.
Plan mode now uses a separate one-shot plan-review path:

- the trigger is the turn-complete event after a whole plan has been emitted and
  no question/modal/queued steer is still active;
- Codex submits a plan-review prompt asking for a revised practical plan;
- the revised plan is not reviewed again;
- regular code self-review remains suppressed during and immediately after plan
  review so it cannot recurse or compete with the planning flow.

Known GUI harness patterns on this PC include `test_gui_automation.py`,
`gui_automation.cpp/.hpp`, `UiAutomationHarness.psm1`,
`test_ui_automation.ps1`, `visual_autotest.py`, `android_visual_e2e.py`,
Playwright configs, and WinAppDriver assets under GitHub repos.

## Implementation Priority

1. Fix auto-loop continuation after automatic self-review.
2. Make operation-cache events carry repo identity for system-wide cache
   scoping.
3. Add native Windows DAB tools to Codex itself, gated by explicit desktop
   automation config and the existing `computer_use` feature.
4. Add native first-moves prediction and hit telemetry for fresh repo openings.
5. Keep skills/hooks discovery native and reportable through app-server/TUI
   status instead of relying on conversational memory.
6. Prefer app-native GUI harnesses before native DAB, and use Wizard DAB only
   as a compatibility fallback.
