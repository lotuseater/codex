# core_tests_client_lane_worker Handoff

Date: 2026-05-20
Status: complete; narrow release verification blocked by unrelated `codex-core`
library compilation errors

## Scope

Owned edit paths:

- `codex-rs/core/tests/client.rs`
- `codex-rs/core/tests/responses_headers.rs`
- `codex-rs/core/tests/suite/cli_stream.rs`
- `codex-rs/core/tests/suite/client.rs`
- `codex-rs/core/tests/suite/client_websockets.rs`
- `codex-rs/core/tests/suite/live_cli.rs`
- `codex-rs/core/tests/suite/realtime_conversation.rs`
- `codex-rs/core/tests/suite/responses_api_proxy_headers.rs`
- `codex-rs/core/tests/suite/rmcp_client.rs`
- `codex-rs/core/tests/suite/websocket_fallback.rs`

## Changes

- Confirmed `codex-rs/core/tests/client.rs` is the top-level client lane
  binary and declares the owned suite modules via direct `#[path = "..."]`
  module declarations.
- Left `codex-rs/core/tests/responses_headers.rs` as a separate top-level
  binary. It is already an independent tracked integration test file with its
  own response-header setup and no local low-risk fold into the new client lane
  was needed for this slice.
- Did not recreate `suite/mod.rs`.

## Verification

- `just fmt` from `codex-rs` completed successfully.
- `git diff --check -- codex-rs/core/tests/client.rs .codex/workflow/agents/core_tests_client_lane_worker.handoff.md` completed successfully.
- Attempted narrow release integration-target check:
  `& .\scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs @('--test','client','--','--list')`.
  Cargo failed while compiling `codex-core` library before reaching the
  `client` test target. The log is
  `logs/test-local-release-codex-core-all-20260521-001610.log`; representative
  unrelated errors include unresolved imports in `core/src` and API mismatches
  such as `Op::UserInput` lacking `thread_settings`,
  `SessionTaskContext::turn_extension_data()` missing, and
  `LocalThreadStore` undeclared.

## Git Notes

- The index already contained unrelated staged test-file renames before this
  slice was staged. This worker must commit only the explicit client-lane paths
  and leave unrelated staged entries untouched.
