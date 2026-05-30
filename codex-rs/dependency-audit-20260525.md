# Codex Rust Dependency Audit - 2026-05-25

## Findings

- `codex-rs/tui/Cargo.toml` directly depended on renderer implementation crates: `diffy`, `pathdiff`, `syntect`, `two-face`, and `urlencoding`.
- Source search found those crates used from `codex-rs/tui-render/src`, while `codex-rs/tui` already depends on the `codex-tui-render` abstraction.
- Keeping those dependencies in `tui` widened the direct dependency surface and coupled the UI shell to renderer internals. Removing them reduces direct dependencies without changing the transitive graph, because `tui-render` still owns the real implementation need.
- `codex-rs/codex-mcp/Cargo.toml` declared `rmcp` in both normal dependencies and dev-dependencies with the same feature set.
- `codex-rs/codex-mcp` has its test support under `src`, so the tests compile in the crate context and can use the existing normal dependency. The duplicate dev-dependency is not needed.

## Changes Made

- Removed renderer-only direct dependencies from `codex-rs/tui/Cargo.toml`: `diffy`, `pathdiff`, `syntect`, `two-face`, and `urlencoding`.
- Left those dependencies in `codex-rs/tui-render/Cargo.toml`, where the renderer code uses them.
- Removed the duplicate `rmcp` dev-dependency from `codex-rs/codex-mcp/Cargo.toml`.

## Verification Scope

- Static manifest and source checks only, per request.
- No Cargo build, test, metadata, or lockfile regeneration was run.
- Existing unrelated `codex-rs/Cargo.lock` changes were left untouched.
