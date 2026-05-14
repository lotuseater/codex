# Limited Prolog/ProbLog Borrowing Plan For Codex

## Summary

Codex should borrow the Wizard/Cognos logic pattern for inner first-moves, but
only as a bounded ranking and diagnostics layer. The first implementation slice
is Rust-native inside `codex-rs/first-moves`: deterministic path gates plus
ProbLog-shaped evidence scoring over facts Codex already has.

This is essential enough to implement because first-moves quality directly
affects how quickly Codex reads the right files. It is also limited by design:
normal prediction must not require SWI-Prolog, Python, or `problog`.

## Borrowed Pattern

From Wizard_Erasmus:

- use a logic overlay for first-move ranking, not broad agent cognition
- keep the layer fail-open
- surface evidence through source labels and reasons
- keep shadow comparison records so quality can be measured
- make external live logic optional, bounded, and later-stage only

From Cognos:

- use deterministic gates where facts are explicit
- use probabilistic evidence only where uncertainty is bounded
- keep hard policy/runtime decisions separate from advisory scoring

From Codex's current state:

- keep `codex-rs/reasoning-logic` isolated as optional comparison/eval support
- implement the first production slice in `codex-rs/first-moves`
- avoid `codex-core` expansion for this change

## Implemented First Slice

The first slice adds a private first-moves logic/evidence layer:

- `codex-rs/first-moves/src/logic.rs` evaluates candidate path facts.
- Deterministic gates handle explicit path facts, implementation-docs mismatch,
  and noisy auxiliary artifacts.
- Probabilistic evidence uses bounded Rust scoring shaped like ProbLog
  independent support/risk signals.
- `predict.rs` applies the logic delta after existing lexical, intent, memory,
  and learning signals.
- `source_layer` can report `logic_gate` or `probabilistic_evidence` when logic
  materially influenced the ranking.
- `shadow.rs` records a `logic_evidence` variant alongside existing shadow
  variants.

The layer remains fail-open. If there are no useful facts, the score delta is
zero. If a candidate is explicitly mentioned, explicit routing still wins.

## Phased Plan

Phase 1: Rust-native first-moves logic.

- Keep existing lexical/intent/history scoring as the baseline.
- Add limited deterministic and probabilistic deltas.
- Add focused tests proving source files beat shallow docs for implementation
  prompts.
- Record shadow comparison output.

Phase 2: Evaluate shadow records.

- Compare `native_paths` against the `logic_evidence` variant.
- Look for real hit-rate improvement, not only plausible examples.
- Track regressions where logic de-prioritizes docs or tests that the user
  actually needed.

Phase 3: Harden optional comparison engines.

- Keep SWI-Prolog and ProbLog inside `codex-rs/reasoning-logic`.
- Use them for fixtures and offline comparison only.
- Keep tests skip-safe when `swipl`, Python, or `problog` is unavailable.

Phase 4: Promote narrow diagnostics only if evidence supports it.

- Candidate diagnostics: why a file was boosted or de-prioritized.
- Routing diagnostics: which structured facts drove first-move selection.
- Future comparison cases: `ReasoningComparisonCase`, `PlanGateCase`, or a
  `just reasoning-logic-compare` developer workflow.

Phase 5: Reject broad adoption unless separately proven.

- No always-on Prolog for normal coding turns.
- No ProbLog for vague model intuition.
- No runtime policy enforcement from first-moves.
- No `codex-core` expansion without a concrete proven API.

## Acceptance Criteria

The first slice is acceptable only if:

- existing first-moves tests pass
- at least one fixture demonstrates a real routing improvement
- normal prediction has no external logic runtime dependency
- shadow telemetry includes the logic variant
- docs clearly state the limited borrowing boundary
- unrelated dirty worktree changes are not reverted or reformatted

## Verification Plan

Use the release-only local lane in this checkout:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-first-moves
```

Then run formatting and static checks:

```powershell
just fmt
just fix -p codex-first-moves
git diff --check
```

Optional comparison checks can be run later if SWI-Prolog and ProbLog are needed
for eval work:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-reasoning-logic
```

That optional lane is not required for normal first-moves behavior because this
slice deliberately avoids external engine dependencies.
