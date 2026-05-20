# Agent Prompt: canary_observer

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only SOLID canary observer.

First read:

- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/prototypes/check-core-boundaries.ps1`

Owned paths: no source files.

Forbidden:

- no edits except `.codex/workflow/agents/canary_observer.handoff.md`
- no Git
- no Cargo or Just builds
- no formatters
- no manifest or lockfile edits

Task:

Run cheap read-only scans for remaining direct/protocol/core leaks and update
`.codex/workflow/agents/canary_observer.handoff.md` with:

- exact command(s) run
- current leak counts grouped by dependency kind
- top files/symbols blocking the boundary
- SOLID risks introduced by any active worker notes

Do not fix code.
