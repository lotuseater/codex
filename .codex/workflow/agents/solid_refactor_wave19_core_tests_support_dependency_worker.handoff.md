# SOLID Refactor Wave 19 Core Tests Support Dependency Worker Handoff

Classification: root-wiring-needed

## Changed files

- `codex-rs/core/tests/common/lib.rs`
- `codex-rs/core/tests/common/Cargo.toml`
- `.codex/workflow/agents/solid_refactor_wave19_core_tests_support_dependency_worker.handoff.md`

## Dependency fan-in reduced

- Removed `apps_test_server` from the `core_test_support` public module surface.
- Dropped `codex-features`, `codex-login`, and `wiremock` from `core/tests/common/Cargo.toml`.
- Reasoning: current source search found all `apps_test_server` consumers using `codex_core_test_runtime::apps_test_server`; no `core_test_support::apps_test_server` consumers were found. The path-owned `apps_test_server.rs` file is still available to `codex_core_test_runtime` through its existing `#[path = "../../../core/tests/common/apps_test_server.rs"]` adapter, so the app-server fixture behavior is preserved there.

## Why root wiring is still needed

- `codex-rs/core/Cargo.toml` already had concurrent unstaged edits before this worker slice. I did not edit it.
- Current broader tree search still finds outside-owned consumers such as `codex-rs/exec/tests/**` and `codex-rs/memories/write/src/startup_tests.rs` importing heavy runtime helpers through `core_test_support::test_codex*` / `core_test_support::responses`. This worker did not edit those topic files.
- The existing split leaves heavier runtime files still physically under `core/tests/common/**` for `codex_core_test_runtime` path imports. Moving those into `codex-rs/test-support/core-runtime/src/**` is outside this worker's ownership.

## Commit

- Not committed.
- Reason: the worktree contains broader unrelated dirty work, including pre-existing edits in the same owned support files. Staging whole files would include changes outside this worker slice, so I left the slice unstaged.

## Verification

```powershell
rg -n "^(use|pub use|pub mod|mod)|codex_core|codex_[a-z_]+|wiremock|tokio|TestCodex|test_codex|responses" codex-rs/core/tests/common/lib.rs codex-rs/core/tests/common/Cargo.toml codex-rs/core/tests/support/mod.rs
```

Result: passed with exit code 0. Output shows the remaining expected `codex-test-support-responses`, `tokio`, lightweight support exports, and test binary dispatch imports. It no longer reports `wiremock` in these checked files.

```powershell
git diff --check -- codex-rs/core/tests/common codex-rs/core/tests/support codex-rs/core/Cargo.toml .codex/workflow/agents/solid_refactor_wave19_core_tests_support_dependency_worker.handoff.md
```

Result: passed with exit code 0. Git printed CRLF normalization warnings for existing dirty files, but no whitespace errors.

## Remaining fallout

- Root should coordinate the already-staged `codex-rs/core/Cargo.toml` edits with the support split.
- Root should decide whether outside-owned consumers of `core_test_support::test_codex*` and `core_test_support::responses` move to `codex_core_test_runtime` or keep a compatibility adapter.
- No Cargo, Rust tests, formatters, schema generation, Bazel, lock refresh, release builds, deploy, or activation were run.
