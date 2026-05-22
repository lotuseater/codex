# SOLID Refactor Wave 19 Agents Runtime Split Handoff

Classification: accepted

## Scope

- Added standalone integration-test wrapper binaries for the already-split agents runtime test groups:
  - `codex-rs/core/tests/agents_jobs.rs`
  - `codex-rs/core/tests/agents_delegate.rs`
  - `codex-rs/core/tests/agents_hierarchy.rs`
  - `codex-rs/core/tests/agents_tool_parallelism.rs`
- Registered those four wrappers in `codex-rs/core/Cargo.toml` immediately after the current `agents_runtime` test target.
- Left `codex-rs/core/tests/agents_runtime.rs` as the compatibility wrapper that still includes all four nested modules.
- Did not edit nested test bodies under `codex-rs/core/tests/agents/`.

## Verification

- `cargo metadata --manifest-path codex-rs\core\Cargo.toml --no-deps --format-version 1`
  - Result: passed.
- `git diff --check -- codex-rs\core\Cargo.toml codex-rs\core\tests\agents_runtime.rs codex-rs\core\tests\agents_jobs.rs codex-rs\core\tests\agents_delegate.rs codex-rs\core\tests\agents_hierarchy.rs codex-rs\core\tests\agents_tool_parallelism.rs codex-rs\core\tests\agents\agent_jobs.rs codex-rs\core\tests\agents\codex_delegate.rs codex-rs\core\tests\agents\hierarchical_agents.rs codex-rs\core\tests\agents\tool_parallelism.rs`
  - Result: passed.
  - Note: Git emitted the existing line-ending warning for `codex-rs/core/Cargo.toml`.
- `rg -n --glob 'agents*.rs' 'agents_jobs|agents_delegate|agents_hierarchy|agents_tool_parallelism|agent_jobs|codex_delegate|hierarchical_agents|tool_parallelism' codex-rs/core/Cargo.toml codex-rs/core/tests codex-rs/core/tests/agents`
  - Result: passed.
- Structural checks:
  - Confirmed `Cargo.toml` now lists `agents_runtime`, `agents_jobs`, `agents_delegate`, `agents_hierarchy`, `agents_tool_parallelism`, then `agents_transport`.
  - Confirmed each new wrapper declares `mod support;` and imports the expected nested module with `#[path = "agents/..."]`.

## Notes

- The worktree was already dirty before this slice:
  - `codex-rs/core/Cargo.toml` had a broad pre-existing test-target diff.
  - Nested files under `codex-rs/core/tests/agents/` already had pre-existing import changes.
  - `codex-rs/core/tests/agents_runtime.rs` was already present as an untracked compatibility wrapper.
- Commit was not created because the index contains unrelated staged files outside this worker's ownership and staging the full manifest would also capture unrelated `Cargo.toml` edits. This slice is left unstaged for root integration.
- No compile/build/test command was run. This follows the worker constraint to avoid builds unless root explicitly asks.
