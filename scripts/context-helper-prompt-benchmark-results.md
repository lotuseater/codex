# Context Helper Prompt Variant Benchmark Results

This is the tracked, redacted summary for the prompt-variant benchmark run saved in the local ignored artifact directory:

`logs/context-helper-prompt-benchmarks/full-20-30-s6-v4`

The ignored artifact directory contains the raw transcript excerpts, reducer prompts, reducer outputs, judge outputs, and JSONL rows. Those raw artifacts are intentionally not tracked because they contain real local Codex conversation excerpts.

## Run Configuration

- Run status: preflight ok
- Thresholds: 20%, 30%
- Cooldown: 24 turns
- Samples per threshold: 6
- Context token budget: 8,000
- Total samples: 12
- Reducer rows: 36
- Judge rows: 12
- Sessions loaded: 80
- Session files scanned: 150
- Benchmark-generated sessions skipped: 70
- Token events scanned: 19,409
- Trigger candidates: 1,216
- Selected samples: 6 at 20%, 6 at 30%

## Prompt Variants

- `no_nudge`: empty reducer instruction used as a control in the targeted rerun
- `standard_compaction_template`: archived pre-prune production checkpoint template used as a legacy control
- `prune`: user's direct pruning prompt
- `delta`: merge prior reduced context with new context delta
- `evidence`: preserve implementation evidence, commands, outputs, constraints, and uncertainty

The three custom variants in the final v4 run received the same canonical reducer input: a deterministic prior summary plus the raw new context delta. Deterministic scoring and LLM judging used that same canonical input. Judge labels were blinded and balanced across A/B/C so each custom variant appeared under each label exactly four times across the 12 judgments. The current benchmark script also supports the `no_nudge` and `standard_compaction_template` baselines; the headline metrics below only include rows present in the final v4 run.

## Headline Metrics

| variant | ok rows | avg compression | avg path retain | avg command retain | avg constraint retain | avg noise markers |
|---|---:|---:|---:|---:|---:|---:|
| prune | 12 | 0.232 | 0.577 | 0.098 | 0.000 | 0.083 |
| delta | 12 | 0.209 | 0.543 | 0.061 | 0.025 | 0.000 |
| evidence | 12 | 0.308 | 0.644 | 0.194 | 0.028 | 0.083 |

## Judge Best Counts

| variant | best-count |
|---|---:|
| prune | 5 |
| delta | 2 |
| evidence | 5 |

## Interpretation

The evidence-preserving prompt retained the most paths and commands in this small run. The blinded judge split was tied between `prune` and `evidence` at 5 of 12 wins each, with `delta` winning 2 of 12. Evidence produced larger summaries, so it is not a pure token-minimization winner.

The result should be treated as directional rather than final. It has only 12 judge rows and one LLM judgment per sample.

## Targeted No-Nudge Control

Additional targeted control run:
`logs/context-helper-prompt-benchmarks/targeted-nonudge-control-v1`

Configuration:

- Thresholds: 20%, 30%
- Selected samples: 3 per threshold, 6 total
- Variants: `no_nudge`, `prune`, `delta`, `evidence`
- Reducer rows: 24
- Judge rows: 6

| variant | ok rows | avg compression | avg path retain | avg command retain | avg constraint retain | avg noise markers | judge best-count |
|---|---:|---:|---:|---:|---:|---:|---:|
| no_nudge | 6 | 0.073 | 0.069 | 0.022 | 0.000 | 0.000 | 0 |
| prune | 6 | 0.202 | 0.457 | 0.136 | 0.000 | 0.000 | 0 |
| delta | 6 | 0.186 | 0.469 | 0.033 | 0.000 | 0.000 | 0 |
| evidence | 6 | 0.304 | 0.533 | 0.243 | 0.000 | 0.000 | 6 |

The no-nudge control was not competitive on this targeted slice. It compressed aggressively but lost nearly all concrete evidence, and the blinded judge selected `evidence` for every case. One judge response had an unescaped newline inside a JSON string; the benchmark parser now salvages the structured label and score fields from that recoverable form.

## Standard Template Baseline Path

The benchmark and reporting scripts now include `standard_compaction_template` as an ordered baseline. The reducer prompt body is read directly from `codex-rs/core/templates/compact/prompt.md` and is sent through the same benchmark safety prefix and canonical full-context wrapper as the custom full-context variants. Report generation derives the ordered variant list from `reductions.jsonl`, so existing runs without standard-template rows still regenerate, while new runs can compare the shipped template against the custom nudges directly.

## Readable quality artifacts

For qualitative review, regenerate the readable reports with:

```powershell
py -3 scripts/report-context-helper-prompt-quality.py --run-dir logs/context-helper-prompt-benchmarks/full-20-30-s6-v4
```

The generated `reports/quality-analysis.md` is the best starting point. It links to
per-variant quality reports, a `reports/test-case-index.md`, and one Markdown file
per sampled compaction window under `reports/test-cases/`. Those sample files
contain the canonical reducer input, full source transcript window, exact saved
variant prompts, full reduced outputs, deterministic metrics, judge notes, and raw
artifact paths needed to audit the result without reading JSONL directly.

The exact nudge variants, common safety prefix, input wrappers, and summary
tradeoffs are saved in:

- `scripts/context-helper-nudge-variants.md`
