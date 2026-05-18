# Context Helper Reduction Benchmark - 2026-05-19

Generated: 2026-05-19T02:08:39+03:00

Session root: `C:\Users\Oleh\.codex\sessions`

Session sample: 80 newest JSONL files by mtime, capped by `--limit 80`.

## Summary

- Parsed 80 sessions with `token_count` events and 13,889 token-count samples.
- Context windows observed: 258400: 80.
- Sessions crossing thresholds: 80k tokens = 3; 55% = 3; 60% = 3; 65% = 2; 70% = 2; 75% = 0.
- Sessions with multi-agent tool calls: 3; calls: `close_agent` 59, `compact_agent` 4, `followup_task` 14, `list_agents` 408, `restart_agent` 1, `resume_agent` 6, `send_message` 14, `spawn_agent` 118, `wait_agent` 217.
- Baseline helper model: 12.0k helper bundle, 2.5k parent retained overhead, 8.0k retained root summary.

## Recommendation

Use a bounded sidecar helper at 65% context pressure with a 24-turn cooldown, only when at least 6 more turns are expected. For automatic loop continuations where the agent is clearly going to keep working, allow an aggressive lane at 55%-60% with a 12-turn cooldown. Keep the turn cooldown as the default gate; do not bypass it with accumulated total prompt input. Do not spawn the reducer with full root context.

Prefer one-shot helpers with `fork_turns: "none"` for the reducer. If a persistent helper is kept warm for several reductions, compact it after every 2 reductions or when its retained state approaches roughly 30k-40k tokens; the helper state is a real model input cost even when it is not retained by the root agent.

This is a sidecar-reducer recommendation, not a recommendation to fork the full root transcript. A full-context helper duplicates the expensive prompt and usually loses before it can save anything.

## Model

For each session, a policy triggers when `last_token_usage.input_tokens >= model_context_window * threshold_percent`. A trigger is suppressed unless at least 6 later token-count samples exist in the same session. After a trigger, later root turns in the cooldown window are charged as if the prompt were reduced to the retained summary size:

`gross_saved += max(0, future_input_tokens - summary_tokens)`

`net_saved = gross_saved - helper_agent_tokens - parent_coordination_tokens`

Cooldown is a hard turn count in the default model. The separate bypass stress table shows why accumulated root input tokens are the wrong bypass signal: repeated high-context prompts quickly satisfy a small token threshold even when no genuinely new context has appeared.

## Current Production Anchor

- Existing model metadata auto-compaction is anchored near 90% of resolved context.
- Slow context-budget mode clamps that to 75% of the model context window.
- Semantic checkpoint pressure is effectively earlier: 80% of the slow-mode limit, or about 60% of the model context window.

The benchmark therefore treats 60%-65% as the practical early-reduction band and 75% as a late safety net.

## Highest-Pressure Sessions

| session | project | token samples | max input | max window | final input | agent calls |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-05-12T22-59-43-019e1dc6-1ed3-7463-b56e-59986d56b7fd | DonutGame | 7,321 | 193.4k | 74.8% | 84.1k | 346 |
| 2026-05-15T22-24-05-019e2d18-91aa-7fe2-b5d8-307e3d17545e | codex | 4,247 | 183.0k | 70.8% | 93.1k | 300 |
| 2026-05-17T13-58-47-019e3596-ab8a-7200-b5e0-c7eade392ace | codex | 2,077 | 160.1k | 61.9% | 68.8k | 195 |
| 2026-05-19T01-15-59-019e3d29-0437-7890-9a0f-507b03b03bfd | DonutGame | 43 | 58.3k | 22.6% | 0 | 0 |
| 2026-05-19T01-27-19-019e3d33-6634-70d3-a047-085ec762b6c6 | codex | 1 | 26.4k | 10.2% | 26.4k | 0 |
| 2026-05-19T02-03-07-019e3d54-2b46-72d3-bf0f-7cf3cf20bc58 | workspace | 8 | 23.6k | 9.1% | 23.6k | 0 |
| 2026-05-19T02-02-17-019e3d53-6b67-7d42-babe-8fcebd7d82a8 | workspace | 4 | 22.6k | 8.7% | 22.6k | 0 |
| 2026-05-19T02-00-23-019e3d51-ab6b-7d22-9c4a-a37397ad1387 | workspace | 3 | 22.5k | 8.7% | 22.5k | 0 |

## Threshold And Cooldown Sweep

Baseline cost model: one-shot helper, 12.0k helper bundle, 2.5k parent overhead, 8.0k retained summary.

