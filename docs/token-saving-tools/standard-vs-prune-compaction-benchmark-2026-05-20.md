# Standard vs Prune Compaction Prompt Benchmark - 2026-05-20

## Summary

This benchmark compares the previous standard non-empty compaction prompt, archived as
`standard_compaction_template`, against the current reduction prompt in
`codex-rs/core/templates/compact/prompt.md`, represented by the `prune` variant.

Result: `prune` is the better default for this sample set. It used fewer prompt tokens,
preserved more actionable paths and commands, and won the LLM quality judge in 2 of 3
cases. The tradeoff is that it produced larger reduced contexts than the previous
standard prompt.

## Sources

- Existing run: `logs/context-helper-prompt-benchmarks/standard-vs-prune-20-24-20260520-005804`
- Existing summary: `scripts/context-helper-prompt-benchmark-results.md`
- Benchmark script: `scripts/benchmark-context-helper-prompt-variants.py`
- Quality report generator: `scripts/report-context-helper-prompt-quality.py`
- Readable quality report: `logs/context-helper-prompt-benchmarks/standard-vs-prune-20-24-20260520-005804/reports/quality-analysis.md`

The run used 3 sampled trigger windows from recent local Codex sessions, threshold 20,
cooldown 24 turns, seed `20260519`, context token budget 8000, and the `codex-exec`
LLM backend for reductions and pairwise judging.

## Token And Retention Metrics

| variant | ok rows | avg prompt tokens | avg output tokens | avg compression | avg path retain | avg command retain | avg constraint retain | avg noise markers |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `standard_compaction_template` | 3 | 2,960 | 491 | 0.170 | 0.363 | 0.044 | 0.000 | 0.000 |
| `prune` | 3 | 2,797 | 572 | 0.214 | 0.598 | 0.156 | 0.000 | 0.000 |

Compared with the previous standard prompt, `prune` used 163 fewer prompt tokens on
average, about 5.5% less prompt overhead. It emitted 81 more output tokens on average,
about 16.5% larger reduced contexts. That larger output carried more useful evidence:
path retention improved by 0.235 absolute and command retention improved by 0.112
absolute.

## Quality Results

| sample | bucket | judge winner | judge scores |
| --- | --- | --- | --- |
| `thr20-early-82019a2ca459` | early | `prune` | `prune` 9, `standard_compaction_template` 8 |
| `thr20-middle-316d7f8da2ef` | middle | `prune` | `prune` 9, `standard_compaction_template` 5 |
| `thr20-late-5af0fa459d2e` | late | `standard_compaction_template` | `standard_compaction_template` 9, `prune` 8 |

The judge favored `prune` in 2 of 3 cases. The readable quality report indicates the
middle case was the clearest win for `prune`: it preserved more implementation-relevant
context. The late case was the standard prompt's narrow win, mostly because its shorter
summary was still accurate enough for that continuation.

## Interpretation

Use the current `prune` prompt when the objective is preserving implementation context
for an agent that will continue work after compaction. It is less aggressive than the
previous standard prompt, but the retained paths and commands are more valuable than the
extra 81 output tokens in these samples.

The previous standard prompt remains a stronger compression baseline. If future work
optimizes purely for output size, it is still useful as a comparison point, but this run
does not support reverting to it as the default for continuation quality.

## Reproduction

Regenerate the readable reports for the existing run:

```powershell
py -3 scripts/report-context-helper-prompt-quality.py --run-dir logs/context-helper-prompt-benchmarks/standard-vs-prune-20-24-20260520-005804
```

Run a fresh equivalent benchmark with a new output directory:

```powershell
py -3 scripts/benchmark-context-helper-prompt-variants.py --thresholds 20 --cooldown-turns 24 --samples-per-threshold 3 --variants standard_compaction_template,prune --out-dir logs/context-helper-prompt-benchmarks/standard-vs-prune-20-24-NEW
```

## Limitations

- This is a small 3-sample targeted run, not a broad statistical benchmark.
- The samples came from local Codex session history and may overrepresent recent work.
- Constraint retention was zero for both prompts in this run, so the result mainly
  differentiates path, command, compression, and judge quality behavior.
