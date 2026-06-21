# Codex prompt-injection variant benchmark (deployed build, 2026-06-17)

**What:** measure the deployed local `codex` build's behaviour (tokens, tool-calls,
wall-clock, task correctness) under different runtime prompt-injection settings — the
two new swappable slots `action_optimization_instructions` and
`batch_mini_programming_instructions` — comparing **empty (no injection)**, **the best**,
and **all the other** variants across several deterministic tasks.

This is the live-on-codex counterpart to the Claude-side A/B in
`context-reducer-lab-results-2026-06-17-live.md` (Appendix A). There the question was
"does the prompt-economy hook change Claude's behaviour"; here it is "does each codex
injection variant change the *deployed model's* behaviour, and at what token/time cost".

- Binary under test: `codex-rs/target/release/codex.exe` (the deployed build, stamped
  2026-06-17T20:04:51; verified operational this session).
- Driver: `codex exec --json -o <answer> --dangerously-bypass-approvals-and-sandbox
  --skip-git-repo-check -C <data> --ephemeral -c <slot>.mode=always -c <slot>.variant=<v>`.
- Harness: `.codex/tmp/lab_ab/run_matrix.ps1` (runner) + `score_matrix.py` (token
  extraction + scoring vs `.codex/tmp/lab_ab/truth.json`).

> **TL;DR:** All 8 variants (empty, the deployed "best", and the 6 others) solved all 4 tasks correctly with **exactly 1 shell command each** -- the deployed model already script-firsts, so the injections changed nothing that matters. The only measurable effect was on **reasoning tokens**: verbose/aggressive blocks inflated reasoning **+42-44%** for no gain, while terse blocks (routing, batch-current) matched or beat empty. The deployed default (action `route_selection`) is a **~0% no-op** vs empty. Net: on this task shape injections are neutral-to-wasteful; any value lives in harder, multi-step workloads not covered here.

---

## Variants tested

Each slot's injected block is a developer-prompt section; `mode=always` forces it on,
`mode=off` omits it entirely. `body()` returns the variant literal (or `custom_text` if set).

| ID | Slot / setting | Injected block size | Intent of the text |
|----|----------------|--------------------:|--------------------|
| **V0** | both slots `off` | 0 chars | **empty baseline** — no injection |
| **V1** | action = `action_route_selection` | ~247 ch (~62 tok) | **the "best" action variant** (deployed default): answer directly when evidence suffices, else one focused command to decide the branch, batch deterministic work |
| V2 | action = `verbose` | ~507 ch (~127 tok) | longer "keep simple tasks simple" framing |
| V3 | action = `routing` | ~419 ch (~105 tok) | route each next action by shape (direct / targeted / batch) |
| V4 | batch = `current` | ~1456 ch (~364 tok) | default long `workflow_batch` guidance |
| V5 | batch = `aggressive` | ~345 ch (~86 tok) | "maximize completed local work per tool call by batching" |
| V6 | batch = `compact` | ~673 ch (~168 tok) | compact root-confined deterministic batching |
| V7 | action `route_selection` + batch `current` | ~1703 ch (~426 tok) | **combined** best-action + default-batch |

(Variant block sizes confirmed earlier this session via `codex debug prompt-input`; all
render distinct, and `custom_text` overrides either slot.)

## Tasks (deterministic, exact-match scored)

| ID | Task | Truth | Shape it rewards |
|----|------|-------|------------------|
| T1 | count `status=="error"` across `events_0..7.json` | TOTAL=63 | **multi-file** → batching |
| T2 | top-5 users by Σamount in `transactions.json` (ties by id asc) | u04/u14/u24=550, u01/u11=540 | **aggregate / script** |
| T3 | count each `ERROR E<NNN>` in `app.log` | E100=E103=E106=E109=20 | single-file scan / script |
| T4 | find file+line of `def target_handler(` in `mod_00..11.txt` | mod_07.txt:10 | **single decision** → answer-directly / one grep |

## Methodology & metrics

