# manifest_wiring_scout Handoff

Status: read-only manifest scout complete.

## Scope

This pass inspected the prepared boundary crates and root workspace manifest only.
It did not edit manifests or source files, did not run Cargo/Just/formatters, and
did not stage or commit anything. The only file written is this handoff.

## Sources inspected

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/auth_boundary.handoff.md`
- `.codex/workflow/agents/thread_projection_boundary.handoff.md`
- `.codex/workflow/agents/mcp_elicitation_boundary.handoff.md`
- `codex-rs/Cargo.toml`
- `codex-rs/runtime-domain/auth-api/Cargo.toml`
- `codex-rs/runtime-domain/auth-api/src/lib.rs`
- `codex-rs/thread/thread-projection-api/Cargo.toml`
- `codex-rs/thread/thread-projection-api/src/lib.rs`
- `codex-rs/thread/thread-projection-api/src/page.rs`
- `codex-rs/thread/thread-projection-api/src/turn.rs`
- `codex-rs/mcp/elicitation-api/Cargo.toml`
- `codex-rs/mcp/elicitation-api/src/lib.rs`

## Prepared crate names

| Boundary | Path | Package name | Lib crate name | Current direct dependencies |
| --- | --- | --- | --- | --- |
| Auth API | `codex-rs/runtime-domain/auth-api` | `codex-auth-api` | `codex_auth_api` | none |
| Thread projection API | `codex-rs/thread/thread-projection-api` | `codex-thread-projection-api` | `codex_thread_projection_api` | `codex-protocol`, `serde` with `derive` |
| MCP elicitation API | `codex-rs/mcp/elicitation-api` | `codex-mcp-elicitation-api` | `codex_mcp_elicitation_api` | `serde` with `derive`, `serde_json` |

## Exact root manifest edits

Root should edit `codex-rs/Cargo.toml`.

Auth is already wired at the root workspace level. No additional root member or
workspace dependency entry is needed for this prepared crate:

```toml
"runtime-domain/auth-api"
codex-auth-api = { path = "runtime-domain/auth-api" }
```

Add the MCP elicitation crate to `[workspace].members`, near the existing MCP
entries:

```toml
    "mcp/elicitation-api",
```

Add its workspace dependency near `codex-mcp` / `codex-mcp-server`:

```toml
codex-mcp-elicitation-api = { path = "mcp/elicitation-api" }
```

Add the thread projection crate to `[workspace].members`, near the other
`thread/thread-*` entries:

```toml
    "thread/thread-projection-api",
```

Add its workspace dependency near the other thread dependencies:

```toml
codex-thread-projection-api = { path = "thread/thread-projection-api" }
```

## Downstream dependency entries for integration slices

Only add these when the corresponding code imports are changed in the same
integration slice:

```toml
codex-auth-api = { workspace = true }
codex-thread-projection-api = { workspace = true }
codex-mcp-elicitation-api = { workspace = true }
```

Likely downstream targets:

- `codex-rs/core/Cargo.toml`: add `codex-auth-api` when core migrates runtime
  auth imports; add `codex-mcp-elicitation-api` when core moves MCP elicitation
  request/schema imports.
- `codex-rs/login/Cargo.toml`: add `codex-auth-api` if `AuthManager` or related
  login APIs return or accept the runtime-domain auth type.
- `codex-rs/app-server-protocol/Cargo.toml`: add boundary crate dependencies only
  for explicit wire/domain conversion code. Protocol may depend on domain API
  crates; the new API crates should not depend on app-server protocol.
- `codex-rs/app-server/Cargo.toml`: likely add `codex-thread-projection-api`
  when thread projection conversion or adapter code moves out of protocol-local
  DTO assumptions.

## Dependency order risks

- Add root workspace member and workspace dependency entries before adding
  downstream `{ workspace = true }` dependencies, otherwise downstream manifests
  will reference missing workspace dependency keys.
- Keep dependency direction outward from neutral API crates. Do not make
  `codex-auth-api`, `codex-thread-projection-api`, or
  `codex-mcp-elicitation-api` depend on `codex-core`,
  `codex-app-server-protocol`, `codex-app-server`, `rmcp`, `schemars`, or
  `ts-rs`.
- Auth has two concepts during migration: runtime/domain `AuthMode` in
  `codex-auth-api`, and wire/schema `AuthMode` in `codex-app-server-protocol`.
  Wire/domain conversions should live at protocol or adapter boundaries, not in
  `codex-auth-api`.
- Thread projection currently keeps item payloads generic and depends only on
  `codex-protocol` plus `serde`. Full `ThreadHistoryBuilder` migration is not
  complete because app-server `ThreadItem`, item builders, and error conversion
  still have separate ownership.
- MCP elicitation request/schema types are ready to wire as neutral API types,
  but response/action ownership is separate. Core currently uses RMCP/client
  response/action types, so response DTO migration should be a later slice.

## Do not wire yet

- Do not create a new `codex-rs/auth/**` crate; `runtime-domain/auth-api` is the
  prepared auth boundary.
- Do not move or declare complete ownership of app-server `ThreadItem` in this
  slice.
- Do not wire a full `ThreadHistoryBuilder` migration as complete until its item
  DTOs, builder helpers, and `CodexErrorInfo` conversion path are handled.
- Do not wire MCP elicitation response/action DTO migration in this slice.
- Do not add back-dependencies from the new neutral API crates into protocol,
  app-server, or core crates.
- Do not refresh Bazel locks, schema fixtures, or broad verification from this
  scout pass; root should do that in the actual manifest/protocol integration
  slice.
