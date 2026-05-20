# core_tests_tools_lane_worker Handoff

Status: owned refactor slice complete; release compile check blocked by unrelated compile errors outside this worker's ownership.

Date: 2026-05-20

## Scope

Owned paths:

- `codex-rs/core/tests/tools.rs`
- `codex-rs/core/tests/suite/mcp_turn_metadata.rs`
- `codex-rs/core/tests/suite/openai_file_mcp.rs`
- `codex-rs/core/tests/suite/pending_input.rs`
- `codex-rs/core/tests/suite/plugins.rs`
- `codex-rs/core/tests/suite/request_plugin_install.rs`
- `codex-rs/core/tests/suite/request_user_input.rs`
- `codex-rs/core/tests/suite/search_tool.rs`
- `codex-rs/core/tests/suite/skill_approval.rs`
- `codex-rs/core/tests/suite/skills.rs`
- `codex-rs/core/tests/suite/tools.rs`
- `codex-rs/core/tests/suite/view_image.rs`
- `codex-rs/core/tests/suite/web_search.rs`

## Changes

- Added the `codex-rs/core/tests/tools.rs` integration-test wrapper.
- The wrapper declares local `support` plus the tools/MCP/plugins suite files with `#[path = "suite/..."]`, matching the existing split-test wrappers.
- No owned suite modules needed source edits. Focused searches found no stale `crate::suite`, `super::suite`, `suite::`, or cross-wrapper references in the owned modules.

## Verification

- `just fmt` from `codex-rs`: passed; formatter reported `64 files left unchanged`.
- `cargo test -p codex-core --test tools --release --no-run` from `codex-rs`: blocked before the new test binary could compile because `codex-thread-store-api` currently fails to compile in `thread/thread-store-api/src/recording.rs`.

Saved log:

- `logs/core-tools-test-no-run-20260521-001005.log`

Observed blocker is outside this worker's owned paths:

- `RecordingLiveThread` does not match the current `LiveThreadHandle` trait signature.
- `RecordingLiveThread` is missing `flush`.
- Several `Option<T>::flatten()` calls are invalid because the values are not nested options.
- `Arc<RecordingLiveThread>` is not coerced to `Arc<dyn LiveThreadHandle>` in the returned future.
- `#[derive(Debug)]` on `RecordingLiveThread` fails because `Arc<dyn ThreadStore>` is not `Debug`.

## Later Focused Command

After the unrelated `codex-thread-store-api` compile errors are fixed, rerun:

```powershell
cd codex-rs
cargo test -p codex-core --test tools --release --no-run
```
