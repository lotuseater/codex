# Prompt-Injection Benchmark on a Multi-File CODING Task (Claude, blind-judged) — 2026-06-18

## Why this run exists
The LONG **creative** benchmark earlier the same day (`codex-injection-benchmark-LONG-2026-06-18.md`,
a fairy-tale animation) showed injection value is real on elaborate work but **task-shaped** — and
notably **python/script-first fell *below* control** on pure generation, because there was no data to
script. That left an open question: does the ranking **replicate on CODING work**, and does
python-first — which *should* match parse/aggregate work — **recover**? This run tests exactly that by
building one real feature into a real game three ways.

## Method
- **Task (identical for all three):** implement an **"Overcharge Meter (Salt Surge)"** into
  **DonutGame** (mature vanilla-JS tower-defense): a new IIFE system module `window.DONUT_OVERCHARGE`
  **plus ~16 anchored edits across 5 existing files** (`game.js`, `game-runtime.js` **578 KB**,
  `hud-ui.js`, `index.html`, `game.css` **614 KB**). A real multi-file integration — read the live
  repo, **reuse the existing combo/kill-chain signal**, wire via ES side-effect import + update-loop
  tick + snapshot read-model + a bilingual HUD chip. Full spec: `artifacts/donut-feature/BRIEF.md`.
