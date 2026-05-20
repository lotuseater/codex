# Agent Prompt: compile_session_store_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only scout for current compile blockers involving session input,
thread settings, and thread-store concrete types.

First read:

- `.codex/workflow/agents/thread_store_boundary.handoff.md`
- `.codex/workflow/agents/canary_observer.handoff.md`
- `.codex/workflow/solid-refactor-handoff.md`

Task:

- Use targeted searches for `input_queue`, `thread_settings`,
  `Op::UserInput`, `LocalThreadStore`, `LocalThreadStoreConfig`,
  `thread_store_from_config`, and `InMemoryThreadStore`.
- Do not edit source files.
- Do not run Cargo, Just, formatters, or Git staging/commits.
- You may delegate focused read-only questions to helper agents if useful.

Write `.codex/workflow/agents/compile_session_store_scout.handoff.md` with:

- exact compile blockers and likely current type/source of truth
- recommended fix order
- exact files likely touched
- which part is root-owned versus delegate-safe
- whether this should be split into smaller implementation workers
