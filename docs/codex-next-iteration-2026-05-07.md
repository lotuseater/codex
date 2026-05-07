# Codex Next Iteration Notes

Date: 2026-05-07

This note captures the next useful slice after the current build/deploy pass.
The current iteration moved local builds to one release-only artifact shape,
added safe cleanup for accidental debug/test outputs, and kept the feature
crates outside `codex-core` where practical.

## Build And Test Speed

- Add a release-profile stamp under `codex-rs\target` so `build-local-codex.ps1`
  can detect when the local release profile changed and warn before Cargo
  creates duplicate old/new release artifact generations.
- Add a `bazel` shim or update the Just recipes to use `bazelisk` when only
  Bazelisk is installed, so `just bazel-lock-update` works on this PC.
- Keep the focused feature harness as the default fast lane and extend it only
  with small crate-level checks. Avoid broad `codex-tui` and `codex-core`
  test lanes unless the touched code requires them.
- Consider a small helper that reruns an already-built release test executable
  by filter before invoking Cargo again.

## DAB And GUI Harnesses

- Add a runtime canary that reports which automation path is used for each app:
  app-native harness, native DAB, Wizard DAB fallback, or unsupported.
- Test the canary against GitHub-folder GUI apps, terminal, PowerShell,
  Calculator, Notepad++, Chrome, and Paint after the deployed binary is active.
- Keep mutating GUI tools sequenced; allow only read-only discovery/screenshot
  tools to be considered for caching or parallel use.

## Self-Review

- Keep self-review grounded in compact work evidence: current diff, recent
  commit/show output when available, verification logs, and work notes.
- Enforce cooldown and single-pass behavior so automatic review cannot loop
  indefinitely after it fixes something.
- For plan mode, review only the complete displayed plan after clarifying
  questions have been answered, then suppress a second review of the revised
  plan.

## First Moves And Task Memory

- Track first-moves hit rate from `.first_moves.db` after real sessions and
  use misses to improve routing rather than adding more eager repo scanning.
- Keep prompt/plan reminders bounded: inject task goal and plan summaries only
  at controlled checkpoints, not on every turn before compaction.
- Reserve a small task-memory summary for the user goal, active plan, and open
  constraints, with a maximum size and refresh cadence to avoid cache churn.

## Cache

- Keep repo identity in system-wide cache namespaces using repo folder name plus
  root hash, so common folder names like `src` cannot collide.
- Continue using Wizard cache telemetry to choose new cacheable operations;
  do not whitelist broad shell commands without measured repeatability and a
  clear invalidation story.
- Add a quick status surface that reports app-tools cache, operation-cache
  bridge state, first-moves storage, and DAB provider availability together.
