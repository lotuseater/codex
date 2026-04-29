# Custom Codex Wizard/Cognos Improvement Plan

## Summary

This branch keeps the system-wide `codex` launcher on the official web/npm
install and develops the custom fork as a repo-local executable. The first
implementation slice is token/cost reduction inside Codex itself, based on the
recent Wizard/Cognos research and local Codex history: defer MCP schemas by
default and avoid resending repeated large tool outputs in the prompt.

The remote-overlap ideas from the earlier plan, `memory/status` and targeted
multi-agent `wait_agent`, are parked in this branch. They are useful, but they
touch API areas likely to move remotely, so the local fork should concentrate on
custom behavior that directly improves this machine's automation cost.

## Phase 1: Safe Local Fork Operations

- Keep the system `codex` launcher on the official npm-installed Codex.
- Use local fork binaries directly from this repo while developing and testing.
- Keep `scripts/clean-fast-release-local.ps1` as a manual-only helper.
- The script defaults to an incremental `dev-small` local build of
  `codex-cli`, writes `codex-rs/target/dev-small/codex.exe`, and leaves all
  build folders intact.
- Use `-BuildMode FastRelease` only when a release-shaped binary is needed.
- Use `-Clean` or `-CleanDebug` only when intentionally reclaiming build space.
- The script must not change PATH, npm shims, `~/.codex/system-wrapper`, or any
  other system-wide launcher state.

## Phase 2: MCP Schema Token Saving

Implement the existing `tool_search_always_defer_mcp_tools` feature as a local
default.

- Enable the feature by default in the custom fork.
- Preserve explicit app tools as direct tools when an app is mentioned.
- Keep small direct MCP exposure test coverage by explicitly disabling the
  feature in that test.
- Keep the config flag available so the behavior can be disabled locally if a
  workflow needs direct MCP tool exposure.

Expected effect: large MCP schemas are not eagerly included in every prompt.
The model receives `tool_search` and can discover MCP tools only when needed.

## Phase 3: Prompt Output Reference Cache

Add prompt-time duplicate elision for repeated large plain-text tool outputs.

- Apply only inside `ContextManager::for_prompt`, after history normalization.
- Keep raw history, transcript rendering, logs, and stored items unchanged.
- Hash normalized large text outputs and keep the first full occurrence.
- Replace later identical large outputs with a compact reference containing the
  earlier `call_id`, digest, and normalized byte count.
- Skip small outputs, structured content items, image outputs, and active
  running-process output.
- Normalize volatile exec metadata such as `Wall time:` and `Chunk ID:` before
  duplicate detection.

Expected effect: repeated `Get-Content`, `rg`, and other large tool results no
longer consume prompt tokens multiple times in long automation sessions.

## Parked Work

- `memory/status` app-server API: useful as a read-only status surface, but
  parked because app-server API work may overlap remote changes.
- Targeted multi-agent v2 `wait_agent`: useful for Team App and loop control,
  but parked because multi-agent control is also likely to move remotely.
- `mcp/cache/status`: keep as a follow-up observability idea after the prompt
  savings are verified in real sessions.

## Verification

- PowerShell script:
  - parse-only check with `System.Management.Automation.Language.Parser`
  - dry-run check with `-WhatIf`
  - no real build execution unless manually requested
- Rust:
  - `cargo test -p codex-features tool_search`
  - `cargo test -p codex-core mcp_tool_exposure`
  - `cargo test -p codex-core context_manager`
  - `just fmt`
  - scoped `just fix -p codex-features`
  - scoped `just fix -p codex-core`
- Rollout:
  - commit logical slices
  - push `local-codex-customizations`
  - rebuild local release binary only when manually requested
