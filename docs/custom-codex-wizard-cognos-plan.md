# Custom Codex Wizard/Cognos Improvement Plan

## Summary

Build small native Codex control-plane improvements that make the local fork
better for Wizard, Team App, memory/cache audits, and loop orchestration. Keep
the first implementation slice read-only or status-oriented where possible.

## Phase 1: Safe Local Fork Operations

- Keep the system `codex` launcher on the official npm-installed Codex.
- Use local fork binaries directly from this repo while developing and testing.
- Add `scripts/clean-fast-release-local.ps1`.
- Do not run it automatically.
- The script cleans only repo-owned build folders under `codex-rs/target`,
  runs a fast release build, and verifies `codex-rs/target/release/codex.exe`
  directly.
- The script must not change PATH, npm shims, `~/.codex/system-wrapper`, or any
  other system-wide launcher state.

## Phase 2: Memory Status API

- Add app-server v2 method `memory/status`.
- Response fields:
  - `memoryRoot`
  - `exists`
  - `memoryMdExists`
  - `memoryMdBytes`
  - `rawMemoriesExists`
  - `rawMemoriesBytes`
  - `rolloutSummaryCount`
  - `extensionResourceCount`
- Behavior:
  - read-only
  - no model calls
  - no DB schema changes
  - no memory pipeline triggering
  - missing memory root returns counts and sizes as zero
- Update app-server docs and generated schema.

## Phase 3: Targeted Multi-Agent Wait

- Extend multi-agent v2 `wait_agent` arguments with optional `targets`.
- Preserve current no-target behavior: wait for mailbox change.
- When `targets` is non-empty:
  - resolve agent ids using existing helper logic
  - wait until any target reaches a final status or timeout expires
  - return a status map keyed by canonical agent path where available
  - emit collab waiting begin/end events with target refs/statuses
- Use the older multi-agent wait implementation as the behavior template.

## Phase 4: Cache And Team App Follow-Up

- Do not port Wizard's full tool-cache into Codex in this slice.
- Next cache step should be `mcp/cache/status` over existing Codex Apps tool
  cache and MCP manager state.
- Next Team App step should be an app-server status endpoint or event stream
  that removes the need for terminal focus/window scraping.

## Verification

- PowerShell script:
  - parse-only check with `System.Management.Automation.Language.Parser`
  - no execution unless manually requested
- Rust:
  - `cargo test -p codex-app-server-protocol`
  - app-server v2 memory status integration test
  - targeted multi-agent v2 wait tests
  - `just write-app-server-schema`
  - `just fmt`
- Rollout:
  - commit and push logical slices
  - after successful Rust checks, rebuild/install only when explicitly requested
