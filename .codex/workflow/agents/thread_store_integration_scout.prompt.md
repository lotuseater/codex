# Agent Prompt: thread_store_integration_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only thread-store integration scout.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/thread_store_boundary.handoff.md`
- `.codex/workflow/agents/thread_projection_boundary.handoff.md`
- `.codex/workflow/agents/compile_session_store_scout.handoff.md`

Task:

- Inspect thread-store, app-server thread processors, and core callsites that
  consume thread metadata, summaries, pagination, or projection data.
- Identify exact callsite changes needed to carry real data through the new
  boundary instead of placeholders.
- Do not edit source files, manifests, lockfiles, generated files, or handoff
  documents other than your own handoff.
- Do not run Cargo, Just, formatters, or Git staging/commits.
- You may delegate focused read-only questions to helper agents if useful.

Write `.codex/workflow/agents/thread_store_integration_scout.handoff.md` with:

- exact callsites that still need integration
- data fields that must not be dropped or replaced with placeholders
- suggested ownership split for future implementation workers
- commit readiness notes, but do not make commits
