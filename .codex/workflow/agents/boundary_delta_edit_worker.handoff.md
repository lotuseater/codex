# boundary_delta_edit_worker Handoff

Status: completed one source-only boundary cleanup slice on 2026-05-20.

## Owned Violation Family

- Family: core `AuthMode` imports from `codex_app_server_protocol`.
- Reason for choosing it: it is part of the app-server protocol DTO boundary lane, but does not overlap the active config provenance worker and does not touch the carved-out MCP elicitation, `ThreadHistoryBuilder`, or `TurnStatus` items.
- Expected canary impact if no other workers changed the same area: `codex_app_server_protocol::` source-pattern violations in core should drop by 4, from 8 to 4. The transitive `codex-app-server-protocol` dependency violation is still expected to remain until root/owner wiring removes the remaining transitive dependency path.

## Files Edited

- `codex-rs/core/src/client.rs`
- `codex-rs/core/src/client_tests.rs`
- `codex-rs/core/src/compact_remote.rs`
- `codex-rs/core/src/realtime_conversation.rs`

## Change Summary

- Removed direct `codex_app_server_protocol::AuthMode` imports from the four owned core files.
- Changed request telemetry classification to derive from `CodexAuth` instead of the app-server protocol DTO.
- Kept remote compaction's API-key service-tier behavior by checking `CodexAuth::ApiKey` through the auth manager.
- Kept realtime API-provider behavior by using the `to_api_provider(/*auth_mode*/ None)` default path, which is the same OpenAI API base URL used for API-key auth.

## Verification Performed

- Formatted only the owned Rust files with:
  `rustfmt codex-rs\core\src\client.rs codex-rs\core\src\client_tests.rs codex-rs\core\src\compact_remote.rs codex-rs\core\src\realtime_conversation.rs`
- Narrow owned-slice static scan returned no matches:
  `rg -n "codex_app_server_protocol::" codex-rs\core\src\client.rs codex-rs\core\src\client_tests.rs codex-rs\core\src\compact_remote.rs codex-rs\core\src\realtime_conversation.rs`
- Narrow owned-slice static scan returned no matches:
  `rg -n "AuthMode|auth_mode\(" codex-rs\core\src\client.rs codex-rs\core\src\client_tests.rs codex-rs\core\src\compact_remote.rs codex-rs\core\src\realtime_conversation.rs`

No Cargo, Just, Bazel, build script, or test script was run.

## Root Static Scan To Confirm Count Delta

Run this later from repo root:

`powershell -ExecutionPolicy Bypass -File .codex\prototypes\check-core-boundaries.ps1`

The expected confirmation for this slice is that the `codex_app_server_protocol::` source-pattern group no longer includes:

- `codex-rs/core/src/client.rs`
- `codex-rs/core/src/client_tests.rs`
- `codex-rs/core/src/compact_remote.rs`
- `codex-rs/core/src/realtime_conversation.rs`