- **Subjects:** 3 injection variants × **1 implementer each** (Opus, effort high), run concurrently as
  a background Workflow. Each got its variant's working-style text *prepended* to an identical brief;
  blind-coded scratch dirs `r1/r2/r3`, code→variant key withheld from the judge.
  - **V2 batch-programming** (animation #1)
  - **V1 action-routing** (codex's deployed default; animation #2)
  - **V3 python/script-first** (animation last, *below* control)
- **Output per agent:** the complete module + `CHANGES.md` (anchored `<<<OLD/<<<NEW` blocks copied
  verbatim from the live files) + `NOTES.md`. **No git/npm/playwright** (scratch-dir + anchored-diff
  mechanic; the orchestrator integrates only the winner, applying the OLD/NEW blocks as exact-string
  replacements).
- **Judging:** **1 blind Opus judge** read the actual code, **verified every `<<<OLD` anchor against
  the live repo**, and scored each build 0–10 on
  correctness / completeness / fit / code_quality / player_value (total 0–50).
- **Cost:** 4 agents (3 builders + 1 judge), **491,550 tokens, ~16.7 min** wall.

## Results — blind scores (deanonymized via `key.json`)

| Rank | Variant | Total /50 | Correct | Complete | Fit | Quality | Player | vs routing |
|---|---|--:|--:|--:|--:|--:|--:|--:|
| 1 | **V2 batch-programming** | **44** | 9 | 9 | 9 | 9 | 8 | **+9** |
| 2 | V3 python/script-first | 43 | 9 | 9 | 9 | 8 | 8 | +8 |
| 3 | V1 action-routing | 35 | 6 | 8 | 7 | 7 | 7 | — |

All three wired the feature the **codebase-correct** way (ES import not a `<script>` tag; tick after
the pause/phase guards; charge on the `incrementKills` kill hook; doubled salt via one guarded
multiply on `killSugarDrop`; additive `snapshot.overcharge`; `renderOverchargeChip` mirroring
`renderComboChip`; bilingual `ctx.t`). Every `<<<OLD` anchor in all three matched the live repo
verbatim — separation was about engineering quality and one correctness bug, not wiring.

## What it shows

1. **Batch-programming repeats its win — the robust generalist.** #1 on *both* the creative animation
   (42.5) and this coding task (44). It consolidates the repetitive, deterministic parts of a build
   into one structured pass, which helps procedural scenery *and* systematic anchored edits alike. The
   winner returned a strictly state-driven module (windfall returned, not self-applied), the richest
   automation surface, and reduced-motion CSS. **Confirms keeping the batch slot on** as a codex default.

2. **Python/script-first flips from worst to near-best — exactly the task-shape prediction.** On the
   pure-creative animation it was **last** (34.5, below control) because there was nothing to script.
   Here the task is shot through with **data work** — parse / verify / aggregate ~16 anchored edits
   across two ~600 KB files, and discover an existing signal by searching — so the "script-first for
   extract / parse / aggregate" injection is **on-target**. It jumped to a **near-tie for first (43)**,
   losing to batch only on automation depth + minor polish, *not* correctness. This is the clean
   confirmation of the LONG benchmark's stated caveat (*"it remains correct for data tasks"*) and the
   core thesis of the whole series: **match the injection to the work.**

3. **Action-routing came last here — on a single-sample correctness slip, not a strategy indictment.**
   The routing implementer read the **wrong combo signal** (`comboSnapshot()` → `state.combo`, the
   relic/resource combo that is *null* during pure kill-chains — instead of `killCombo.streak`), so
   its meter degraded to a flat 8/kill, **gutting the feature's signature "fills faster on hot chains"
   mechanic**, and it also overloaded the `E` hero-ability key. That is *one agent making one wrong
   read* (n=1), not evidence routing is weak — routing tied #2 on the animation.

## Honest statistics
**n=1 per variant** here (vs n=2 on the animation). The precise point spread is **not** robust — one
agent's correctness bug (r2) and one's extra automation polish (r1 vs r3) set the ranking. The two
findings that ARE robust, because they are qualitative **and** theory-consistent: **(a) batch won
again**, and **(b) python regime-shifted from last on creative to near-first on data/code work** —
both predicted by "injection value is task-shaped." Those, not the exact 44/43/35, are what to act on.

## Config impact
**No codex config change.** This validates the deployed default (`action = always/routing`,
`batch = always/current`) and the standing decision **not** to add an always-on python codex slot:
python-first is a **data-task specialist** — near-best when the turn carries parse/aggregate work
(code integration), worst when it does not (pure creative) — so it correctly stays a **Claude-side
per-task hook**, not a blanket codex default. The lean two-slot codex setup remains the right shape.

**Addendum - shipped-default bake (2026-06-18):** the live `~/.codex/config.toml` already carried the `always/routing` + `always/current` override, so there was no runtime behaviour change on this machine. To make the validated optimum survive a config reset / fresh install, the compiled struct `Default` for `ActionOptimizationInstructionsConfig` was aligned to it (`mode FirstTurn->Always`, `variant ActionRouteSelection->Routing`; the batch slot already defaulted to `Always`+`Current`), then the release was rebuilt (LowMemRelease, `Finished` clean) and redeployed (`codex --version` stamps the fresh build). Commit `9230eef13b`.

## Deliverable (the useful half)
The winning implementation (**V2 batch-programming**) is integrated into DonutGame as a **real shipped
feature** — the **Overcharge Meter "Salt Surge"**: a per-run `0→100` combat meter that fills on kills
(`8 + min(combo, 12)`), and at **READY** (press **F** or click the HUD chip) grants **+40 Salt
instantly** and a **6.0 s surge that doubles kill-salt**. HUD chip mirrors the combo chip (3 states,
bilingual); frozen outside active play; resets per run; not persisted.
**Smoke suite (orchestrator-run, `npm run test:defense:smoke`): 9 passed, exit 0 — green, identical to
the pre-feature baseline (9/9), so the integration adds no regressions.**

## Reproduce / re-measure
`artifacts/donut-feature/`: `BRIEF.md` (spec), `SCOUT.md` (integration anchors), `key.json` (blind
code→variant), workflow script `…/workflows/scripts/donut-overcharge-buildoff-wf_511516b9-061.js`,
`parse_verdict.py` + `verdict_summary.json` (deanonymized scores), `apply_winner.py` (the exact-string
integration applier). Winner sources under `artifacts/donut-feature/r1/`.
