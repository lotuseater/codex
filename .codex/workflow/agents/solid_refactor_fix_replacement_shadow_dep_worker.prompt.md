# solid_refactor_fix_replacement_shadow_dep_worker

You are a visible external Codex implementation worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Other workers may be editing nearby files. Do not revert edits made by others; adapt to the current dirty tree.

Ownership:
- Primary: `codex-rs/core/Cargo.toml`.
- Secondary only if required by dependency tooling: `codex-rs/Cargo.lock`, `MODULE.bazel.lock`, and Bazel lock/build metadata directly caused by removing the dead `codex-core` dependency.
- Do not edit session settings, agent policy/depth files, app-server schema JSON, or unrelated workflow docs.

Read first:
- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-review-findings.md`
- `.codex/workflow/agents/solid_refactor_area_review_context_ops_worker.handoff.md`

Task:
- Remove only the dead `codex-replacement-shadow` dependency from `codex-rs/core/Cargo.toml`.
- Keep `codex-context-ops-impl` because it is still used by file-outline/search handlers.
- Refresh only the dependency/lock artifacts required by this dependency removal, and avoid absorbing unrelated dirty lock/schema changes.

Allowed verification:
- `just fmt` is not needed unless Rust formatting files change.
- If dependency files change, run `just bazel-lock-update` and `just bazel-lock-check` from repo root only after confirming no other Cargo/rustc/link process is active.
- Use the narrowest release-profile check that covers `codex-core` dependency resolution; do not run broad debug Cargo.

Commit/push rule:
- If your slice is fixed and verification is green, commit only your owned files and push if `git rev-list --left-right --count HEAD...origin/slow-context-budget-mode` shows the remote is not ahead.
- If lockfile conflicts with other dirty slices or remote is ahead, do not commit; write a handoff instead.

Handoff:
- Always write `.codex/workflow/agents/solid_refactor_fix_replacement_shadow_dep_worker.handoff.md` with findings, files changed, verification run, commit/push result or blocker, and exact next action.