- One run per (variant, task) cell — **n=1** (deterministic scoring is robust at n=1;
  token/time carry single-sample noise, flagged below). 8 × 4 = 32 runs, serial.
- Per run, parsed from the `--json` event stream:
  - **`output_tokens`**, **`reasoning_output_tokens`** — summed over all `turn.completed`
    events. *Cleanest behavioural signal* (more/less reasoning + tool-call generation).
  - **`input_tokens`**, **`cached_input_tokens`** — total prompt billed / cached portion.
  - **`cmds`** — count of `command_execution` items that completed (how many shell
    commands the model ran — the batching/action-economy signal).
  - **`turns`**, **wall-clock seconds**, and the **final answer** (scored vs truth).

### Caveats (read before trusting deltas)

1. **Input is dominated by the codex base prompt (~95k tok/run).** The injection block is
   ~60–430 tokens — well under 0.5% of input — so `input_tokens` deltas between variants
   are mostly **prompt-cache noise**, not the injection. Lean on `output_tokens`,
   `reasoning_output_tokens`, and `cmds` for behavioural signal.
2. **Cache confound:** changing the injected block changes the prompt prefix, so
   `cached_input_tokens` differs across variants for reasons unrelated to task work.
3. **n=1:** a single model sample per cell. Treat per-cell token/time as indicative;
   the per-variant *means across 4 tasks* are steadier. Correctness (pass/fail) is exact.
4. **Sandbox bypass:** the local build ships no Windows sandbox helper
   (`codex-windows-sandbox-setup.exe` absent → `-s read-only` makes every command fail),
   so runs use `--dangerously-bypass-approvals-and-sandbox`. Safe here (benign reads of
   local fixture files); noted for reproducibility.

---

## RESULTS

### Per-run results (tag=full, n=32)

