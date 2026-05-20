# Hook Runtime Compile Scout Handoff

Date: 2026-05-20

## Scope

Read-only scout for the hook runtime compile blocker in
`C:\Users\Oleh\Documents\GitHub\open_ai\codex`. I inspected the requested
handoffs and source files and did not edit Rust source, manifests, lockfiles,
Bazel files, generated files, tests, snapshots, staging, or commits.

Sources inspected:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/compile_hook_skill_scout.handoff.md`
- `codex-rs/core/src/hook_runtime.rs`
- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/tasks/mod.rs`

## Current Hook Runtime Contract

`codex-rs/core/src/hook_runtime.rs` is already on the `HookRuntimeOutcome`
contract:

```rust
pub(crate) struct HookRuntimeOutcome {
    pub should_stop: bool,
    pub additional_contexts: Vec<String>,
}
```

Relevant functions:

- `inspect_pending_input(sess, turn_context, pending_input_item: &TurnInput) -> HookRuntimeOutcome`
- `record_pending_input(sess, turn_context, pending_input: TurnInput, additional_contexts: Vec<String>)`
- `record_additional_contexts(sess, turn_context, additional_contexts: Vec<String>)`

Do not restore `PendingInputHookDisposition` or add a compatibility enum. The
current source of truth is `HookRuntimeOutcome`, where `should_stop == true`
means blocked/stop and `should_stop == false` means accepted.

Search result: `HookExecutionDisposition` was not found under
`codex-rs/core/src`; the stale symbol in this tree is
`PendingInputHookDisposition`.

## Exact Stale Callsites

### `codex-rs/core/src/session/turn.rs`

Stale import:

- `use crate::hook_runtime::PendingInputHookDisposition;`

Stale pending-input hook loop:

```rust
match inspect_pending_input(&sess, &turn_context, pending_input_item).await {
    PendingInputHookDisposition::Accepted(pending_input) => {
        accepted_pending_input.push(*pending_input);
    }
    PendingInputHookDisposition::Blocked {
        additional_contexts,
    } => {
        let remaining_pending_input = pending_input_iter.collect::<Vec<_>>();
        if !remaining_pending_input.is_empty() {
            let _ = sess.prepend_pending_input(remaining_pending_input).await;
            requeued_pending_input = true;
        }
        blocked_pending_input_contexts = additional_contexts;
        blocked_pending_input = true;
        break;
    }
}
```

Stale recording call:

```rust
for pending_input in accepted_pending_input {
    record_pending_input(&sess, &turn_context, pending_input).await;
}
```

Replacement shape:

```rust
let outcome = inspect_pending_input(&sess, &turn_context, &pending_input_item).await;
if outcome.should_stop {
    let remaining_pending_input = pending_input_iter.collect::<Vec<_>>();
    if !remaining_pending_input.is_empty() {
        let _ = sess.prepend_pending_input(remaining_pending_input).await;
        requeued_pending_input = true;
    }
    blocked_pending_input_contexts = outcome.additional_contexts;
    blocked_pending_input = true;
    break;
}

accepted_pending_input.push((pending_input_item, outcome.additional_contexts));
```

Then record accepted input with the new arity:

```rust
for (pending_input, additional_contexts) in accepted_pending_input {
    record_pending_input(&sess, &turn_context, pending_input, additional_contexts).await;
}
```

Keep the existing blocked-context recording after the loop:

```rust
record_additional_contexts(&sess, &turn_context, blocked_pending_input_contexts).await;
```

This preserves the existing behavior that a blocked current item is not
recorded, remaining later items are requeued, and already accepted items are
recorded before the stop handling continues.

### `codex-rs/core/src/tasks/mod.rs`

Stale import:

- `use crate::hook_runtime::PendingInputHookDisposition;`

Stale pending-input hook loop:

