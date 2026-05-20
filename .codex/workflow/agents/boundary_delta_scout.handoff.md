# boundary_delta_scout Handoff

Status: completed read-only delta scan on 2026-05-20.

## Scope

- Read `.codex/workflow/solid-refactor-handoff.md`.
- Read `.codex/workflow/agents/canary_observer.handoff.md`.
- Read `.codex/prototypes/check-core-boundaries.ps1`.
- Ran only the allowed boundary canary:
  `powershell -ExecutionPolicy Bypass -File .codex\prototypes\check-core-boundaries.ps1`
- Did not edit source files, run Cargo/Just/formatters, or stage/commit.

## Current Canary Result

- Exit code: `1`
- Total violations: `23`
- Source-pattern violations: `22`
- Transitive dependency violations: `1`
- Direct forbidden crate dependency violations emitted by the canary: `0`

Pattern totals:

| Pattern / dependency | Count |
| --- | ---: |
| `codex_app_server_protocol::` | 8 |
| `LocalThreadStore` | 5 |
| `LocalThreadStoreConfig` | 4 |
| `thread_store_from_config` | 3 |
| `InMemoryThreadStore` | 2 |
| `codex-core` transitively depends on `codex-app-server-protocol` | 1 |

## Grouped Violations

### App-server protocol DTO boundary lane

Likely root-owned fix: move core-facing DTO/provenance/status types behind domain or protocol-owner crates, then remove the `codex-core` dependency path to `codex-app-server-protocol`. This is root-owned because it touches manifest wiring and the core/protocol ownership boundary.

Current violations:

- `codex-rs/core/src/client.rs`: `codex_app_server_protocol::`
- `codex-rs/core/src/client_tests.rs`: `codex_app_server_protocol::`
- `codex-rs/core/src/compact_remote.rs`: `codex_app_server_protocol::`
- `codex-rs/core/src/mcp_tool_call.rs`: `codex_app_server_protocol::`
- `codex-rs/core/src/realtime_conversation.rs`: `codex_app_server_protocol::`
- `codex-rs/core/src/session/mod.rs`: `codex_app_server_protocol::`
- `codex-rs/core/src/session/tests.rs`: `codex_app_server_protocol::`
- `codex-rs/core/src/thread_manager.rs`: `codex_app_server_protocol::`
- `codex-core` transitively depends on forbidden crate `codex-app-server-protocol`.

### Thread-store concrete implementation boundary lane

Likely root-owned fix: route `codex-core` through thread-store ports/factories or existing/new thread API crates, with concrete `codex-thread-store` construction kept in an outer adapter. This probably needs manifest ownership by root because the intended fix is a crate boundary change, not local import hiding.

Current production violations:

- `codex-rs/core/src/session/mod.rs`: `LocalThreadStore`
- `codex-rs/core/src/thread_manager.rs`: `LocalThreadStore`, `LocalThreadStoreConfig`
- `codex-rs/core/src/prompt_debug.rs`: `thread_store_from_config`

Current test/helper violations:

- `codex-rs/core/src/agent/control_tests.rs`: `LocalThreadStore`, `LocalThreadStoreConfig`
- `codex-rs/core/src/session/tests.rs`: `InMemoryThreadStore`, `LocalThreadStore`, `LocalThreadStoreConfig`
- `codex-rs/core/src/session/tests/guardian_tests.rs`: `LocalThreadStore`, `LocalThreadStoreConfig`
- `codex-rs/core/src/thread_manager_tests.rs`: `InMemoryThreadStore`, `thread_store_from_config`
- `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`: `thread_store_from_config`

### Test fixture cleanup lane

Likely root-owned fix: after production boundaries are split, provide test-only abstract stores/builders in the new owner crate or move concrete fixture construction outside the `codex-core/src` scan surface. Avoid compatibility re-export shims; tests should validate the new port surface directly.

Current violations are the test/helper subset listed above plus `client_tests.rs` and `session/tests.rs` for app-server protocol DTO references.

## Delta Versus `canary_observer`

No discernible delta.

`canary_observer` reported the same total shape: `23` violations, made up of `22` source-pattern leaks and `1` transitive dependency violation. The current scan has the same pattern counts:

- `codex_app_server_protocol::`: `8`
- `LocalThreadStore`: `5`
- `LocalThreadStoreConfig`: `4`
- `thread_store_from_config`: `3`
- `InMemoryThreadStore`: `2`
- transitive `codex-app-server-protocol`: `1`

## Highest-impact Violation Group

The app-server protocol DTO boundary lane is the highest-impact group.

Reason: it accounts for 8 source-pattern violations plus the only transitive dependency violation. Clearing this group should reduce the canary by up to 9 lines and removes a core-to-outer-protocol ownership leak that blocks the larger clean-architecture direction.

## Exact Next Implementation Slice Recommendation

Start with a root-owned app-server protocol DTO boundary slice in `codex-core`, excluding MCP elicitation types, `ThreadHistoryBuilder`, and `TurnStatus` per the current handoff guidance.

Recommended slice:

1. Replace direct `codex_app_server_protocol::` imports in production files first:
   - `codex-rs/core/src/client.rs`
   - `codex-rs/core/src/compact_remote.rs`
   - `codex-rs/core/src/mcp_tool_call.rs`
   - `codex-rs/core/src/realtime_conversation.rs`
   - `codex-rs/core/src/session/mod.rs`
   - `codex-rs/core/src/thread_manager.rs`
2. Use an existing/prepared domain owner where available, for example `codex_config_types::ConfigLayerSource` for config provenance instead of app-server protocol DTOs.
3. Update the corresponding tests after the production imports are gone:
   - `codex-rs/core/src/client_tests.rs`
   - `codex-rs/core/src/session/tests.rs`
4. Rerun only `.codex\prototypes\check-core-boundaries.ps1` to verify the expected reduction before starting the thread-store lane.
