# SOLID Refactor Wave 20 Core Test Support Responses Repair Worker Handoff

Classification: accepted

## Decision

- Kept the response helper implementation owned by `codex-test-support-responses`.
- Kept `codex_core_test_runtime::responses` as the preferred import surface for core topic tests.
- Restored `core_test_support::responses` as a thin compatibility adapter because non-core topic adapters still import it and this worker does not own those migrations.
- Restored `core_test_support::streaming_sse` as a compatibility re-export for the same reason.
- Updated `common/context_snapshot.rs` to depend on `crate::responses::ResponsesRequest` so the shared source works through either crate boundary.

## Changed files

- `codex-rs/core/tests/common/lib.rs`
- `codex-rs/core/tests/common/context_snapshot.rs`
- `.codex/workflow/agents/solid_refactor_wave20_core_tests_support_responses_repair_worker.handoff.md`

## Verification

```powershell
rg -n "responses|core_test_support|codex_core_test_runtime|codex-test-support-responses" codex-rs/core/tests/common codex-rs/core/tests/support codex-rs/core/tests
```

Result: passed with exit code 0. Output shows core tests using `codex_core_test_runtime::responses`, `core_test_support` keeping the compatibility dependency, and the new adapter in `common/lib.rs`.

```powershell
git diff --check -- codex-rs/core/tests/common codex-rs/core/tests/support .codex/workflow/agents/solid_refactor_wave20_core_tests_support_responses_repair_worker.handoff.md
```

Result: passed with exit code 0.

## Commit

Not committed. The owned files already contained broad unstaged Wave 19 edits before this worker slice; staging whole files would include unrelated dirty work, and staging only this worker's hunks would not represent a coherent standalone change against `HEAD`.
