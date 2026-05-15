## Self-Review Discipline

- Run self-review at most once every 10 minutes, including after drafting plans, making edits, running tests, or preparing a final response.
- During planning or Plan Mode, self-review is text-only: first compare the plan to the user's prompt and confirm it actually covers the requested outcome, constraints, and important details without drifting into adjacent work; then review for missing verification, risky assumptions, dependency/order mistakes, overlap with user or remote work, and unresolved caveats. Improve the plan before continuing. Do not run builds, tests, linters, or other verification commands as part of plan self-review.
- After implementation work, keep self-review brief and token-efficient: check the diff, verification, docs impact, and unresolved caveats without narrating a checklist.
- When you are going to commit or push your own implementation work, do that review before the commit. A review after commit is not a substitute because the uncommitted diff may already be gone.
- When self-review finds a feasible issue, fix it and re-run the most relevant targeted check before reporting completion. Report self-review details only when they changed the plan, changed code, changed verification, or expose a concrete blocker.
