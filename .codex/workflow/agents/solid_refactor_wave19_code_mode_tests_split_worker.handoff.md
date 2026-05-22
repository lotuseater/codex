# SOLID Refactor Wave 19 Code-Mode Tests Split Worker Handoff

Classification: accepted

## Changed Files

- `codex-rs/core/tests/config_code_mode_apps.rs`
- `codex-rs/core/tests/config_code_mode_async.rs`
- `codex-rs/core/tests/config_code_mode_config.rs`
- `codex-rs/core/tests/config_code_mode_execution.rs`
- `codex-rs/core/tests/config_code_mode_mcp.rs`
- `codex-rs/core/tests/config_code_mode_media.rs`
- `codex-rs/core/tests/suite/code_mode_apps.rs`
- `codex-rs/core/tests/suite/code_mode_async.rs`
- `codex-rs/core/tests/suite/code_mode_config.rs`
- `codex-rs/core/tests/suite/code_mode_execution.rs`
- `codex-rs/core/tests/suite/code_mode_mcp.rs`
- `codex-rs/core/tests/suite/code_mode_media.rs`
- `codex-rs/core/tests/suite/code_mode_shared.rs`

## Split Binaries Created

- `config_code_mode_apps`
- `config_code_mode_async`
- `config_code_mode_config`
- `config_code_mode_execution`
- `config_code_mode_mcp`
- `config_code_mode_media`

`codex-rs/core/Cargo.toml` already had the six `config_code_mode_*` `[[test]]` entries in `HEAD`; this worker did not commit a manifest change.

## Manifest Collision / Fallout

- No code-mode manifest collision was found.
- `codex-rs/core/Cargo.toml` still has unrelated unstaged edits from other slices; they were not staged or committed by this worker.
- Nearby unowned files `codex-rs/core/tests/config.rs` and `codex-rs/core/tests/suite/code_mode.rs` still have unrelated unstaged edits; they were not staged or committed by this worker.

## Commit

- `fecd3ce63c` - `Split code mode tests by topic`

## Verification

- `rg -n "config_code_mode_|code_mode_" codex-rs/core/Cargo.toml codex-rs/core/tests/config_code_mode_*.rs codex-rs/core/tests/suite/code_mode_*.rs`
  - PowerShell passes the wildcard paths through to `rg`, so the literal prompt form fails with `os error 123`.
  - Ran the PowerShell-expanded equivalent:
    `$paths = @('codex-rs/core/Cargo.toml') + (Resolve-Path 'codex-rs/core/tests/config_code_mode_*.rs').Path + (Resolve-Path 'codex-rs/core/tests/suite/code_mode_*.rs').Path; rg -n "config_code_mode_|code_mode_" @paths`
  - Result: passed, exit 0; found the six Cargo entries plus the split root/suite references.
- `git diff --cached --check -- codex-rs/core/tests/config_code_mode_apps.rs codex-rs/core/tests/config_code_mode_async.rs codex-rs/core/tests/config_code_mode_config.rs codex-rs/core/tests/config_code_mode_execution.rs codex-rs/core/tests/config_code_mode_mcp.rs codex-rs/core/tests/config_code_mode_media.rs codex-rs/core/tests/suite/code_mode_apps.rs codex-rs/core/tests/suite/code_mode_async.rs codex-rs/core/tests/suite/code_mode_config.rs codex-rs/core/tests/suite/code_mode_execution.rs codex-rs/core/tests/suite/code_mode_mcp.rs codex-rs/core/tests/suite/code_mode_media.rs codex-rs/core/tests/suite/code_mode_shared.rs`
  - Result: passed, exit 0 before commit.
- `git diff --check -- codex-rs/core/Cargo.toml codex-rs/core/tests/config_code_mode_*.rs codex-rs/core/tests/suite/code_mode_*.rs .codex/workflow/agents/solid_refactor_wave19_code_mode_tests_split_worker.handoff.md`
  - Result: passed, exit 0 after handoff was written; emitted only the existing CRLF warning for unstaged `codex-rs/core/Cargo.toml`.