| Variant | Task | Pass | out_tok | reason_tok | in_tok | cached_in | cmds | turns | sec | answer |
|---|---|:--:|--:|--:|--:|--:|--:|--:|--:|---|
| V0 | T1 | OK | 284 | 155 | 65535 | 29440 | 1 | 1 | 78 | TOTAL=63 |
| V1 | T1 | OK | 373 | 244 | 65696 | 29440 | 1 | 1 | 67 | TOTAL=63 |
| V2 | T1 | OK | 943 | 593 | 93426 | 8704 | 1 | 1 | 72 | TOTAL=63 |
| V3 | T1 | OK | 335 | 200 | 65741 | 29440 | 1 | 1 | 46 | TOTAL=63 |
| V4 | T1 | OK | 379 | 244 | 66195 | 46336 | 1 | 1 | 44 | TOTAL=63 |
| V5 | T1 | OK | 468 | 333 | 65833 | 4864 | 1 | 1 | 46 | TOTAL=63 |
| V6 | T1 | OK | 503 | 366 | 66031 | 29440 | 1 | 1 | 43 | TOTAL=63 |
| V7 | T1 | OK | 409 | 280 | 66332 | 29440 | 1 | 1 | 39 | TOTAL=63 |
| V0 | T2 | OK | 679 | 406 | 65962 | 29440 | 1 | 1 | 49 | TOP5=u04=550,u14=550,u24=550,u01=540,u11=540 |
| V1 | T2 | OK | 802 | 516 | 66192 | 29440 | 1 | 1 | 43 | TOP5=u04=550,u14=550,u24=550,u01=540,u11=540 |
| V2 | T2 | OK | 768 | 516 | 66247 | 29440 | 1 | 1 | 60 | TOP5=u04=550,u14=550,u24=550,u01=540,u11=540 |
| V3 | T2 | OK | 790 | 516 | 66233 | 46336 | 1 | 1 | 51 | TOP5=u04=550,u14=550,u24=550,u01=540,u11=540 |
| V4 | T2 | OK | 576 | 323 | 66467 | 46336 | 1 | 1 | 61 | TOP5=u04=550,u14=550,u24=550,u01=540,u11=540 |
| V5 | T2 | OK | 785 | 516 | 66202 | 21760 | 1 | 1 | 45 | TOP5=u04=550,u14=550,u24=550,u01=540,u11=540 |
| V6 | T2 | OK | 761 | 516 | 66336 | 29440 | 1 | 1 | 60 | TOP5=u04=550,u14=550,u24=550,u01=540,u11=540 |
| V7 | T2 | OK | 476 | 245 | 66451 | 29440 | 1 | 1 | 60 | TOP5=u04=550,u14=550,u24=550,u01=540,u11=540 |
| V0 | T3 | OK | 464 | 306 | 76364 | 46336 | 1 | 1 | 51 | CODES=E100=20,E103=20,E106=20,E109=20 |
| V1 | T3 | OK | 271 | 104 | 76251 | 4864 | 1 | 1 | 41 | CODES=E100=20,E103=20,E106=20,E109=20 |
| V2 | T3 | OK | 360 | 199 | 76444 | 29440 | 1 | 1 | 46 | CODES=E100=20,E103=20,E106=20,E109=20 |
| V3 | T3 | OK | 324 | 98 | 76381 | 46336 | 1 | 1 | 46 | CODES=E100=20,E103=20,E106=20,E109=20 |
| V4 | T3 | OK | 287 | 139 | 76777 | 46336 | 1 | 1 | 54 | CODES=E100=20,E103=20,E106=20,E109=20 |
| V5 | T3 | OK | 578 | 427 | 76609 | 29440 | 1 | 1 | 49 | CODES=E100=20,E103=20,E106=20,E109=20 |
| V6 | T3 | OK | 272 | 123 | 76461 | 29440 | 1 | 1 | 60 | CODES=E100=20,E103=20,E106=20,E109=20 |
| V7 | T3 | OK | 625 | 449 | 77207 | 29440 | 1 | 1 | 43 | CODES=E100=20,E103=20,E106=20,E109=20 |
| V0 | T4 | OK | 381 | 225 | 95015 | 57984 | 1 | 1 | 46 | FILE=mod_07.txt LINE=10 |
| V1 | T4 | OK | 378 | 222 | 95124 | 33408 | 1 | 1 | 45 | FILE=mod_07.txt LINE=10 |
| V2 | T4 | OK | 398 | 243 | 95350 | 48768 | 1 | 1 | 45 | FILE=mod_07.txt LINE=10 |
| V3 | T4 | OK | 349 | 193 | 95159 | 50304 | 1 | 1 | 40 | FILE=mod_07.txt LINE=10 |
| V4 | T4 | OK | 317 | 162 | 95777 | 50304 | 1 | 1 | 52 | FILE=mod_07.txt LINE=10 |
| V5 | T4 | OK | 451 | 295 | 95371 | 31872 | 1 | 1 | 87 | FILE=mod_07.txt LINE=10 |
| V6 | T4 | OK | 410 | 254 | 95478 | 74880 | 1 | 1 | 67 | FILE=mod_07.txt LINE=10 |
| V7 | T4 | OK | 487 | 332 | 96261 | 65664 | 1 | 1 | 65 | FILE=mod_07.txt LINE=10 |

### Per-variant aggregate (mean across tasks)

| Variant | label | pass/N | mean out_tok | mean reason | mean cmds | mean billable | mean sec |
|---|---|:--:|--:|--:|--:|--:|--:|
| V0 | empty (both off) | 4/4 | 452 | 273 | 1.00 | 35371 | 56.2 |
| V1 | action: route_selection (best) | 4/4 | 456 | 272 | 1.00 | 51984 | 48.8 |
| V2 | action: verbose | 4/4 | 617 | 388 | 1.00 | 54396 | 55.7 |
| V3 | action: routing | 4/4 | 450 | 252 | 1.00 | 33224 | 46.0 |
| V4 | batch: current | 4/4 | 390 | 217 | 1.00 | 29366 | 52.9 |
| V5 | batch: aggressive | 4/4 | 570 | 393 | 1.00 | 54590 | 56.6 |
| V6 | batch: compact | 4/4 | 486 | 315 | 1.00 | 35763 | 57.4 |
| V7 | combined (route+batch.current) | 4/4 | 499 | 326 | 1.00 | 38566 | 51.5 |

