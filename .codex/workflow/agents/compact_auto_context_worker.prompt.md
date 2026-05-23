# Compact Auto-Context Worker

You are continuing the stale compact/context-reduction task in this Codex checkout.

Root is handling the repeated-read bug and context-access guard. Do not duplicate that root investigation unless you need a narrow integration fact for the compact task.

## Goals

- Rehydrate from `.codex/workflow/solid-refactor-handoff.md` and nearby compact/context-reduction workflow notes using the smallest useful reads.
- Identify the current unfinished compact/auto-context-reduction slice and move one coherent repo-local piece forward.
- Prefer code, tests, scripts, or handoff improvements that make compact/context reduction easier to resume and verify.
- Keep `$env:CODEX_WORKER_HANDOFF` updated with changed files, exact verification, and any blocker.

## Constraints

- Do not run a broad Codex build or broad Cargo workspace test.
- Use targeted symbol searches, `workflow_batch`, `multi_read`, or scoped search-style commands instead of repeated overlapping reads.
- If a whole file is needed, read it once through a batch/script path and summarize the needed facts in the handoff.
- Do not revert or overwrite root/user edits. You are not alone in this codebase.
- Stop after a coherent verified slice or a concise blocker handoff.

## Verification

Run only targeted canaries or narrow package/test filters that directly match the slice you changed. If verification is not possible without a broad build, write the exact narrow command you would run and why it is currently blocked.
