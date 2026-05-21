# SOLID Refactor Review Findings

Date: 2026-05-21
Status: root-authored findings from the partial stopped review plus direct source checks. Treat this as a review queue, not a completed full review.

## Confirmed Findings

### 1. Possible workspace-root override regression blocks the session/thread slice

`CodexThreadSettingsOverrides` still exposes `workspace_roots` and `profile_workspace_roots` as public optional overrides in `codex-rs/core/src/codex_thread.rs:90-91`, but `CodexThread::thread_settings_update` currently destructures and drops those fields at `codex-rs/core/src/codex_thread.rs:298-299`.

`SessionSettingsUpdate` begins at `codex-rs/core/src/session/session.rs:361` and no longer carries workspace-root fields, so the old update path cannot apply them. Before committing this slice, a scoped reviewer must determine whether the API removal is intentional and all callers were updated, or whether the workspace-root data flow must be restored.

### 2. `replacement_shadow.rs` deletion needs dependency cleanup review before commit

`codex-rs/core/src/tools/handlers/replacement_shadow.rs` is deleted, and no `replacement_shadow` Rust references remain in the focused search. However `codex-rs/core/Cargo.toml:122` still depends on `codex-replacement-shadow`.

The same area still has active `codex_context_ops_impl` use in `file_outline.rs` and `search_text.rs`, with `codex-rs/core/Cargo.toml:94` retaining `codex-context-ops-impl`. A reviewer should separate the dead replacement-shadow dependency from still-live context-ops implementation calls before root changes manifests.

### 3. The broad handoff-review worker was stopped and has no authoritative handoff

`solid_refactor_review_handoffs_worker` was stopped before it wrote `.codex/workflow/agents/solid_refactor_review_handoffs_worker.handoff.md`. Its visible log showed broad source scanning beyond the intended bounded review, so it should be treated only as partial evidence.

Do not use that worker as a completed review gate. The replacement plan is multiple narrow visible reviewers, each with a small file set and a required handoff-review.

## Verification Gaps From Completed Handoffs

- Wave 3/4 source workers mostly obeyed their command bans, so source changes still need root-owned formatting, focused release checks/tests, and scoped `just fix -p`.
- The core-api identifier move still needs lock/Bazel/schema follow-up after source review confirms the boundary is correct.
- The stale test API repair handoff reports edits but no test run; it needs targeted release test execution after review.
- Generated app-server schema files are dirty and should be committed only with the DTO/source changes that caused them.

## Scoped Reviews To Launch

- Session settings and workspace-root data flow.
- Context-ops and replacement-shadow deletion/dependency cleanup.
- Core-api identifier export move and consumer fallout.
- Agent policy plus tools telemetry boundary.
- Core tests, schema fixtures, manifest/Bazel/lock fallout.
