# SOLID Refactor Wave 19 Shell/Unified-Exec Boundary Worker Handoff

Classification: accepted

## Changed Files

- `codex-rs/tools-domain/tool-execution-api/src/lib.rs`
- `codex-rs/tools/src/tool_config.rs`
- `codex-rs/core/src/tools/tool_family/shell.rs`
- `codex-rs/core/src/tools/handlers/unified_exec.rs`
- `codex-rs/core/src/tools/handlers/unified_exec_tests.rs`
- `codex-rs/core/src/tools/handlers/shell/shell_command.rs`
- `codex-rs/core/src/tools/handlers/shell_tests.rs`
- `codex-rs/tools/src/tool_config_tests.rs`
- `codex-rs/core/src/tools/spec_tests.rs`

## Boundary Moved

- Moved `ShellCommandBackendConfig`, `UnifiedExecShellMode`, and `ZshForkConfig` definitions from the `codex-tools` implementation path into `codex-tool-execution-api`.
- Kept `codex-tools` compatibility re-exports in `tool_config.rs` so existing public `codex_tools::*` imports continue to resolve while core shell/unified-exec code can depend on the narrow boundary.
- Left zsh-fork session resolution and absolute-path validation in `codex-tools`; the boundary type now carries the validated paths as `PathBuf`.
- Updated owned shell/unified-exec core imports to use `codex_tool_execution_api` for the moved backend/mode types.
- Updated existing direct `ZshForkConfig` test constructors to pass `PathBuf` after the same absolute-path validation. These two fallout edits are outside the initial source ownership list but are direct compile fallout from changing the boundary type payload.

## Commit

Not committed.

Reason: the worktree already has unrelated dirty work in touched files from other workers, especially `codex-rs/tools-domain/tool-execution-api/src/lib.rs`, `codex-rs/core/src/tools/handlers/shell/shell_command.rs`, `codex-rs/core/src/tools/handlers/shell_tests.rs`, and `codex-rs/core/src/tools/spec_tests.rs`. A normal file-level commit would include unrelated hunks.

## Verification

```powershell
rg -n "codex_tools|ShellCommandBackendConfig|UnifiedExecShellMode|ZshForkConfig|ToolName" codex-rs/core/src/unified_exec codex-rs/core/src/tools/handlers codex-rs/core/src/tools/tool_family/shell.rs codex-rs/tools-domain/tool-execution-api/src/lib.rs codex-rs/tools/src/tool_config.rs
```

Result: exit 0. The moved shell/unified-exec types now resolve from `codex-tool-execution-api` / `tool_config.rs`. The remaining `codex_tools` matches are unrelated tool-spec/schema imports in other handler families plus existing `ToolName` matches.

Supplemental focused check:

```powershell
rg -n "codex_tools::(ShellCommandBackendConfig|UnifiedExecShellMode|ZshForkConfig)|use codex_tools::(ShellCommandBackendConfig|UnifiedExecShellMode|ZshForkConfig)" codex-rs/core/src/unified_exec codex-rs/core/src/tools/handlers codex-rs/core/src/tools/tool_family/shell.rs
```

Result: exit 1, no matches.

```powershell
scripts\check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json
```

Result: exit 0. JSON reported `"violation_count": 0`.

```powershell
git diff --check -- codex-rs/core/src/unified_exec codex-rs/core/src/tools/handlers codex-rs/core/src/tools/tool_family/shell.rs codex-rs/tools-domain/tool-execution-api/src/lib.rs codex-rs/tools/src/tool_config.rs .codex/workflow/agents/solid_refactor_wave19_shell_unified_exec_boundary_worker.handoff.md
```

Result: exit 0 with existing LF/CRLF working-copy warnings only.

No Cargo/Rust builds or tests, formatters, schema generation, Bazel, lock refresh, release builds, deploy, or activation were run.

## Remaining Fallout

- No manifest or root wiring was required.
- The broad allowed `rg` still reports unrelated `codex_tools` imports in non-shell handler specs; those are outside this worker's shell/unified-exec boundary.
- Commit remains for root or a clean staging pass because touched files contain unrelated dirty hunks.
