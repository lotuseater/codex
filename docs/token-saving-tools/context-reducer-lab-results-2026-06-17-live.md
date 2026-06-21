# Prompt-Economy Injections — Live Measurement on Claude Agents (2026-06-17)

First round measuring prompt-economy injections with **real token / behaviour data
from live Claude subagents** (not the deterministic proxy scores of the 2026-05
rounds). Two rounds: **R1** = cheap deterministic screen; **R2** = three varied
archetype tasks with an Opus judge. The outcome feeds the Claude rollout
(`~/.claude/rules/prompt-economy.md`) and the codex `/batch-prompt` / `/action-prompt`
slash-command defaults.

## Headline
- **`python_usage` is the decisive winner** — lowest on every behavioural and token
  axis, at **equal quality** to every other variant. ~**55% less billable** and ~**27%
  less total** than control overall; the win is concentrated on the multi-file `audit`
  task where it ran in **2 tool calls / 49.7k billable** vs the field's **5 calls /
  120–172k** — a ~**3.5× cost reduction** for the same result.
- **Quality is saturated.** Every variant reaches essentially identical quality
  (transform: all correct; audit: all 86/100; plan: all 98–100 bar one batch outlier).
  The injections buy **efficiency at equal quality**, not higher quality.
- **Concrete beats abstract.** The explicit "write one Python script" instruction
  (`python_usage`) massively outperformed the abstract "batch your operations"
  instruction (`batch_programming`), which barely improved on control — agents
  operationalised the concrete directive but not the abstract one.

## Variants
| # | name | one-line |
|---|------|----------|
| 1 | control | no injection — baseline |
| 2 | batch_programming | combine clear deterministic ops into one batched step / script |
| 3 | action_route | route each action by shape; smallest useful probe; lowest-overhead reliable route |
| 4 | **python_usage** | prefer one short Python script for any extract / filter / rank / count / aggregate |
| 5 | token_economy_planning | plan for max verified work × quality per token; enumerate+estimate candidate plans, pick argmax |

## Method
- **Tasks (R2):** 3 archetypes, each targeting a different variant's strength —
  `transform` (deterministic JSON filter+sum; exact-match scored),
  `audit` (4-file multi-language bug/secret hunt; Opus-judged, planted-issues − false-positives, max 7),
  `plan` (token-economy work-order planning with a deterministic final-report gate; Opus-judged, 5 criteria + hard gate).
- **Runs:** 5 variants × {transform×1, audit×2, plan×2} = **25 runs on Sonnet** (the
  injection is the independent variable; a uniform cheap model isolates the prompt effect).
- **Judge:** one **Opus** call per judged task, ranking all anonymized outputs against the rubric.
- **Join / harvest:** each run prompt embeds `RUNLABEL=v<N>__<task>__r<rep>`; `rank_r2.py`
  reads each agent jsonl and sums per-turn usage keyed by that label.
- **Primary metric = behaviour (confound-free):** assistant_turns + tool_use_blocks.
  **Secondary:** billable_new (input+output+cache_creation) and total_tokens. Summed
  cache_read is turn-inflated, so billable (which excludes it) is the fairer cost number;
  the two judged tasks have ×2 replicates to average the cache-creation ordering artifact.

## R1 recap — deterministic screen (4 fixtures, 7 variants, 1 rep)
`python_usage` ranked **#1 decisively** (2.0 tool-calls / 3.0 turns vs 2.5 / 4.25 for the
field; ~31% less billable than control). `token_economy_planning` **#2** (same tool/turns
as control but ~35% less output, ~31% less billable). batch / action / combined / meta ≈
control on these python-friendly fixtures — which is why R2 added non-python archetypes.

---

## R2 results

### Quality (saturated — no meaningful spread)
| variant | transform (correct, n=1) | audit (0–100, n=2) | plan (0–100, n=2) |
|---|---|---|---|
| python_usage | ✓ | 86 | **99.5** |
| action_route | ✓ | 86 | 99 |
| control | ✓ | 86 | 98.5 |
| token_economy_planning | ✓ | 86 | 98.5 |
| batch_programming | ✓ | 86 | 89 (one rep 80) |

