# core_test_split_analysis_proto Handoff

Date: 2026-05-20

## Files Changed

- `.codex/prototypes/plan-core-test-split.ps1`
  - Read-only PowerShell prototype that inventories `codex-rs/core/tests/suite/*.rs`.
  - Reports file size, line count, approximate `#[test]` and `#[tokio::test]` counts, `super::` references, import roots, dependency hints, and a suggested split-lane label.
  - Supports Markdown output by default and `-Json` for machine-readable follow-up planning.
- `.codex/workflow/agents/core_test_split_analysis_proto.handoff.md`
  - This handoff.

No Rust source, manifests, lockfiles, Bazel files, generated files, snapshots, staging, commits, Cargo, Just, or formatter lanes were touched.

## Script Usage

From the repo root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .codex\prototypes\plan-core-test-split.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .codex\prototypes\plan-core-test-split.ps1 -SortBy Size -Top 12
powershell -NoProfile -ExecutionPolicy Bypass -File .codex\prototypes\plan-core-test-split.ps1 -SortBy Tests -Top 10
powershell -NoProfile -ExecutionPolicy Bypass -File .codex\prototypes\plan-core-test-split.ps1 -Json
```

Parameters:

- `-RepoRoot <path>` defaults to the repo root inferred from the script location.
- `-SortBy Name|Size|Tests|Lane` controls ordering.
- `-Top <n>` limits displayed rows after sorting.
- `-Json` emits the row objects instead of Markdown.

## Verification

Ran successfully:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .codex\prototypes\plan-core-test-split.ps1 -SortBy Size -Top 12
```

Also checked:

```powershell
.codex\prototypes\plan-core-test-split.ps1 -Json
.codex\prototypes\plan-core-test-split.ps1 -SortBy Tests -Top 5
```

## Sample Findings

Full suite inventory summary:

- 85 suite files
- 730 approximate test attributes
- 2383.7 KB total source size
- 2 files with `super::` references:
  - `compact_resume_fork.rs` -> `super::compact`
  - `window_headers.rs` -> `super::compact`

Lane summary from the prototype:

```text
auth-login: files=2; tests=11; sizeKB=14.1
config: files=19; tests=98; sizeKB=232.6
conversation-state: files=3; tests=9; sizeKB=53.3
exec-sandbox: files=11; tests=48; sizeKB=121.5
mcp-tools: files=20; tests=89; sizeKB=304.4
protocol-responses: files=8; tests=15; sizeKB=55.8
review-large: files=21; tests=460; sizeKB=1600.0
support/no-tests: files=1; tests=0; sizeKB=2.1
```

Largest files from `-SortBy Size -Top 12`:

```text
realtime_conversation.rs: 131.2 KB, tests=38, tokio=38, super=0, lane=review-large
compact.rs: 130.1 KB, tests=25, tokio=25, super=0, lane=review-large
hooks.rs: 126.4 KB, tests=37, tokio=37, super=0, lane=review-large
approvals.rs: 119.9 KB, tests=10, tokio=10, super=0, lane=review-large
compact_remote.rs: 118.9 KB, tests=29, tokio=29, super=0, lane=review-large
client.rs: 107.0 KB, tests=36, tokio=36, super=0, lane=review-large
unified_exec.rs: 103.2 KB, tests=31, tokio=31, super=0, lane=review-large
code_mode.rs: 89.7 KB, tests=39, tokio=39, super=0, lane=review-large
rmcp_client.rs: 88.3 KB, tests=15, tokio=14, super=0, lane=review-large
client_websockets.rs: 73.4 KB, tests=36, tokio=36, super=0, lane=review-large
request_permissions.rs: 65.6 KB, tests=14, tokio=14, super=0, lane=review-large
apply_patch_cli.rs: 63.7 KB, tests=35, tokio=35, super=0, lane=review-large
```

Interpretation:

- The obvious first split target is not a dependency knot: nearly all files have no `super::` references.
- The largest 21 files dominate the test count and size, so root should treat `review-large` as the first manual bucketing pass rather than a final lane.
- `compact_resume_fork.rs` and `window_headers.rs` both depend on `super::compact`; keep those with `compact.rs` or move the shared fixture/helper out before creating separate binaries.
- The straightforward labels (`mcp-tools`, `config`, `exec-sandbox`, `protocol-responses`, `auth-login`, `conversation-state`) look usable as initial release-test lanes after root chooses the Rust split mechanism.

## Recommended Next Root Action

Use the JSON output to draft the first concrete split plan around independent test binaries or modules:

1. Keep `compact.rs`, `compact_remote.rs`, `compact_resume_fork.rs`, and `window_headers.rs` together until the `super::compact` dependency is removed or promoted into shared test support.
2. Carve out the smaller straightforward lanes first (`config`, `mcp-tools`, `exec-sandbox`, `protocol-responses`) because they have low `super::` coupling and clear dependency hints.
3. Re-run this prototype after root edits `suite/mod.rs` or introduces new harness files, then compare lane counts before release-test verification.
