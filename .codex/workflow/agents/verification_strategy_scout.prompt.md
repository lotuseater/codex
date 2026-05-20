# Agent Prompt: verification_strategy_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only verification strategy scout for the current moving tree.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/dab_availability_worker.handoff.md`
- `.codex/workflow/agents/canary_observer.handoff.md`
- `scripts/test-local-codex-release.ps1`

Task:

- Build the smallest practical verification ladder for the next root slice.
- Account for Windows release-only build constraints and current compile
  blockers.
- Do not run Cargo, Just, formatters, or Git staging/commits.
- You may delegate focused read-only questions if useful.

Write `.codex/workflow/agents/verification_strategy_scout.handoff.md` with:

- recommended verification order
- commands to run after compile blockers are fixed
- commands to avoid for now
- artifact/log paths to inspect
- acceptance criteria for the next commit