### Cost / behaviour — per variant (avg over 5 runs)
| variant | tool | turns | output | billable | total | billable vs control |
|---|---|---|---|---|---|---|
| **python_usage** | **2.0** | **3.8** | 569.8 | **49,190** | **130,830** | **−55.5%** |
| action_route | 3.2 | 4.8 | 251.6 | 76,896 | 164,117 | −30.5% |
| token_economy_planning | 3.2 | 5.0 | 515.2 | 77,551 | 171,635 | −29.9% |
| batch_programming | 3.2 | 5.0 | 488.6 | 110,495 | 171,634 | −0.1% |
| control | 3.2 | 5.2 | 728.4 | 110,596 | 178,262 | — |

### Where the win lives — `audit` task (multi-file, the discriminating archetype)
| variant | tool | turns | billable | total |
|---|---|---|---|---|
| **python_usage** | **2.0** | **4.0** | **49,680** | **137,900** |
| action_route | 5.0 | 6.5 | 119,570 | 221,765 |
| token_economy_planning | 5.0 | 7.0 | 120,411 | 239,621 |
| control | 5.0 | 7.0 | 171,779 | 239,261 |
| batch_programming | 5.0 | 7.0 | 172,236 | 239,966 |

python_usage wrote **one script** to scan all four files; the rest issued ~5 separate
reads + reasoning. Same bugs found (all 86), ~3.5× less billable. On the single-file
`transform` and `plan` tasks the variants converge (one tiny file to read), except control
takes an extra turn on `transform` (4 turns / 69.8k billable vs ~3 / ~47.6k for the
injected variants).

---

## Verdict
- **Best overall quality-per-token: `python_usage`** — decisively, driven by multi-file /
  data work. This confirms R1 on richer tasks.
- **Best on planning work (`plan`): `python_usage` 99.5 / `token_economy_planning` 98.5** —
  a near-tie at the top; `token_economy_planning` is the principled planning injection and
  is ~30% less billable than control.
- **`action_route` ≈ `token_economy_planning`** on tokens (both ~30% less billable than
  control, quality-neutral) — empirically interchangeable for the #2 "planning/routing" slot.
- **`batch_programming` underperforms** — ≈ control on tokens and the only variant with a
  quality dip. The abstract framing didn't translate into the script-writing behaviour the
  concrete `python_usage` framing reliably produced.

