# SOLID Refactor Wave 20 Remaining MCP Tooling Test Split Worker Handoff

Classification: root-wiring-needed

## Scope

- Continued the remaining MCP/tooling test split after the code-mode, RMCP client,
  and search-tool workers.
- Did not touch code-mode, RMCP client, search-tool, schema, lockfile, build, or
  generated-output lanes.

## Split State

- `tools_mcp_openai_file` is wired to `tests/tools_mcp_openai_file.rs`, which
  loads `suite/openai_file_mcp.rs`.
- `tools_mcp_plugins` is wired to `tests/tools_mcp_plugins.rs`, which loads
  `suite/plugins.rs`.
- `tools_mcp_turn_metadata` is wired to `tests/tools_mcp_turn_metadata.rs`, which
  loads `suite/mcp_turn_metadata.rs`.
- `tests/tools.rs` now has no remaining MCP/local-file/turn-metadata split
  candidates; its remaining matches are non-MCP custom-tool and shell-tool cases.

## Changed Files In This Slice

- `codex-rs/core/Cargo.toml`
- `codex-rs/core/tests/tools_mcp_openai_file.rs`
- `codex-rs/core/tests/tools_mcp_plugins.rs`
- `codex-rs/core/tests/tools_mcp_turn_metadata.rs`
- `codex-rs/core/tests/suite/openai_file_mcp.rs`
- `codex-rs/core/tests/suite/plugins.rs`
- `codex-rs/core/tests/suite/mcp_turn_metadata.rs`
- `.codex/workflow/agents/solid_refactor_wave20_remaining_mcp_tooling_split_worker.handoff.md`

## Verification

- `rg -n "mcp_|mcp|local_file|turn_metadata|tool_call"` over
  `codex-rs/core/Cargo.toml`, existing `tests/mcp*.rs`, existing
  `tests/suite/mcp*.rs`, and the split `tools_mcp_*` wrappers/modules: passed,
  exit 0.
- `git diff --check -- codex-rs/core/Cargo.toml codex-rs/core/tests/mcp*.rs
  codex-rs/core/tests/suite/mcp*.rs
  .codex/workflow/agents/solid_refactor_wave20_remaining_mcp_tooling_split_worker.handoff.md`:
  passed, exit 0. Git emitted existing line-ending warnings for touched files.
- `git diff --check` over the split `tools_mcp_*`, `openai_file_mcp`, and
  `plugins` files: passed, exit 0. Git emitted existing line-ending warnings for
  touched files.

## Commit

- Not committed by this worker. The index already contains broad wave-integration
  changes from adjacent split work, so this slice should be committed by root or
  the integration steward rather than as a standalone worker commit.
