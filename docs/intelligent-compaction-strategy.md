# Intelligent Conversation Compaction Strategy

## Scope And Method

This note studies local Codex session artifacts from the last 10 days on this PC, ending on 2026-05-05. Sources inspected:

- `C:\Users\Oleh\.codex\sessions\**\*.jsonl`
- `C:\Users\Oleh\.codex\history.jsonl`
- Existing Codex compaction and token-usage code paths found under `codex-rs/`

The analysis used `event_msg` records with `type: token_count`, deduplicated adjacent repeated totals, and summed `last_token_usage`. Sessions were classified as code-related when their prompts or cwd matched repo/build/code terms. The savings estimate is a planning proxy, not billing truth: it estimates repeated context that could have been replaced by a compact summary after an early checkpoint.

## Findings

- Files in 10-day window: 1,367 session JSONL files.
- Code-related sessions: 1,365.
- Sessions with enough calls to benefit from earlier compaction: 133.
- Observed input tokens in code sessions: about 5.22B.
- Observed uncached input tokens: about 239.6M.
- Existing compact markers: 370, so compaction already happens, but often after very large context has accumulated.
- Estimated context-token savings from earlier checkpoint compaction: about 1.06B tokens, or 20.3% of observed input-context traffic.
- Estimated billable-token savings should be treated as a lower, capped range because cached input dominates. A practical range is 8-15% of uncached input on long coding sessions, with larger latency/context-window benefits than billing benefits.

Highest-value examples where earlier compaction would have helped:

| Date | Repo | Shape | Existing compact markers | Better compact point |
| --- | --- | ---: | ---: | --- |
| 2026-04-23 | `SlavaTask` | 125 user turns, 3,414 token-count calls | 42 | After first working implementation/test checkpoint, then after each verified subtask |
| 2026-04-29 | `Serial_to_Google_Doc_topdown` | 191 user turns, 5,997 token-count calls | 63 | After environment/bootstrap diagnosis and before repeated continuation loops |
| 2026-04-30 | `open_ai/codex` | 408 user turns, 2,178 token-count calls | 20 | After each committed logical chunk, especially before switching topic |
| 2026-04-29 | `Wizard_Erasmus` | 1,012 user turns, 3,933 token-count calls | 35 | After launch-chain diagnosis, after implementation plan, and after each tool/test subsystem |
| 2026-04-30 | `DonutGame` | 296 user turns, 4,259 token-count calls | 34 | After visual baseline, after asset/UI implementation, after verification loop |

## Strategy

Use compaction based on task coupling, not only token pressure.

Compact when all of these are true:

- A durable checkpoint exists: committed code, accepted plan, passing focused test, saved research memo, or clear diagnosis.
- The next work is less coupled with old transcript details than with repo state.
- At least one high-churn artifact has been summarized: files changed, commands run, test result, remaining blockers, and assumptions.

Avoid compaction when any of these are true:

- The current bug investigation depends on exact recent command output or a live failing trace.
- The agent is in the middle of a merge/conflict/edit sequence.
- The next step depends on unresolved user preference or a precise short-lived UI/runtime observation.

Recommended trigger classes:

- `Plan handoff`: after Plan Mode produces a decision-complete plan and before implementation starts.
- `First implementation checkpoint`: after the first coherent code/test iteration, even if the conversation is still small.
- `Logical commit boundary`: after each commit/push or verified patch series.
- `Topic switch`: before moving from code changes to research, review, build tuning, or another repo.
- `Long continuation loop`: after 8-12 continuation turns or after 80k active context tokens, whichever comes first.
- `Tool-output cleanup`: after large build/test/search outputs have been inspected and distilled.

## Implementation Plan For Codex

Add an intelligent compaction policy in `codex-rs/core` that scores every completed turn and can request a compact task before the next regular turn.

Policy inputs:

- Active context tokens before/after, already emitted in compaction analytics.
- Last and total token usage from `TokenUsageInfo`.
- Current collaboration mode and task kind.
- Recent events: plan completion, commit/push command success, test/build result, review result, file edit count, topic-switch keywords.
- Coupling signals: active failed command, unresolved approval, pending tool call, dirty edit in progress, live GUI/runtime observation.

Decision model:

- Compute `compact_score = token_pressure + checkpoint_score + topic_switch_score + loop_score - coupling_penalty`.
- Auto-compact when score is high and no coupling guard blocks it.
- Prefer a lower threshold after a first implementation checkpoint or accepted plan.
- Enforce a cooldown, for example no more than once per 10 minutes, except immediately after the first completed plan/implementation checkpoint.

Compaction summary must include:

- User goal and current chosen plan.
- Files changed or read that matter.
- Commands/tests run and outcomes.
- Open tasks, blockers, and exact next action.
- User preferences discovered in the session.
- Explicit note when raw command output or GUI state is intentionally not preserved.

Suggested rollout:

1. Add a non-mutating advisory mode first: emit an internal event and status-line hint when policy would compact.
2. Add tests for scoring around plan handoff, commit boundary, repeated continuation, high token pressure, and blocked live-debug cases.
3. Enable auto-compaction behind a feature flag for local Codex sessions.
4. Track analytics: `reason`, `score`, `cooldown_overridden`, `active_context_tokens_before`, `active_context_tokens_after`, and `turns_since_last_compact`.
5. Tune thresholds from local telemetry after 3-5 days.

## Expected Savings

Expected savings for coding-heavy local work:

- Context tokens: 15-20% reduction overall, with larger wins in long continuation sessions.
- Uncached/billable input: 8-15% reduction in practical cases because much of the repeated context is cached.
- Latency and reliability: likely better than the billing estimate, because shorter active context reduces model load, output drift from stale details, and late-session compaction pressure.

The biggest improvement is not compacting more often globally. It is compacting at semantic boundaries where the next subtask can rely on a short handoff plus the repository state instead of replaying the entire prior transcript.
