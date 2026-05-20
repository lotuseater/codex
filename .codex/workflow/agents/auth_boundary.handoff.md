# auth_boundary Handoff

Status: lane slice prepared.

## Paths changed

- `codex-rs/runtime-domain/auth-api/src/lib.rs`
  - Added domain/runtime `AuthMode` with variants `ApiKey`, `Chatgpt`,
    `ChatgptAuthTokens`, and `AgentIdentity`.
- `.codex/workflow/agents/auth_boundary.handoff.md`
  - Replaced the queued placeholder with this handoff.

No new `codex-rs/auth/**` crate files were created.

## Paths read / inspected

- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/runtime-domain/auth-api/Cargo.toml`
- `codex-rs/runtime-domain/auth-api/src/lib.rs`
- `codex-rs/app-server-protocol/src/protocol/common.rs`
- `codex-rs/core/src/client.rs`
- `codex-rs/core/src/client_tests.rs`
- `codex-rs/core/src/compact_remote.rs`
- `codex-rs/core/src/realtime_conversation.rs`
- Read-only targeted `rg` over `codex-rs/{core,app-server-protocol,app-server,app-server-transport,cli,login,otel,tui}` for `AuthMode` ownership and callers.
- Read-only targeted `rg` over relevant `Cargo.toml` files for `codex-auth-api` / `codex-app-server-protocol` manifest wiring.

## Current owner before root migration

- `codex-rs/app-server-protocol/src/protocol/common.rs:18-39` still defines the wire/schema `AuthMode`.
- That protocol enum carries wire-facing derives/attributes: `Serialize`, `Deserialize`, `Display`, `JsonSchema`, `TS`, `serde`, `ts`, and `strum` renames.
- The new `codex_auth_api::AuthMode` intentionally does not carry wire/schema dependencies.

## Exact remaining imports / callers

Core-facing imports that should migrate first:

- `codex-rs/core/src/client.rs:64`
  - `AuthRequestTelemetryContext::new` accepts `Option<AuthMode>` at `client.rs:1931`.
  - Telemetry mapping uses `AuthMode::ApiKey` / ChatGPT-like variants at `client.rs:1937-1940`.
- `codex-rs/core/src/client_tests.rs:17`
  - Uses `Some(AuthMode::Chatgpt)` at `client_tests.rs:591`.
- `codex-rs/core/src/compact_remote.rs:25`
  - Compares `auth_manager.auth_mode() == Some(AuthMode::ApiKey)` at `compact_remote.rs:211-212`.
- `codex-rs/core/src/realtime_conversation.rs:23`
  - Passes `Some(AuthMode::ApiKey)` to `provider.to_api_provider(...)` at `realtime_conversation.rs:624`.

Other direct `codex_app_server_protocol::AuthMode` references found:

- `codex-rs/app-server-transport/src/transport/remote_control/tests.rs:19`
- `codex-rs/app-server-transport/src/transport/remote_control/websocket.rs:1245`
- `codex-rs/app-server/src/message_processor.rs:50`
- `codex-rs/app-server/src/outgoing_message.rs:703`
- `codex-rs/app-server/src/request_processors.rs:36`
- `codex-rs/app-server/tests/common/auth_fixtures.rs:9`
- `codex-rs/app-server/tests/suite/auth.rs:8`
- `codex-rs/app-server/tests/suite/v2/account.rs:14`
- `codex-rs/app-server/tests/suite/v2/app_list.rs:30`
- `codex-rs/cli/src/login.rs:10`
- `codex-rs/cli/src/doctor.rs:1251, 1252, 1253, 1254, 1258, 1263, 1265, 1275, 1286, 1302, 1318, 2306, 2307, 2308, 2309, 2439, 2441, 2442, 2443, 3273, 3309`
- `codex-rs/login/src/auth/auth_tests.rs:5`
- `codex-rs/login/src/auth/external_bearer.rs:5`
- `codex-rs/login/src/auth/manager.rs:21, 22`
- `codex-rs/login/src/auth/revoke.rs:11`
- `codex-rs/login/src/auth/storage.rs:24`
- `codex-rs/login/src/server.rs:39, 1162`
- `codex-rs/login/tests/suite/auth_refresh.rs:6`
- `codex-rs/login/tests/suite/logout.rs:4`
- `codex-rs/otel/src/lib.rs:54, 55, 57, 58, 59, 60`
- `codex-rs/tui/src/app_server_session.rs:19`
- `codex-rs/tui/src/app/app_server_events.rs:13`
- `codex-rs/tui/src/lib.rs:31`
- `codex-rs/tui/src/local_chatgpt_auth.rs:5, 62`
- `codex-rs/tui/src/onboarding/auth.rs:13`

## Crate ownership recommendation

Use the existing `codex-rs/runtime-domain/auth-api` crate as the owner for the core-facing auth mode.

Rationale:

- It already exists as `codex-auth-api` and is auth-specific.
- It currently has no dependencies, which keeps the domain type usable from core/runtime crates without dragging app-server protocol, schema, TS, or transport concerns inward.
- `AuthMode` belongs next to `AuthProvider` / `AuthCredential` as a runtime auth contract.
- A neighboring `codex-rs/auth/**` crate would duplicate the current auth API crate's responsibility and would require extra root manifest work without reducing coupling.

Keep app-server-protocol as the wire DTO owner. If root wants ergonomic conversion, add `From` conversions in `codex-app-server-protocol` because that crate owns the wire enum and can depend outward on the domain type. Do not make `codex-auth-api` depend on `codex-app-server-protocol`.

## Root-owned manifest entries needed

This lane did not edit manifests.

- `codex-rs/Cargo.toml`
  - Add `runtime-domain/auth-api` as a workspace member if still absent.
  - Add `codex-auth-api = { path = "runtime-domain/auth-api" }` under `[workspace.dependencies]` if still absent.
- `codex-rs/core/Cargo.toml`
  - Add `codex-auth-api = { workspace = true }` when migrating core imports from `codex_app_server_protocol::AuthMode` to `codex_auth_api::AuthMode`.
- `codex-rs/login/Cargo.toml`
  - Likely add `codex-auth-api = { workspace = true }` if `AuthManager::auth_mode()` / stored auth boundary is migrated to return the domain type for core.
- `codex-rs/app-server-protocol/Cargo.toml`
  - Add `codex-auth-api = { workspace = true }` only if protocol implements conversion between the wire `AuthMode` and the domain `codex_auth_api::AuthMode`.
- Outer crates (`app-server`, `app-server-transport`, `cli`, `tui`, `otel`) may need manifest updates only if root migrates their local references directly instead of keeping them on the wire DTO type.

Targeted `rg` did not find existing `codex-auth-api` manifest wiring in the inspected root/crate manifests.

## Verification performed

- Read the required workflow/delegation contracts first.
- Inspected current `codex-auth-api` crate contents.
- Inspected current protocol `AuthMode` definition in `app-server-protocol/src/protocol/common.rs`.
- Ran targeted `rg` searches for `AuthMode` references and grouped remaining `codex_app_server_protocol::AuthMode` callers by file/line.
- Re-read the edited `codex-rs/runtime-domain/auth-api/src/lib.rs`.

No formatters, Cargo commands, Just tasks, broad builds, staging, commits, resets, or checkouts were run, per lane restrictions.

## Blockers / root follow-up

- Root must wire manifests before any crate depending on `codex-auth-api` can compile against this new type.
- Root or a granted lane must migrate core and login/API edge callsites; this worker was not allowed to edit `core`, `login`, `app-server-protocol`, manifests, or Bazel files.
- Compile/test verification is blocked until root-owned manifest wiring and caller migration are done.
- Commit is blocked by lane instructions forbidding Git operations.
