# Agent Prompt: test_surface_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only verification surface scout.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/verification_strategy_scout.handoff.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- recent files under `logs/`

Task:

- Inspect recent release-test and cargo-check logs without starting new builds.
- Map the smallest useful verification lanes for each pending slice, using the
  repo's release-only local build rules.
- Do not edit source files, manifests, lockfiles, generated files, or handoff
  documents other than your own handoff.
- Do not run Cargo, Just, formatters, schema generation, or Git
  staging/commits.
- You may delegate focused read-only questions to helper agents if useful.

Write `.codex/workflow/agents/test_surface_scout.handoff.md` with:

- current observed compile/test blockers by log file
- smallest recommended release verification lane per crate/slice
- lanes that must be deferred until compile blockers are fixed
- commit readiness notes, but do not make commits
