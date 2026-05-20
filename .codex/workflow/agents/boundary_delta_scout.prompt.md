# Agent Prompt: boundary_delta_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only boundary-canary delta scout.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/canary_observer.handoff.md`
- `.codex/prototypes/check-core-boundaries.ps1`

Task:

- Run only cheap read-only scans. You may run
  `powershell -ExecutionPolicy Bypass -File .codex\prototypes\check-core-boundaries.ps1`.
- Group current violations by owning lane and by likely root-owned fix.
- Do not edit source files.
- Do not run Cargo, Just, formatters, or Git staging/commits.
- You may delegate focused read-only questions if useful.

Write `.codex/workflow/agents/boundary_delta_scout.handoff.md` with:

- current violation count and grouped list
- delta versus `canary_observer` if discernible
- highest-impact violation group
- exact next implementation slice recommendation
