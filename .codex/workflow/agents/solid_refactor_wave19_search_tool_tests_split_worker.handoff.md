# SOLID Refactor Wave 19 Search Tool Test Split Worker Handoff

Classification: root-wiring-needed

## Changed Files

- `codex-rs/core/Cargo.toml`
- `codex-rs/core/tests/suite/search_tool.rs`
- `codex-rs/core/tests/search_tool_deferred.rs`
- `codex-rs/core/tests/search_tool_dynamic.rs`
- `codex-rs/core/tests/search_tool_matching.rs`
- `codex-rs/core/tests/search_tool_mcp.rs`
- `codex-rs/core/tests/suite/search_tool_deferred.rs`
- `codex-rs/core/tests/suite/search_tool_dynamic.rs`
- `codex-rs/core/tests/suite/search_tool_matching.rs`
- `codex-rs/core/tests/suite/search_tool_mcp.rs`

Observed but outside this worker's stated source ownership:

- `codex-rs/core/tests/tools_search.rs`

## Split Binaries Created

- `search_tool_deferred -> tests/search_tool_deferred.rs -> suite/search_tool_deferred.rs`
- `search_tool_dynamic -> tests/search_tool_dynamic.rs -> suite/search_tool_dynamic.rs`
- `search_tool_matching -> tests/search_tool_matching.rs -> suite/search_tool_matching.rs`
- `search_tool_mcp -> tests/search_tool_mcp.rs -> suite/search_tool_mcp.rs`

The remaining baseline search-tool tests are in `suite/search_tool.rs`. The current manifest routes those through `tools_search -> tests/tools_search.rs -> suite/search_tool.rs`, but `tests/tools_search.rs` is outside this worker's source ownership.

## Manifest Collision / Fallout

- `codex-rs/core/Cargo.toml` already has unrelated staged manifest/test-split edits from other workers.
- The index also already contains unrelated staged files under `config_code_mode*`.
- I did not overwrite, revert, or stage over those edits.
- Because the manifest is shared and already staged with unrelated work, root should perform final manifest wiring/commit staging.

## Commit

- Not committed.
- Reason: unrelated staged files and shared `Cargo.toml` edits make a safe worker-local commit boundary unavailable.

## Verification

- Source inspection found the original `suite/search_tool.rs` async test names preserved once across the split files: `old_count=19`, `new_count=19`, `missing=`, `extra=`, `duplicates=`.
- Allowed static verification only; no Cargo/Rust builds, formatters, schema generation, Bazel, lock refresh, release builds, deploy, or activation were run.

Commands run after handoff:

```powershell
$files = @('codex-rs/core/Cargo.toml') + (Get-ChildItem -Path codex-rs/core/tests -Filter 'search_tool*.rs').FullName + (Get-ChildItem -Path codex-rs/core/tests/suite -Filter 'search_tool*.rs').FullName; rg -n "search_tool" $files
git diff --check -- codex-rs/core/Cargo.toml codex-rs/core/tests/search_tool*.rs codex-rs/core/tests/suite/search_tool*.rs .codex/workflow/agents/solid_refactor_wave19_search_tool_tests_split_worker.handoff.md
```

Results:

- `rg`: passed; found the four split wrappers, four manifest entries, and remaining baseline `suite/search_tool.rs` tests.
- `git diff --check`: passed with exit code 0. Git emitted the existing line-ending warning for `codex-rs/core/Cargo.toml`.
