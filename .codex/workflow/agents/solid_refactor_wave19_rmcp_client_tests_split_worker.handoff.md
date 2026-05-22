# SOLID Refactor Wave 19 RMCP Client Tests Split Worker Handoff

Classification: root-wiring-needed

## Changed Files

- `.codex/workflow/agents/solid_refactor_wave19_rmcp_client_tests_split_worker.handoff.md`

## Pre-existing Owned Files Observed

- `codex-rs/core/tests/rmcp_client_connection.rs`
- `codex-rs/core/tests/rmcp_client_responses.rs`
- `codex-rs/core/tests/rmcp_client_streamable_http.rs`
- `codex-rs/core/tests/rmcp_client_tool_calls.rs`
- `codex-rs/core/tests/suite/rmcp_client_connection.rs`
- `codex-rs/core/tests/suite/rmcp_client_responses.rs`
- `codex-rs/core/tests/suite/rmcp_client_streamable_http.rs`
- `codex-rs/core/tests/suite/rmcp_client_support.rs`
- `codex-rs/core/tests/suite/rmcp_client_tool_calls.rs`

These files were already untracked when this worker started. I did not modify them.

## Split Binaries Created

Observed split adapters:

- `rmcp_client_connection`
- `rmcp_client_responses`
- `rmcp_client_streamable_http`
- `rmcp_client_tool_calls`

Each adapter includes `tests/support.rs`, `suite/rmcp_client_support.rs`, and the matching topic-focused suite module.

## Manifest Collision / Fallout

`codex-rs/core/Cargo.toml` was already modified when this worker started. The diff is not limited to the RMCP client split entries; it contains a broad set of unrelated `[[test]]` and dependency edits. Per the worker instruction, I stopped source/manifest editing and classified this as `root-wiring-needed` rather than overwriting or staging concurrent manifest work.

Observed RMCP manifest entries are present around the split adapters, but root needs to reconcile them with the concurrent manifest changes before commit.

## Commit

No commit was created. The owned source split and manifest state were pre-existing/unstaged, and the manifest collision makes a worker commit unsafe.

## Verification

```powershell
rg -n "rmcp_client_" codex-rs/core/Cargo.toml codex-rs/core/tests/rmcp_client_*.rs codex-rs/core/tests/suite/rmcp_client_*.rs
```

Result: failed under native PowerShell before source evaluation because `rg.exe` received the literal wildcard paths and Windows rejected them with `os error 123`. Git Bash could not be used as a fallback because `bash.exe` is currently routed through a broken WSL instance (`ERROR_PATH_NOT_FOUND` attaching the Ubuntu-rebuilt VHDX).

Equivalent source/static rerun with PowerShell-expanded file list:

```powershell
$files = @('codex-rs/core/Cargo.toml') + (Get-ChildItem -Path 'codex-rs/core/tests/rmcp_client_*.rs').FullName + (Get-ChildItem -Path 'codex-rs/core/tests/suite/rmcp_client_*.rs').FullName; rg -n "rmcp_client_" @files
```

Result: passed, exit 0. It found 28 matches across the manifest, four split adapter files, and four suite files. Manifest entries were present for `rmcp_client_connection`, `rmcp_client_responses`, `rmcp_client_streamable_http`, and `rmcp_client_tool_calls`.

```powershell
git diff --check -- codex-rs/core/Cargo.toml codex-rs/core/tests/rmcp_client_*.rs codex-rs/core/tests/suite/rmcp_client_*.rs .codex/workflow/agents/solid_refactor_wave19_rmcp_client_tests_split_worker.handoff.md
```

Result: passed, exit 0. Git emitted the existing line-ending warning for `codex-rs/core/Cargo.toml`: `LF will be replaced by CRLF the next time Git touches it`.
