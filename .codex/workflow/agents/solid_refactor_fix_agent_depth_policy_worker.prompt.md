# solid_refactor_fix_agent_depth_policy_worker

You are a visible external Codex implementation worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are not alone in the codebase. Other workers may be editing nearby files. Do not revert edits made by others; adapt to the current dirty tree.

Ownership:
- Primary: `codex-rs/agent-policy/src/lib.rs`, `codex-rs/core/src/agent/policy.rs`, and the MultiAgentV2 resume/registry callsites that compute persisted-descendant depth.
- Do not edit session settings files, replacement-shadow dependency files, app-server schema JSON, or unrelated workflow docs.

Read first:
- `AGENTS.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/solid-refactor-review-findings.md`
- `.codex/workflow/agents/solid_refactor_area_review_agent_tools_worker.handoff.md`

Task:
- Fix the P2 boundary leak where resume-descendant depth still computes child depth locally instead of using `codex-agent-policy`.
- Keep `codex-core` as adapter; owner policy logic and tests belong in `codex-agent-policy`.
- Add focused owner-crate tests for the depth policy if missing.

Allowed verification:
- `just fmt` from `codex-rs` after Rust edits.
- Focused release-profile tests only, especially `scripts\test-local-codex-release.ps1 -Package codex-agent-policy` and any precise core filter needed for resume behavior.
- `just fix -p codex-agent-policy` and `just fix -p codex-core` only after focused tests pass.

Commit/push rule:
- If your slice is fixed and verification is green, commit only your owned files and push if `git rev-list --left-right --count HEAD...origin/slow-context-budget-mode` shows the remote is not ahead.
- If verification is blocked or remote is ahead, do not commit; write a handoff instead.

Handoff:
- Always write `.codex/workflow/agents/solid_refactor_fix_agent_depth_policy_worker.handoff.md` with findings, files changed, verification run, commit/push result or blocker, and exact next action.
