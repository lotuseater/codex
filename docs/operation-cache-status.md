# Operation Cache Status

Date: 2026-05-01

## Current Result

The working tree now contains a Codex-side operation-cache interceptor plus a
thin Wizard bridge CLI. The system-wide wrapper is currently pointed at a
copied custom binary and has `WIZARD_CODEX_OPERATION_CACHE=1` enabled.

Evidence from `scripts/check-operation-cache.ps1`:

- The active wrapper points to `C:\Users\Oleh\.codex\local-builds\codex-custom-20260501-172101\codex.exe`.
- That copied binary matches `codex-rs\target\release\codex.exe`:
  `4478DFAE47430CCA92780903380F1C29BA27E6211E6CFE911AFD5BEB661B561B`.
- The release rebuild that produced it completed successfully in
  `logs\codex-cli-side-footer-release-build-20260501-170105.log`.
- The active wrapper stop-hook set is `caveat_hedge,premature_stop,not_green`.
- Codex's native MCP app tools cache exists at `~\.codex\cache\codex_apps_tools`.
- Codex's general operation cache DB is missing at `~\.codex\cache\tool_cache.sqlite`.
- The legacy Wizard/Claude operation cache DB exists at `~\.claude\cache\tool_cache.sqlite`.
- That shared DB has Codex-tagged rows and current-project rows for this repo.
- That shared DB has no `project_cache_state` row for this repo.
- This repo has no `.first_moves.db`.

## What Is Working

Codex has a native cache for app/MCP tool metadata:

- Code: `codex-rs/codex-mcp/src/codex_apps.rs`
- Runtime location: `~\.codex\cache\codex_apps_tools\*.json`
- Purpose: avoid repeatedly listing ChatGPT app connector tools.

This is not a general command/read/grep operation-result cache.

## What Is Implemented Here

Codex-side interceptor:

- Code: `codex-rs/core/src/tools/operation_cache.rs`
- Integration: `codex-rs/core/src/tools/registry.rs`
- Gate: `WIZARD_CODEX_OPERATION_CACHE=1`
- Bridge path: `WIZARD_CODEX_CACHE_BRIDGE_PY`
- Cache DB selection: delegated to Wizard's existing `WIZARD_TOOL_CACHE_DIR`
  resolver.

Wizard bridge CLI:

- Code: `Wizard_Erasmus/src/mcp/hooks/codex_cache_bridge_cli.py`
- Action `pre`: calls `codex_cache_bridge.pretool_lookup(event)` and returns a
  compact JSON hit/miss response to Rust.
- Action `post`: calls `codex_cache_bridge.posttool_store(event, output,
  success)` after a successful real tool call.
- The CLI wrapper intentionally returns misses for `mcp__...` tool names. The
  current Rust integration injects cached hits as function-tool output, which is
  correct for shell/read/grep style calls but not for MCP response items.

Wrapper helpers:

- `scripts/activate-copied-codex.ps1` copies a built `codex.exe`, points the
  system wrapper to that copy, and enables the operation cache env.
- `scripts/restore-standard-codex.ps1` points the wrapper back to the standard
  executable and disables the operation cache.
- `scripts/check-operation-cache.ps1` reports the active wrapper cache env and
  current cache DB state.
- `scripts/test-operation-cache.ps1` runs the focused Wizard bridge tests plus
  the Rust `codex-core --lib operation_cache` release lane.
- `scripts/test-operation-cache-runtime.ps1` runs the active `codex` wrapper
  twice against a repo-local canary file, asserts the second run increments the
  Codex cache hit count, then verifies a failed cacheable read actually runs,
  exits nonzero, and does not create a cache row.
- `scripts/test-session-limit-footer.ps1` checks the source plumbing, both
  accepted footer snapshots, the copied `codex.exe`, and the system wrapper
  entrypoint without compiling Rust.

## Verification Evidence

- `just fmt`
- Direct release TUI harness:
  `codex_tui-601ddfb907c446f8.exe session_limit_footer --nocapture`:
  5 passed.
- Direct release core harness:
  `codex_core-1dce93e568e9e8ce.exe operation_cache --nocapture`:
  2 passed.
- Direct release core harness:
  `codex_core-1dce93e568e9e8ce.exe exec_command_tool_output_success_for_logging_tracks_exit_code --nocapture`:
  1 passed.
- Wizard bridge pytest:
  `python -m pytest -q src/mcp/test_codex_cache_bridge_cli.py src/mcp/test_codex_cache_bridge.py -k "cli or codex_hits_claude_read_entry or claude_hits_codex_stored_entry or codex_shell_grep_canonicalization_is_conservative"`:
  6 passed.
- Active-wrapper footer smoke:
  `.\scripts\test-session-limit-footer.ps1` passed.
- Active-wrapper operation-cache canary:
  `.\scripts\test-operation-cache-runtime.ps1` passed and removed its canary
  cache and miss-telemetry rows.

## Existing Wizard Cache

Wizard has a cross-agent operation cache implementation:

- Cache DB resolver: `Wizard_Erasmus/src/mcp/tool_cache.py`
- Claude hooks:
  - `Wizard_Erasmus/src/mcp/hooks/pretool_cache_hook.py`
  - `Wizard_Erasmus/src/mcp/hooks/posttool_cache_hook.py`
- Codex bridge:
  - `Wizard_Erasmus/src/mcp/hooks/codex_cache_bridge.py`

The Codex bridge functions are now called by the Rust interceptor instead of
being left as an unused in-process API.

## Practical Meaning

- Repeated cacheable Codex shell operations can now be short-circuited from the
  shared Wizard cache when the custom binary and wrapper env are active.
- Wizard/Claude can still use its operation cache.
- Codex can still benefit from MCP app-tools metadata caching.
- Cache keys are still produced by Wizard's canonicalizer, so Codex and Claude
  collide intentionally for equivalent read/grep/glob operations while keeping
  non-equivalent shell commands under their own Bash-shaped keys.
- MCP cache rows remain available to Wizard's existing bridge-server path, but
  the Rust Codex wrapper path does not currently short-circuit MCP calls.

## Verification Command

Run:

```powershell
.\scripts\check-operation-cache.ps1
```

Expected fields to inspect:

- `Codex MCP tools cache: present`
- `Wrapper operation cache: 1` after activation
- `Wrapper cache bridge: ...\codex_cache_bridge_cli.py`
- `Cache DB: ...\.claude\cache\tool_cache.sqlite`
- `tool_cache current-project rows: ...`

For no-build runtime verification, run:

```powershell
.\scripts\test-operation-cache-runtime.ps1
```

The canary creates a temporary file under `logs\cache-canaries`, runs
`codex exec` twice through the system wrapper, checks
`~\.claude\cache\tool_cache.sqlite` for one stored Codex row plus an increased
hit count, verifies a failed read does not create a cache row, and removes the
temporary canary file afterwards. It also checks the run logs for the first
shell execution and for the failed read's nonzero exit. After assertions pass,
it removes the successful canary cache row, its dependency rows, and the exact
miss-telemetry rows for the generated canary paths so repeated runs do not
pollute the shared cache. The per-run Codex output logs are retained under
`logs\`.