```rust
match inspect_pending_input(self, &turn_context, pending_input_item).await {
    PendingInputHookDisposition::Accepted(pending_input) => {
        record_pending_input(self, &turn_context, *pending_input).await;
    }
    PendingInputHookDisposition::Blocked {
        additional_contexts,
    } => {
        record_additional_contexts(self, &turn_context, additional_contexts).await;
    }
}
```

Replacement shape:

```rust
let outcome = inspect_pending_input(self, &turn_context, &pending_input_item).await;
if outcome.should_stop {
    record_additional_contexts(self, &turn_context, outcome.additional_contexts).await;
} else {
    record_pending_input(
        self,
        &turn_context,
        pending_input_item,
        outcome.additional_contexts,
    )
    .await;
}
```

This preserves the current task path behavior: accepted pending input is
recorded immediately; blocked pending input only records hook-injected context.

## Independence Assessment

This compile fix can be edited independently of the session input queue and
thread-store integration work if the worker limits the patch to:

- removing the stale `PendingInputHookDisposition` imports,
- updating the two pending-input hook loops to consume `HookRuntimeOutcome`,
- passing `additional_contexts` into `record_pending_input`.

It does not require changes to `hook_runtime.rs`, manifests, lockfiles, Bazel
files, generated schemas, thread-store crates, or session queue data models.

## Path Ownership And Conflict Risks

- `codex-rs/core/src/session/turn.rs`: moderate conflict risk. This file is
  already locally modified by another session at
  `sess.record_model_move_finished_for_semantic_compact().await;`, and other
  workers may be touching pending-input/session queue logic. Worker should read
  current file state before patching and preserve unrelated edits.
- `codex-rs/core/src/tasks/mod.rs`: moderate conflict risk because it has a
  parallel pending-input hook path and may be touched by task/session
  integration work.
- `codex-rs/core/src/hook_runtime.rs`: no source change recommended for this
  blocker. Avoid adding one-off helper methods; repo rules discourage helpers
  used only once.

## Recommended Worker Prompt

```text
Work in C:\Users\Oleh\Documents\GitHub\open_ai\codex. Fix only the stale
PendingInputHookDisposition compile blocker. You are not alone in this
worktree; read current file state first and do not revert unrelated edits from
other sessions.

In codex-rs/core/src/session/turn.rs and codex-rs/core/src/tasks/mod.rs, remove
the PendingInputHookDisposition import and update pending-input hook handling to
use HookRuntimeOutcome returned by inspect_pending_input. Borrow pending input
when calling inspect_pending_input. Treat outcome.should_stop=true as blocked
and false as accepted. Preserve accepted pending input ordering and the existing
session requeue behavior. Pass outcome.additional_contexts into
record_pending_input for accepted input and record_additional_contexts for
blocked input.

Do not touch manifests, lockfiles, Bazel files, generated files, snapshots,
thread-store integration, session queue model changes, or unrelated compile
blockers. After edits, run just fmt in codex-rs, then run the focused release
lane if feasible:
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core
```

## Verification Lane

For an implementation worker:

1. Run `just fmt` from `codex-rs` after Rust edits.
2. Run
   `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core`
   from repo root if feasible.
3. If this exposes unrelated core compile blockers, report them separately and
   do not expand this slice into skill dependency, plugin tool, input queue, or
   thread-store work.

Scout verification performed:

- Read-only `rg` confirmed no `HookExecutionDisposition` under
  `codex-rs/core/src`.
- Read-only `rg` found `PendingInputHookDisposition` only in
  `codex-rs/core/src/session/turn.rs` and `codex-rs/core/src/tasks/mod.rs`.
- Read-only `rg` found the only stale `record_pending_input(...)` callsites in
  those same two files.

## Commit Readiness

The scout handoff file is ready as a standalone documentation artifact if the
user wants scout outputs committed.

The implementation commit should include only:

- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/tasks/mod.rs`
- formatting changes produced by `just fmt`, if any

Do not include the unrelated existing local edit in `session/turn.rs` unless it
belongs to the same implementation worker's verified slice.
