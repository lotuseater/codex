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

## Scoped Review Handoff Findings

### P1 - Runtime workspace roots are still being dropped on turn/session settings updates

`solid_refactor_area_review_session_settings_worker.handoff.md` confirmed the earlier root finding: `turn/start.runtimeWorkspaceRoots` remains public API, but the dirty `SessionSettingsUpdate` / `CodexThread::thread_settings_update` path no longer carries the runtime workspace-root data through to session settings after a thread already exists.

Root-owned next action: restore runtime root propagation through the proper update model instead of passing `None`, then run the focused release-profile session/thread tests named in that handoff.

### P2 - Resume-descendant depth bypasses the extracted agent spawn-depth policy

`solid_refactor_area_review_agent_tools_worker.handoff.md` found that regular child spawn uses the extracted `codex-agent-policy` depth helper, but recursive persisted-descendant resume still computes depth locally. This leaves duplicated policy in `codex-core` and can drift from the new owner crate.

Root-owned next action: route resume-descendant depth through `codex-agent-policy`, add owner-crate tests for the policy, and keep `codex-core` as adapter only.

### P2 - Replacement-shadow source deletion is safe, but dependency cleanup remains incomplete

`solid_refactor_area_review_context_ops_worker.handoff.md` found no live Rust references to `replacement_shadow` and classified the deleted handler as safe. It also confirmed `codex-replacement-shadow` remains a dead `codex-core` dependency while `codex-context-ops-impl` is still required by file-outline/search handlers.

Root-owned next action: remove only the dead `codex-replacement-shadow` dependency from `codex-rs/core/Cargo.toml`; keep `codex-context-ops-impl`; then run the required lock/Bazel follow-up after source boundaries settle.

Self-review update: `7917c50e52` already committed the dead `codex-replacement-shadow` removal from `codex-rs/core/Cargo.toml` and `codex-rs/Cargo.lock` because those worker files were staged when the review handoff commit was made. Treat that source slice as pending verification, not green, until the active replacement-shadow worker or current Cargo/rustc lane reports success.

### P2 - Tests/schema/lock changes need separate commit boundaries

`solid_refactor_area_review_tests_schema_worker.handoff.md` classified the dirty schema, lock, manifest, and test-support changes. It explicitly warns not to commit app-server schema JSON, Bazel scaffold files, workflow prompts/handoffs, and core test-support changes as one blob.

Root-owned next action: commit orchestration docs/prompts separately; commit test-support/stale-test repairs only after focused release verification; commit app-server schema JSON only with the DTO/source change that caused it; keep Bazel/lock refreshes with their owning dependency or schema changes.

## Scoped Reviews Still Outstanding

Core-api visible retry workers were still running without a handoff, so root performed a narrow source review and wrote `.codex/workflow/agents/solid_refactor_area_review_core_api_root_review.handoff.md`.

### P2 - Core-api identifier boundary looks coherent, but lock/Bazel verification remains required

Root found no direct source consumer of `codex_core_api::ThreadId`, `codex_core_api::{... ThreadId ...}`, or `codex_core_api::identifiers` outside the core-api boundary. `ProtocolThreadId` appears only in the core-api export layer, which matches the intended split between protocol and domain identifiers.

`solid_refactor_area_review_retry_core_api_worker.handoff.md` adds a concrete commit blocker: `codex-rs/Cargo.lock` is stale/mixed for the new `codex-core-api -> codex-core-domain-types` dependency. The app-server schema JSON still does not belong to this core-api identifier slice.

Root-owned next action: keep the core-api source slice separate from app-server schema JSON, refresh dependency/Bazel locks after source blockers settle, then run focused release verification plus `just bazel-lock-update` / `just bazel-lock-check` before committing the core-api slice.