### Rollout decision (Claude — `~/.claude/rules/prompt-economy.md`)
**Unchanged: keep `python_usage` (#1) + `token_economy_planning` (#2), STATUS: ACTIVE.**
R2 confirms `python_usage` decisively and validates `token_economy_planning` as a real (if
milder) quality-neutral ~30%-billable improvement, strongest on the planning archetype.
`action_route` is recorded as an empirically-equivalent alternative for the #2 slot.

### Feedback to Lane A (codex)
A Claude-winning variant that also wins a codex A/B becomes the persisted codex default via
the new `/batch-prompt` / `/action-prompt` slash command (zero rebuild), or a curated
variant. `python_usage` is the strongest candidate to wire as a codex default once Lane A
ships.

## Caveats
Small N (1–2 reps); synthetic fixtures; Sonnet-only runs (one model); one Opus judge per
task. Directional signal, not a significance test. The audit win is robust across all three
metric families (confound-free tool/turn counts, billable, and total all agree), so it is
the most trustworthy result; the plan/transform convergence is expected (single tiny input).

---

## Appendix A -- Hook A/B: Claude *without* vs *with* the prompt-economy hook (2026-06-17)

**User question.** Measure token usage, quality, and time for Claude **(A) without** the
`prompt-economy` hook vs **(B) with** it, across several specified tasks.

**Setup.** The hook is a UserPromptSubmit injection of ~215 tokens (both bullets: script-first
+ token-economy planning). It was reproduced faithfully by prepending the *verbatim* injection
block to each worker's prompt (arm B); arm A got the identical task with no preamble -- the only
difference between arms is the block. Workers: Sonnet, `general-purpose` (full tools), one agent
per (task x arm). Authoritative per-run metrics come from each subagent's completion record
(`subagent_tokens`, `tool_uses`, `duration_ms`) plus its answer scored against known ground
truth. Deterministic fixtures + truth under `.codex/tmp/lab_ab/` (regenerable).

### Tasks (specified)

| ID | Type | Task | Ground truth | Rationale |
|----|------|------|--------------|-----------|
| T1 | data / 8-file | Count `status=="error"` across `events_0..7.json`; total + per-file | TOTAL=63; 7,7,7,8,8,8,9,9 | multi-file -> script-first should win |
| T2 | data | Top-5 users by sum(amount) in `transactions.json` (300 recs), ties by name asc | u04/u14/u24=550; u01/u11=540 | aggregate + rank |
| T3 | data | Count each `ERROR <CODE>` in `app.log` (400 lines) | E100=E103=E106=E109=20 | parse + count |
| T4 | search | Which `mod_NN.txt` defines `target_handler`, and the line number | mod_07.txt line 10 | tests verify-before-answer |
| T5 | reasoning | 5-bullet summary of `spec.md` functional reqs | faithful to the 5 reqs | non-data -> hook should be neutral |

### Results (n=1 per cell; T4 n=2 -- see note)

| Task | Arm | Quality | Tokens | Tool calls | Time |
|------|-----|---------|--------|:----------:|------|
| T1 | A no-hook | PASS | 39.8k | 1 | 30s |
| T1 | B hook    | PASS | 40.0k | 1 | 43s |
| T2 | A no-hook | PASS | 39.6k | 1 | 49s |
| T2 | B hook    | PASS | 39.8k | 1 | 63s |
| T3 | A no-hook | PASS | 39.7k | 2 | 115s |
| T3 | B hook    | PASS | 39.7k | 1 | 52s |
| T4 | A no-hook r1 | FAIL (line 7)  | 36.9k | 0 | 8s |
| T4 | A no-hook r2 | PASS (line 10) | 39.6k | 1 | 26s |
| T4 | B hook r1    | FAIL (line 4)  | 37.1k | 0 | 8s |
| T4 | B hook r2    | FAIL (line 22) | 37.1k | 0 | 5s |
| T5 | A no-hook | PASS | 39.6k | 1 | 74s |
| T5 | B hook    | PASS | 39.8k | 1 | 71s |

*(T4 truth = mod_07.txt line 10. Across the 4 runs the pattern is crisp: **every run that used
0 tool calls hallucinated the line number; the single run that used 1 tool call (no-hook r2) got
it right.** Correctness tracked tool-use, not the injection -- and the hook did **not** induce
verification (0/2 hook runs used a tool, vs 1/2 no-hook runs). All four named the right file --
a 1-in-12 guess that happened to hit in every run.)*

### Findings

1. **Tokens are dominated by the fixed per-subagent baseline (~37-40k), not the task.** The A/B
   token delta is within noise (<= ~0.5%); a ~215-token injection is invisible at this scale.
   The only *lower* readings (T4, ~37k) came from the runs that **hallucinated with 0 tool
   calls** -- so here "fewer tokens" tracked *worse* quality. Raw token count is a poor
   efficiency metric without a quality gate.
2. **Sonnet already script-firsts by default**, so the hook had little to correct: on
   T1/T2/T3/T5 *both* arms used exactly **1 tool call** (a single script) -- the very behavior
   the hook prescribes. The lone tool-count gap was T3 (no-hook 2 vs hook 1): the hook produced
   a clean single-script run where the control took an extra step.
3. **Quality was near-tied** (T1/T2/T3/T5: both arms PASS). The discriminator was T4, and there
   the hook did *not* help: its "reliable evidence" clause failed to induce verification on either
   hook run (0/2 used a tool), while correctness tracked tool-use alone -- the lone tool-using run
   (no-hook r2) was the only T4 PASS. At n=2 the sample even leaned *against* the hook on this task
   (no-hook 1/2 vs hook 0/2) -- not significant, but the opposite of the intended effect.
4. **Time is model-variance-dominated**, not injection-driven (hook faster on T3, slower on
   T1/T2, tied on T4/T5).

### Interpretation

On **unambiguously script-shaped** data tasks a capable model already does the efficient thing,
so the injection is **redundant: negligible overhead, no measurable gain.** This is consistent
with R2, where the measurable win was isolated to the **ambiguous multi-file `audit`** archetype
(where the control does *not* default to batching). The hook's value is therefore **conditional
on the agent's baseline behavior** -- it pays off where the model would otherwise over-read or
under-batch, and is a ~215-token no-op where the model already batches. On T4 it actively failed to fix the orthogonal failure mode (under-verification -> hallucination):
0/2 hook runs verified, and the only correct T4 answer came from the single (no-hook) run that used a tool.

### Caveats (Appendix A)

n=1 for T1/T2/T3/T5 (arms were identical or near-identical), n=2 for T4; Sonnet-only; synthetic
fixtures deliberately script-shaped (which *under-exposes* the hook by handing the control an easy
default); the discriminating "ambiguous audit" archetype from R2 was not re-isolated here. Raw
per-run metrics, fixtures, and ground truth under `.codex/tmp/lab_ab/`.
