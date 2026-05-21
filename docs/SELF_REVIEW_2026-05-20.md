# Self Review: Architecture SOLID Review Slice

Date: 2026-05-20

## Scope

Reviewed the just-completed Markdown architecture-review slice centered on
`docs/current-project-architecture-solid-review.md`. The wider worktree already
contains many unrelated dirty paths, so this review intentionally limited fixes
to the documented slice and the requested self-review note.

## Findings

### 1. Incorrect dependency evidence for `codex-core-api`

Severity: Medium

`docs/current-project-architecture-solid-review.md` said
`codex-rs/core-api/Cargo.toml` still depends on `codex-core`. That is incorrect:
the current manifest depends on API/config/protocol crates such as
`codex-app-server-protocol`, `codex-config`, and `codex-protocol`, but not
`codex-core`.

The architectural concern is still valid in a narrower form: extension crates
such as `codex-rs/ext/memories` and `codex-rs/ext/guardian` still depend
directly on `codex-core`, so `codex-core-api` is not yet the extension-facing
boundary the review discusses.

Fix applied:

- Corrected the evidence in
  `docs/current-project-architecture-solid-review.md`.
- Reworded the recommendation to preserve the current core-free state of
  `codex-core-api` and migrate extension crates toward boundary crates instead
  of direct `codex-core` dependencies.
- Updated the phase-plan text to match the corrected direction.

## Checks

- Verified the target document is an ignored/untracked docs artifact in the
  current checkout, so the fix did not touch unrelated dirty tracked code.
- Verified line-count claims in the review against the current source files.
- Verified `codex-rs/core-api/Cargo.toml` does not contain a `codex-core`
  dependency.
- Verified `codex-rs/ext/memories/Cargo.toml` and
  `codex-rs/ext/guardian/Cargo.toml` still depend on `codex-core`.

No Rust tests were required because the fix changes Markdown only.
