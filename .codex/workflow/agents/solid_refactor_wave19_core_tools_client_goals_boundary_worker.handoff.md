# SOLID Refactor Wave 19 Core Tools Client/Goals Boundary Worker Handoff

Classification: repair-needed

## Changed Files

- `codex-rs/core/src/client.rs`
- `codex-rs/core/src/client_common.rs`
- `codex-rs/core/src/goals.rs`
- `codex-rs/core/src/tools/handlers/goal_spec.rs`
- `codex-rs/tools-domain/tool-execution-api/src/lib.rs`
- `codex-rs/tools-domain/tool-registry-api/src/lib.rs`
- `codex-rs/tools/src/lib.rs`
- `codex-rs/tools/src/tool_output.rs`
- `codex-rs/tools/src/tool_spec.rs`

## Boundary Result

- Direct `codex_tools` references remain in the originally owned core client/goals files: no.
- `client.rs` serializes model tools through `codex_tool_registry_api::create_tools_json_for_responses_api`.
- `client_common.rs` uses `codex_tool_registry_api::ToolSpec`.
- `goals.rs` no longer imports the concrete goal tool-spec module for the update-goal tool name; it uses `codex_tool_registry_api::UPDATE_GOAL_TOOL_NAME`.
- Goal tool names live at the registry API boundary and `goal_spec.rs` re-exports them for existing handler call sites.
- `codex-tools` now re-exports registry API tool schema/types directly and keeps `tool_spec.rs` as construction/adaptation helpers.
- The MCP `CallToolResult` output adapter moved to `codex_tool_execution_api`, matching the new `ToolOutput` ownership boundary.

## Verification

Passed:

```powershell
rustfmt --edition 2024 codex-rs\core\src\goals.rs codex-rs\core\src\tools\handlers\goal_spec.rs codex-rs\tools-domain\tool-registry-api\src\lib.rs codex-rs\tools-domain\tool-execution-api\src\lib.rs codex-rs\tools\src\lib.rs codex-rs\tools\src\tool_output.rs codex-rs\tools\src\tool_spec.rs
```

```powershell
if (Select-String -Path codex-rs\core\src\client.rs,codex-rs\core\src\client_common.rs,codex-rs\core\src\goals.rs -Pattern 'codex_tools::|\bcodex_tools\b' -Quiet) { Write-Error 'owned core files still reference codex_tools'; exit 1 } else { 'ok: owned core files do not reference codex_tools' }
```

```powershell
& scripts\check-cargo-dependency-boundaries.ps1 -Package codex-tool-registry-api -ForbiddenPackages @('codex-core','codex-tools') -ForbiddenSourcePatterns @('codex_core::','codex_tools::') -Json
```

```powershell
& scripts\check-cargo-dependency-boundaries.ps1 -Package codex-tool-execution-api -ForbiddenPackages @('codex-core','codex-tools') -ForbiddenSourcePatterns @('codex_core::','codex_tools::') -Json
```

```powershell
cargo check --release -p codex-tool-registry-api -p codex-tool-execution-api -p codex-tools
```

Result: passed. The cargo check emitted pre-existing warnings from `codex-execpolicy` and `codex-protocol`.

Failed outside this worker slice:

```powershell
cargo check --release -p codex-core --lib
```

Result: failed after source-static worker checks passed. Log: `logs/wave19-codex-core-lib-check.log`.

Observed first failures include unresolved imports for `codex_file_system` and `codex_session_state`, undeclared `ToolsConfig`, private `PreviousTurnSettings`, and incompatible `ToolOutput` method signatures in multi-agent handler modules. No diagnostics in the log pointed at `client.rs`, `client_common.rs`, `goals.rs`, or `goal_spec.rs`.

Also blocked:

```powershell
just fmt
```

Result: failed on Windows with `The filename or extension is too long. (os error 206)`.

```powershell
cargo fmt -p codex-core -p codex-tool-registry-api -p codex-tool-execution-api -p codex-tools -- --check
```

Result: failed while traversing unrelated `core/tests/suite/items_message_events.rs`, which rustfmt could not parse. Direct rustfmt on the touched files passed.

## Commit

Not committed. The repository has broad unrelated dirty work, and `codex-core --lib` is not green because of cross-worker fallout outside this handoff's owned files.

## Remaining Fallout

- Root/wiring workers need to repair the broader `codex-core --lib` failures before this slice can be classified accepted.
- `codex-rs/core/Cargo.toml` still has a direct `codex-tools` dependency because other core modules/tests outside this worker's ownership still reference `codex_tools`.
