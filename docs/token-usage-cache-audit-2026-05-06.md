# Token Usage And Cache Audit

Date: 2026-05-06

## Measurements

- Recent local Codex session volume is high: ISO week 17 had 1,382 session files totaling about
  662 MB, week 18 had 1,175 files totaling about 448 MB, and week 19 has 378 files totaling about
  284 MB so far.
- A sample of 233 May `token_count` events showed about 385.5M input tokens, 366.7M cached input
  tokens, and 1.1M output tokens. Cached input was about 95.1% of sampled input.
- Wizard/Codex tool-cache telemetry over 14 days showed 690 hits, 4,934 misses, and an overall hit
  rate of about 12.3%.
- The Codex-filtered cache rows were much smaller than the global row set: 1,602 entries, 145 hits,
  about 12.4 MB stored, and about 355 KB estimated saved.
- Repeated misses were dominated by Bash operations that are not cache-whitelisted, then first-time
  reads/searches. `Read` had most cache hits; `Bash`, `Grep`, and `Glob` were mostly misses.
- Project warming was uneven. `Wizard_Erasmus` was warmed many times, while this Codex checkout had
  only one observed project-cache warm event in the same telemetry window.

## Conclusions

- Provider-side prompt caching is already doing a lot of work. The large remaining cost is repeated
  broad context entering turns, not missing provider cache reuse.
- Earlier semantic compaction is the highest-leverage reduction because it shrinks the active
  conversation before another large cached prefix is sent.
- The local tool cache is useful for repeated file reads, but its current configuration cannot save
  much on one-off `rg`, `git`, and operator Bash commands.
- More consistent use of repo-local skills and first-move predictions should reduce unnecessary
  opening sweeps and duplicated file reads.

## Followed-Up Changes

- Post-turn semantic compaction now runs after ordinary task completion and can trigger before the
  hard auto-compact limit.
- The semantic policy includes focused checkpoints for sustained work, tool churn, and observed git
  commits when the active context is already meaningful.
- The build skill now records the Cargo single-filter rule so focused release tests are not retried
  with invalid multiple filters.

## Further Reduction Candidates

- Warm this Codex checkout in the project cache more consistently when the first-moves predictor is
  used successfully.
- Add cache-whitelisted wrappers for repeated safe read-only Bash patterns only when their output is
  stable enough to be worth caching.
- Prefer common-prefix Cargo filters and module-level `rg` searches over repeated exact-test or
  whole-repo probes.
- Keep token audits periodic and telemetry-backed; avoid guessing from a single large session.
