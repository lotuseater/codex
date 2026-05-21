# solid_refactor_fix_replacement_shadow_dep_worker Handoff

## Findings

- `codex-replacement-shadow` is no longer a `codex-core` dependency in `codex-rs/core/Cargo.toml`.
- `codex-rs/Cargo.lock` no longer lists `codex-replacement-shadow` in the `codex-core` package dependency list. The `codex-replacement-shadow` package entry remains because it is still a workspace crate.
- `codex-context-ops-impl` remains in `codex-rs/core/Cargo.toml` and is still referenced by the context-ops file-outline/search handlers.
- The current working tree still has unrelated dirty additions in `codex-rs/core/Cargo.toml` and `codex-rs/Cargo.lock` for other refactor slices, including `codex-thread-store` and `codex-core-domain-types`/`serde`. This worker did not stage or revert them.

## Files Changed

- `codex-rs/core/Cargo.toml`
- `codex-rs/Cargo.lock`
- `.codex/workflow/agents/solid_refactor_fix_replacement_shadow_dep_worker.handoff.md`

## Verification

- `just bazel-lock-update` from repo root: passed.
- `just bazel-lock-check` from repo root: passed.
- `cargo check -p codex-core --release --locked` from `codex-rs`: passed.
  - Log: `logs/solid-refactor-codex-core-release-check-20260521-125117.log`
  - Result: `Finished release profile [optimized] target(s) in 2m 02s`; existing `codex-core` warnings only.

## Commit / Push Result

- While this worker was preparing a partial commit, the branch tip advanced; current `HEAD` is `8067147750 Document replacement shadow verification state`.
- That current `HEAD` already contains the `codex-replacement-shadow` removals from `codex-rs/core/Cargo.toml` and the `codex-core` dependency list in `codex-rs/Cargo.lock`.
- `git rev-list --left-right --count HEAD...origin/slow-context-budget-mode` reports `0 0`, so there was no local worker commit to push.

## Exact Next Action

- Root should treat the stale `codex-replacement-shadow` core dependency finding as fixed and already present at current branch `HEAD`.
- Leave the unrelated dirty `codex-thread-store` and `codex-core-domain-types`/`serde` manifest/lock hunks to their owning workers.
