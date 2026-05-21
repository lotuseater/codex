# core_protocol_dependency_resume_worker Handoff

## Status

Source-only core cleanup complete. Read-only search found no remaining
`codex_app_server_protocol`, `app_server_protocol`, or `codex-app-server-protocol`
references under `codex-rs/core`.

## Files Changed

- `codex-rs/core/Cargo.toml`
- `codex-rs/core/src/mcp_tool_call.rs`
- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/thread_manager.rs`

## Notes

- MCP elicitation code now uses the protocol-neutral
  `codex-mcp-elicitation-api` crate instead of app-server protocol types.
- `snapshot_turn_state` no longer depends on app-server `ThreadHistoryBuilder`
  or app-server `TurnStatus`; it tracks the explicit active turn state locally
  from core rollout events.

## Verification

Source-only verification was skipped beyond read-only searches because this is a
no-build wave and this worker was explicitly blocked from running Cargo, Just,
build scripts, tests, schema generation, or check commands.

Read-only search run:

```powershell
rg -n "codex_app_server_protocol|app_server_protocol|codex-app-server-protocol" codex-rs\core codex-rs\core\Cargo.toml
```

Result: no matches.

## Later Verification For Root

```powershell
cd C:\Users\Oleh\Documents\GitHub\open_ai\codex\codex-rs
just fmt
just fix -p codex-core
cd C:\Users\Oleh\Documents\GitHub\open_ai\codex
just bazel-lock-update
just bazel-lock-check
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core
```

## Blockers / Next Owner

No app-server-owned blocker remains for the core-side references found in this
worker. Root should run the verification commands above after the no-build wave.
