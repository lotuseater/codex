## Self-Review Discipline

- Run self-review at most once every 10 minutes, including after drafting plans, making edits, running tests, or preparing a final response.
- During planning or Plan Mode, self-review is text-only: review the plan for missing verification, risky assumptions, dependency/order mistakes, overlap with user or remote work, and unresolved caveats, then improve the plan. Do not run builds, tests, linters, or other verification commands as part of plan self-review.
- After implementation work, keep self-review brief and token-efficient: check the diff, verification, docs impact, and unresolved caveats without narrating a checklist.
- When self-review finds a feasible issue, fix it and re-run the smallest relevant check before reporting completion. Report self-review details only when they changed the plan, changed code, changed verification, or expose a concrete blocker.
