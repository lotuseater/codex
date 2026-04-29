# Custom Codex Self-Review Plan

## Scope

This plan adds a local Codex behavior improvement: after substantial work, Codex
should explicitly review what it changed before finalizing, and the first
iteration of any non-trivial plan should be reviewed before implementation
continues too far.

The goal is not to build a separate review agent in this slice. The useful
first step is a low-risk default instruction that improves every new session
without touching the tool protocol, app-server API, or TUI rendering.

## Sources Inspected

- `codex-rs/models-manager/prompt.md`
- `codex-rs/models-manager/src/model_info.rs`
- `codex-rs/models-manager/src/manager.rs`
- `codex-rs/models-manager/BUILD.bazel`
- `codex-rs/core/src/tools/handlers/plan.rs`
- `codex-rs/core/src/tasks/review.rs`
- `codex-rs/core/src/review_prompts.rs`

## Findings

Codex already has review machinery for explicit review tasks, and it already
has strong planning instructions in the base prompt. The missing part is a
default local habit: reviewing the plan before acting and reviewing the final
work after a large sequence of edits, tests, or commits.

The lowest-risk implementation point is `models-manager`. Bundled model
metadata and remote model metadata both pass through `with_config_overrides`.
Appending a small local instruction overlay there avoids editing the large
`models.json` prompt strings and avoids changing the `update_plan` tool
protocol.

## Plan

1. Add a small self-review instruction file in `codex-rs/models-manager`.
2. Append that instruction to model base instructions unless a user explicitly
   overrides `base_instructions`.
3. Guard against duplicate appends when fallback model metadata is processed
   more than once.
4. Add focused tests for fallback models, remote-style models, and explicit
   base-instruction overrides.
5. Update Bazel compile data so `include_str!` works outside Cargo.

## Behavior

- First plan iteration review:
  - After drafting a non-trivial plan, inspect it for missing verification,
    risky assumptions, dependency/order mistakes, and remote/user-overlap.
  - Update the plan before implementing when the review finds a real issue.
- Post-work review:
  - After a large batch of edits, verification, or commits, inspect the diff,
    tests, docs, and user intent before finalizing.
  - Fix feasible problems immediately instead of only reporting them.

## Later Options

- Add a visible TUI reminder when a long turn has many edits and no recent
  self-review event.
- Add a structured hidden checklist around `update_plan` calls.
- Add telemetry counters for plan revisions and post-work review fixes.
- Integrate with explicit `/review` only after the lightweight default habit is
  proven useful.