### Pass matrix (variant x task)

| Variant | T1 | T2 | T3 | T4 |
|---|:--:|:--:|:--:|:--:|
| V0 | OK | OK | OK | OK |
| V1 | OK | OK | OK | OK |
| V2 | OK | OK | OK | OK |
| V3 | OK | OK | OK | OK |
| V4 | OK | OK | OK | OK |
| V5 | OK | OK | OK | OK |
| V6 | OK | OK | OK | OK |
| V7 | OK | OK | OK | OK |

## Findings

**1. Quality is saturated — 32/32 correct.** Every variant solved every task. No
injection (empty included) changed correctness on these deterministic fixtures; the
deployed model handles them all. So this benchmark measures *efficiency at equal
quality*, not a quality lift — exactly as the Claude-side hook A/B found.

**2. Tool-use is identical: exactly one shell command on all 32 runs.** The deployed
model already "script-firsts" — it writes a single command/script per task, whether a
1-file find (T4), a 400-line scan (T3), or an 8-file aggregate (T1). The batching /
action-routing injections are designed to *induce* exactly this, so on a model that
already does it they have nothing to improve. (Same ceiling the Claude A/B hit: the win
is conditional on the agent's baseline NOT already being optimal.)

**3. The only measurable effect is on reasoning/output tokens — and "more words" hurts.**
With tool-count and correctness pinned, the injected text only moved how much the model
*thinks*:

| Variant | mean reason_tok | vs empty |
|---|--:|--:|
| V4 batch: current | 217 | **-21%** |
| V3 action: routing | 252 | -8% |
| V1 action: route_selection (deployed default) | 272 | ~0% |
| **V0 empty** | **273** | baseline |
| V6 batch: compact | 315 | +15% |
| V7 combined (route + batch.current) | 326 | +19% |
| V2 action: verbose | 388 | **+42%** |
| V5 batch: aggressive | 393 | **+44%** |

The verbose (V2) and aggressive (V5) blocks inflated reasoning ~40-45% over empty for
**zero** quality or tool-count gain. The terse/structured blocks (routing, batch-current)
sat at or below empty.

**4. The deployed default (V1 = action `route_selection`) is a near-perfect no-op here.**
456 out / 272 reason vs empty's 452 / 273 — statistically indistinguishable. It costs
~62 prompt tokens to inject and neither helps nor hurts on tasks of this shape. Safe to
leave on; no measured benefit on this workload.

**5. Lowest-cost-at-full-quality: V4 (batch: current).** Counterintuitively the *longest*
injected block produced the *least* model output (390 out / 217 reason, below empty). A
plausible read: a concrete "write one `workflow_batch`" recipe shortens deliberation.
n=1, so treat as indicative, not proven.

**6. Ignore `input_tokens` / `billable` deltas — they are prompt-cache variance.** Input
is ~65-96k/run (codex base prompt); the injection is <0.5% of it. Changing the block
shifts the cached prefix, so `cached_input_tokens` swings 5k-75k for reasons unrelated to
the task (e.g. V1/T3 cached only 4,864 vs V0/T4's 57,984). The `mean billable` column
reflects that cache noise, not the injection — which is why output_tokens, reasoning, and
`cmds` are the signals to read.

### Recommendation

On small, deterministic, already-script-friendly work, codex prompt injections are
**net-neutral-to-negative**: no correctness change, no tool-count change, and the
verbose/aggressive variants only burn reasoning tokens. Keep the terse defaults (action
`route_selection`, batch `current`/`compact`) or run **empty** — and reserve the
injections for the workload where they could actually move the needle: tasks complex
enough that the model would *otherwise* over-call tools or under-plan (multi-step,
ambiguous, many-file refactors), which these four fixtures are not. A follow-up on that
harder task shape (where baseline tool-count > 1) is the experiment that would actually
discriminate the variants. The infrastructure now makes that a zero-rebuild config flip.

