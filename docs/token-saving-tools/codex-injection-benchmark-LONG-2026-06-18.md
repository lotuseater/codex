# Prompt-Injection Benchmark on LONG Creative Tasks (Claude, blind-judged) — 2026-06-18

## Why this run exists
The earlier benchmark (`codex-injection-benchmark-2026-06-17.md`) used sub-second deterministic
fixtures. On those, **quality was saturated** — every variant produced a correct answer, so the
only measurable difference was token count, and the variants were statistically
indistinguishable on quality. Its own recommendation was to *"reserve the injections for the
workload where they could actually move the needle: tasks complex enough that the model would
otherwise over-call tools or under-plan."*

This run is that workload. The task is a real, elaborate creative+technical build — one that
takes an agent ~15 min and ~65k tokens — so quality has room to vary and the injections can
actually help or hurt.

## Method
- **Task (identical for all entries):** build one self-contained `fairytale.html` — pure
  HTML/CSS/vanilla-JS, no network/CDN/build — a genuinely funny 4–6 scene fairy-tale animation
  on a data-driven timeline, 3+ animated characters, a real punchline. (Full spec:
  `artifacts/anim-bench/BRIEF.md`.) The deliverable is itself useful — these are watchable.
- **Subjects:** 6 injection variants × 2 replicates = **12 Opus creator agents** (effort high),
  run concurrently as a background Workflow. Each got its variant's working-style text
  *prepended* to an otherwise identical prompt; **V0 control got none.**
- **Judging:** **1 blind Opus judge** read all 12 finished HTML files from disk and scored each
  0–10 on humor / narrative / animation / polish / completeness (total 0–50). The
  code→variant key was **withheld** from the judge — it scored artifacts, not labels.
- **Variants:** V0 control; V1 action-routing (codex's deployed action default, verbatim);
  V2 batch-programming (Claude-faithful rewrite of the codex batch block); V3 python/script-first
  (the live prompt-economy hook text, verbatim); V4 token-economy planning (live hook text,
  verbatim); V5 combined (all four).
- **Cost:** 13 agents, 792,749 tokens, ~15 min wall.

## Results — by variant (mean quality, n=2)

| Rank | Variant | Quality /50 | Humor | Narrative | Animation | Polish | Complete | vs control |
|---|---|--:|--:|--:|--:|--:|--:|--:|
| 1 | **V2 batch-programming** | **42.5** | 8.5 | 8.5 | 7.5 | 9.0 | 9.0 | **+6.5** |
| 2 | V1 action-routing *(codex default)* | 40.0 | 7.5 | 8.0 | 8.0 | 8.0 | 8.5 | +4.0 |
| 2 | V4 token-economy planning | 40.0 | 8.5 | 8.0 | 8.0 | 7.5 | 8.0 | +4.0 |
| 4 | V0 control (no injection) | 36.0 | 7.5 | 6.5 | 7.0 | 7.0 | 8.0 | — |
| 5 | V5 combined (all four) | 35.0 | 7.0 | 7.0 | 6.5 | 7.0 | 7.5 | −1.0 |
| 6 | V3 python/script-first | 34.5 | 7.0 | 6.5 | 7.0 | 6.5 | 7.5 | −1.5 |

Per-run detail (blind scores + code metrics) is in `artifacts/anim-bench/results_table.md`;
raw judge output in `artifacts/anim-bench/judge.json`. Every one of the 12 was verified
**fully self-contained** (`ext_ref=0` for all) — separation was about craft, not correctness.

## What it shows

1. **On long creative work the injections DO move quality — unlike on short fixtures.** The
   three "plan/structure" injections (batch, routing, planning) beat the no-injection control
   by **+4 to +6.5 points**. This is the discriminating signal the fixture benchmark lacked, and
   it confirms the injections are designed for exactly this regime.

2. **An off-target injection actively HURTS.** V3 python/script-first scored *below* control
   (−1.5) and was the worst variant. For a pure-generation task with no data to extract/parse,
   "prefer a Python script for data work" is irrelevant guidance that distracts rather than
   helps. The injection has to match the work.

3. **Stacking everything is worse than one good injection.** V5 combined (all four at once)
   also fell *below* control (−1.0) — the four competing directives dilute focus, and dragging
   the counterproductive python directive into the mix pulls the whole thing down. **More
   injection text is not better;** one well-matched nudge beats a kitchen sink.

4. **Why batch-programming won here:** the brief explicitly rewards *generating repetitive
   visual elements programmatically* (stars, scales, crowds) and a *data-driven timeline* —
   so "batch the deterministic, repetitive work into one consolidated construction" is unusually
   well-aligned with this task. The winner (s02, 648 lines) had the heaviest procedural scenery
   and the most structured build. The win is real but partly task-shaped; routing and planning
   are the more *general-purpose* performers.

### Honest caveat on statistics
n=2 per variant, and within-variant spread is real (e.g. V1: 43 and 37). So the **top three
(40–42.5) are statistically indistinguishable from each other** — do not over-read batch's #1.
The robust, repeated signals are the two *gaps*: top-cluster (plan/structure injections, ~40–42)
clearly above control (36), and the two python-containing variants (V3, V5) clearly *below*
control. Those two conclusions, not the exact ranking, are what to act on.

## Reconsidered codex config decision
Current deployed default (`~/.codex/config.toml`): action = `always`/`routing`,
batch = `always`/`current`. This run **validates that default and argues against expanding it**:

- **Keep action = routing.** V1 (the exact deployed action text) beat control by +4 on elaborate
  work — its value shows up precisely on the long, multi-step tasks the user actually runs, which
  the short benchmark could not see. ✔ Confirmed.
- **Keep the batch block on.** V2 (mirror of the codex batch block) was the single best variant
  (+6.5). The batch recipe earns its place on long builds with repetitive structure. ✔ Confirmed.
- **Do NOT add a python/script-first slot as a codex default.** It underperforms control on
  non-data work and would silently tax every creative/implementation turn. (It remains correct
  for *data* tasks — it's why the Claude-side hook keeps it — but as an always-on codex default
  it's a net negative.)
- **Do NOT stack all injections.** The combined variant was below control; piling routing + batch
  + python + planning into one prompt is counterproductive. The lean two-slot setup codex ships
  is the right shape — resist adding more.
- **Token-economy planning (V4, +4) is a legitimate third option** and is already live in the
  Claude prompt-economy hook; no codex change needed.

**Net:** no config change. The 2026-06-17 tuning (action always/routing + batch always/current)
is the correct, quality-validated default for the user's long tasks; the actionable new finding
is a *negative* one — don't broaden it with python-first or a combined kitchen-sink.

## Deliverables (the "useful" half)
- **Watch them all:** `artifacts/anim-bench/gallery.html` — a ranked gallery ("The Drowsy Dragon
  Film Festival"); click any card to play that animation offline.
- **The winner:** `artifacts/anim-bench/WINNER_fairytale.html` (= run s02 /
  `V2_batch_programming`, *"The Knight Who Cried Dragon"* — "Mate, it's 3am.
  She's not even here.").
- Per-run sources: `artifacts/anim-bench/runs/s00..s11/fairytale.html`.

## Reproduce / re-measure
`artifacts/anim-bench/`: `BRIEF.md` (task), `key.json` (blind code→variant), workflow script
`…/workflows/scripts/anim-injection-bench-wf_1efd9b20-647.js`, `parse_results.py` (extract
judge.json + creators.json from the workflow output), `measure.py` (join blind scores to key +
HTML metrics → `results_table.md`), `gen_gallery.py` (build the gallery).
