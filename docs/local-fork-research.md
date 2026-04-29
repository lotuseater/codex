# Local Codex Fork Research

## Current Launch Chain

On this PC, `codex` resolves first to `C:\Users\Oleh\.codex\system-wrapper\codex.ps1`, then `codex.cmd`. Both wrappers call Wizard's `codex_wrapper_launcher.py` with `system.codex-wrapper.env.json`.

The current live wrapper environment keeps Wizard stop hooks and launcher behavior outside this repo. The effective Codex executable is selected by `WIZARD_CODEX_REAL_EXE`. That makes the safest substitution point the wrapper environment JSON, not PATH mutation or editing npm shims.

## Repo Feasibility

This repo is a real Codex CLI source tree. The Rust workspace builds the `codex` binary from `codex-rs/cli`, and `docs/install.md` documents `cargo build` from `codex-rs`. On Windows, this local machine can still use a locally built `codex.exe` even though the upstream docs describe Windows support primarily through WSL2.

Relevant existing primitives:

- Token accounting is persisted through protocol/state `TokenUsage` and app-server thread metadata.
- Context compaction already exists through `/compact` and app-server `thread/compact/start`.
- App-server v2 already exposes `thread/start`, `thread/resume`, `thread/fork`, `thread/turns/list`, `thread/inject_items`, `turn/start`, and `turn/interrupt`.
- MCP tool names, filtering, schema shaping, and tool cache boundaries live in `codex-rs/codex-mcp`.
- Tool output truncation and unified exec output accounting already exist in `codex-rs/core/src/tools/context.rs`.
- PowerShell detection, command extraction, and UTF-8 output prefixing live in `codex-rs/shell-command/src/powershell.rs`.

## Token And Automation Findings

The highest value token-saving path is not a broad prompt change. The repo already has structured APIs for compacting, resuming without full turn payloads, deferred tool loading, and truncating tool output. A local fork should first add measurement and better defaults around those seams, then connect Wizard and Team App to app-server v2 rather than hardcoding Wizard behavior into Codex core.

The Wizard MCP tools repaired earlier provide cross-session reports for cache misses, first-move prediction, and history patterns. Those tools should remain an external orchestration layer. Codex should expose stable control surfaces and concise telemetry; Wizard should decide policy.

## System-Wide Replacement Strategy

The local fork can safely substitute system-wide by:

1. building `codex-rs\target\release\codex.exe`;
2. backing up `C:\Users\Oleh\.codex\system-wrapper\system.codex-wrapper.env.json`;
3. changing only `WIZARD_CODEX_REAL_EXE` to the local binary;
4. verifying `Get-Command codex -All` still resolves to the system wrapper first;
5. verifying `codex --version` and optional `codex exec` smoke behavior.

Rollback is restoring the prior wrapper environment JSON from the manifest backup.

The local fork is intentionally visible in the interactive TUI session header as `Wizard_Codex_April_29_2_49`, so a new session can be distinguished from stock Codex without inspecting the wrapper environment.
