# Context Helper Prompt Variant Benchmark Results

This is the tracked, redacted summary for the prompt-variant benchmark run saved in the local ignored artifact directory:

`logs/context-helper-prompt-benchmarks/full-20-30-s6-v3`

The ignored artifact directory contains the raw transcript excerpts, reducer prompts, reducer outputs, judge outputs, and JSONL rows. Those raw artifacts are intentionally not tracked because they contain real local Codex conversation excerpts.

## Run Configuration

- Run status: preflight ok
- Thresholds: 20%, 30%
- Cooldown: 24 turns
- Samples per threshold: 6
- Context token budget for recorded run: 12,000
- Total samples: 12
- Reducer rows: 36
- Judge rows: 12
- Sessions loaded: 80
- Token events scanned: 19,324
- Trigger candidates: 1,209
- Selected samples: 6 at 20%, 6 at 30%

## Prompt Variants

- `prune`: user's direct pruning prompt
- `delta`: merge prior reduced context with new context delta
- `evidence`: preserve implementation evidence, commands, outputs, constraints, and uncertainty

All variants in the final v3 run received the same canonical reducer input: a deterministic prior summary plus the raw new context delta. Deterministic scoring and LLM judging used that same canonical input. Judge labels were blinded and balanced across A/B/C so each variant appeared under each label exactly four times across the 12 judgments.

## Headline Metrics

| variant | ok rows | avg compression | avg path retain | avg command retain | avg constraint retain | avg noise markers |
|---|---:|---:|---:|---:|---:|---:|
| prune | 12 | 0.203 | 0.501 | 0.031 | 0.012 | 0.250 |
| delta | 12 | 0.208 | 0.478 | 0.074 | 0.012 | 0.000 |
| evidence | 12 | 0.410 | 0.575 | 0.184 | 0.137 | 0.417 |

## Judge Best Counts

| variant | best-count |
|---|---:|
| prune | 1 |
| delta | 4 |
| evidence | 7 |

## Interpretation

The evidence-preserving prompt was the best directional performer in this small run: it retained the most paths, commands, and constraints, and won 7 of 12 blinded judge comparisons. It also produced larger summaries and more measured noise markers, so it is not a pure token-minimization winner.

The result should be treated as directional rather than final. It has only 12 judge rows and one LLM judgment per sample.

After review, the benchmark script default context token budget was lowered to 8,000 for future runs so the judge prompt has more headroom after reducer outputs are inserted.
