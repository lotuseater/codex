# solid_refactor_fix_session_workspace_roots_worker

You are a visible external Codex implementation worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Other workers may be editing nearby files. Do not revert edits made by others; adapt to the current dirty tree.

Ownership:
- Primary: `codex-rs/core/src/codex_thread.rs`, `codex-rs/core/src/session/session.rs`.
- Tests only if needed: focused app-server/core tests that cover `turn/start.runtimeWorkspaceRoots` and session settings update behavior.
- Do not edit agent policy/depth files, replacement-shadow deps, app-server schema JSON, Bazel files, or unrelated workflow docs.

Read first:
- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-review-findings.md`
- `.codex/workflow/agents/solid_refactor_area_review_session_settings_worker.handoff.md`

Task:
- Fix the P1 regression where `turn/start.runtimeWorkspaceRoots` remains public API but no longer reaches the session settings update path after a thread already exists.
- Preserve real workspace-root/profile-workspace-root data through the proper model/API. Do not pass `None` or drop fields merely to compile.
- Add or adjust focused tests only where needed to prove the update path.

Allowed verification:
- `just fmt` from `codex-rs` after Rust edits.
- Focused release-profile tests only, using `scripts\test-local-codex-release.ps1 -Package <crate> -Filter <specific-filter>` from repo root.
- `just fix -p codex-core` only after the focused tests pass.

Commit/push rule:
- If your slice is fixed and verification is green, commit only your owned files and push if `git rev-list --left-right --count HEAD...origin/slow-context-budget-mode` shows the remote is not ahead.
- If verification is blocked or remote is ahead, do not commit; write a handoff instead.

Handoff:
- Always write `.codex/workflow/agents/solid_refactor_fix_session_workspace_roots_worker.handoff.md` with findings, files changed, verification run, commit/push result or blocker, and exact next action.
