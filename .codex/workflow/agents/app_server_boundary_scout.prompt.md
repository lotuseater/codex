# Agent Prompt: app_server_boundary_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only app-server boundary scout.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/app_catalog_followup.handoff.md`
- `.codex/workflow/agents/thread_store_boundary.handoff.md`
- `.codex/workflow/agents/thread_projection_boundary.handoff.md`

Task:

- Inspect current `codex-rs/app-server` changes and identify places where
  app-server is still carrying business/domain logic that should live in a
  dedicated domain/API crate.
- Focus on request processors, app catalog protocol handling, thread-store
  calls, and config/external-agent boundaries.
- Do not edit source files, manifests, lockfiles, generated files, or handoff
  documents other than your own handoff.
- Do not run Cargo, Just, formatters, or Git staging/commits.
- You may delegate focused read-only questions to helper agents if useful.

Write `.codex/workflow/agents/app_server_boundary_scout.handoff.md` with:

- exact app-server files and functions that still own too much policy
- proposed destination crate/module for each policy cluster
- risks for app-server API schema or tests
- commit readiness notes, but do not make commits