| threshold | cooldown | strategy | triggers | sessions | triggers/1k samples | gross saved | helper cost | net saved | net/trigger |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 55% | 6 | one_shot | 370 | 3.8% | 26.6 | 259.3M | 5.4M | 253.9M | 686.3k |
| 55% | 12 | one_shot | 203 | 3.8% | 14.6 | 295.2M | 2.9M | 292.2M | 1.4M |
| 55% | 24 | one_shot | 117 | 3.8% | 8.4 | 331.1M | 1.7M | 329.4M | 2.8M |
| 55% | 48 | one_shot | 75 | 3.8% | 5.4 | 385.5M | 1.1M | 384.4M | 5.1M |
| 60% | 6 | one_shot | 213 | 3.8% | 15.3 | 150.7M | 3.1M | 147.7M | 693.2k |
| 60% | 12 | one_shot | 122 | 3.8% | 8.8 | 173.7M | 1.8M | 172.0M | 1.4M |
| 60% | 24 | one_shot | 76 | 3.8% | 5.5 | 199.6M | 1.1M | 198.5M | 2.6M |
| 60% | 48 | one_shot | 55 | 3.8% | 4.0 | 257.7M | 797.5k | 256.9M | 4.7M |
| 65% | 6 | one_shot | 101 | 2.5% | 7.3 | 70.1M | 1.5M | 68.6M | 679.2k |
| 65% | 12 | one_shot | 63 | 2.5% | 4.5 | 85.3M | 913.5k | 84.4M | 1.3M |
| 65% | 24 | one_shot | 43 | 2.5% | 3.1 | 106.6M | 623.5k | 106.0M | 2.5M |
| 65% | 48 | one_shot | 34 | 2.5% | 2.4 | 157.9M | 493.0k | 157.4M | 4.6M |
| 70% | 6 | one_shot | 18 | 2.5% | 1.3 | 10.9M | 261.0k | 10.7M | 592.9k |
| 70% | 12 | one_shot | 14 | 2.5% | 1.0 | 15.0M | 203.0k | 14.8M | 1.1M |
| 70% | 24 | one_shot | 12 | 2.5% | 0.9 | 22.3M | 174.0k | 22.1M | 1.8M |
| 70% | 48 | one_shot | 12 | 2.5% | 0.9 | 45.1M | 174.0k | 44.9M | 3.7M |
| 75% | 6 | one_shot | 0 | 0.0% | 0.0 | 0 | 0 | 0 | 0 |
| 75% | 12 | one_shot | 0 | 0.0% | 0.0 | 0 | 0 | 0 | 0 |
| 75% | 24 | one_shot | 0 | 0.0% | 0.0 | 0 | 0 | 0 | 0 |
| 75% | 48 | one_shot | 0 | 0.0% | 0.0 | 0 | 0 | 0 | 0 |

## Helper Cost Sensitivity

Sensitivity keeps the recommended 65% / 24-turn policy and varies helper bundle cost and retained summary size.

| threshold | cooldown | helper bundle | summary | triggers | helper cost | net saved |
| --- | --- | --- | --- | --- | --- | --- |
| 65% | 24 | 12.0k | 6.0k | 43 | 623.5k | 107.9M |
| 65% | 24 | 12.0k | 8.0k | 43 | 623.5k | 106.0M |
| 65% | 24 | 12.0k | 10.0k | 43 | 623.5k | 104.0M |
| 65% | 24 | 12.0k | 20.0k | 43 | 623.5k | 94.3M |
| 65% | 24 | 30.0k | 6.0k | 43 | 1.4M | 107.2M |
| 65% | 24 | 30.0k | 8.0k | 43 | 1.4M | 105.2M |
| 65% | 24 | 30.0k | 10.0k | 43 | 1.4M | 103.3M |
| 65% | 24 | 30.0k | 20.0k | 43 | 1.4M | 93.5M |
| 65% | 24 | 60.0k | 6.0k | 43 | 2.7M | 105.9M |
| 65% | 24 | 60.0k | 8.0k | 43 | 2.7M | 103.9M |
| 65% | 24 | 60.0k | 10.0k | 43 | 2.7M | 102.0M |
| 65% | 24 | 60.0k | 20.0k | 43 | 2.7M | 92.2M |

## New-Input Bypass Stress

This keeps the recommended 65% / 24-turn policy and varies only the accumulated-total-input bypass. `turn-only` disables the bypass. Small bypass values are noisy because a single high-context model call can add more than the bypass threshold.

| bypass | triggers | triggers/1k samples | helper cost | net saved |
| --- | --- | --- | --- | --- |
| turn-only | 43 | 3.1 | 623.5k | 106.0M |
| 32.0k | 398 | 28.7 | 5.8M | 57.4M |
| 80.0k | 398 | 28.7 | 5.8M | 57.4M |
| 160.0k | 398 | 28.7 | 5.8M | 57.4M |

## Helper State And Compaction

This table keeps the recommended 65% / 24-turn policy, 12.0k helper bundle, and 8.0k retained root summary. Persistent helper state grows by `summary_tokens + parent_overhead_tokens` after each reduction. A helper compaction costs the current helper state plus the compact output, then resets helper state to the compact output.

| helper strategy | triggers | helper compactions | max helper state | helper cost | net saved |
| --- | --- | --- | --- | --- | --- |
| one_shot | 43 | 0 | 0 | 623.5k | 106.0M |
| persistent_compact_every_1 | 43 | 42 | 18.5k | 2.1M | 104.6M |
| persistent_compact_every_2 | 43 | 21 | 29.0k | 1.9M | 104.7M |
| persistent_compact_every_3 | 43 | 14 | 39.5k | 2.0M | 104.6M |
| persistent_no_compact | 43 | 0 | 451.5k | 10.1M | 96.5M |

## Public API And Behavior Changes

None. This benchmark and report do not change production compaction behavior.

## Acceptance Gates For Production Promotion

- Keep reducer helpers sidecar-only: `fork_turns: "none"` or a bounded explicit artifact bundle.
- Require positive net savings under a 60k helper-bundle sensitivity case.
- Keep retained root summaries at or below 10k tokens for default operation.
- Avoid helper spawning for short sessions below 80k observed input tokens unless loop mode indicates at least 6 future turns.
- Do not use accumulated total prompt input as a cooldown bypass; require measured new source/context material if a token-based bypass is later added.
- Do not keep a persistent reducer helper un-compacted beyond 2 reductions or roughly 30k-40k retained helper state.
