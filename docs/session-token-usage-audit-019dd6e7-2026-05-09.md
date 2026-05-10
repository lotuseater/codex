# Session Token-Usage Audit: 019dd6e7-fffc-7512-9608-00bcddbcfbd0

Date: 2026-05-09

## Scope

This report audits the Codex session referenced by:

```text
codex resume 019dd6e7-fffc-7512-9608-00bcddbcfbd0
```

The user-facing terminal line was:

```text
Token usage: total=73,083,578 input=67,216,567 (+ 1,581,604,480 cached) output=5,867,011 (reasoning 1,759,733)
```

The audited session was selected with `scripts/find-codex-sessions.ps1` for
`C:\Users\Oleh\Documents\GitHub\Serial_to_Google_Doc_topdown`:

| Field | Value |
| --- | --- |
| Session id | `019dd6e7-fffc-7512-9608-00bcddbcfbd0` |
| Transcript | `C:\Users\Oleh\.codex\sessions\2026\04\29\rollout-2026-04-29T04-43-42-019dd6e7-fffc-7512-9608-00bcddbcfbd0.jsonl` |
| Transcript size | 208,861,543 bytes |
| JSONL records | 87,335 |
| Session title | `see recent changes in the repo and utilities to transfer to C++, go on - we need all the app in C++` |
| Indexed raw cumulative tokens | 1,654,688,058 |

## Method

The parser read only the selected JSONL transcript and used cumulative
`total_token_usage` deltas from `token_count` events. This matters because the
transcript has repeated token-count records: 12,548 records included token info,
but 405 of them repeated an unchanged cumulative total. The attribution tables
below use the 12,143 positive cumulative-delta records, so the tables sum to the
final TUI totals.

Day buckets use the session timezone (`Europe/Kyiv`) from the transcript turn
context. Prompt-type classification is heuristic, based on the active user
message that preceded each positive token delta. Transcript byte attribution is
storage evidence and likely prompt-history pressure evidence, not an exact
provider-side token category. Exact prompt-layer attribution still needs Codex
token-ledger instrumentation.

## Token Math

The TUI total is not the raw provider cumulative total. In
`codex-rs/tui/src/token_usage.rs`, the displayed total is:

```text
display total = non-cached input + output
```

For this session:

```text
67,216,567 uncached input + 5,867,011 output = 73,083,578 displayed total
```

| Bucket | Tokens | Share / Meaning |
| --- | ---: | --- |
| Raw cumulative input | 1,648,821,047 | Everything sent as model input, including cached prefix reuse. |
| Cached input | 1,581,604,480 | 95.92% of raw input hit provider prompt cache. |
| Uncached input | 67,216,567 | New input charged/displayed by the TUI input field. |
| Output | 5,867,011 | 8.03% of displayed total. |
| Reasoning output | 1,759,733 | 29.99% of output tokens. |
| Raw cumulative total | 1,654,688,058 | Raw input plus output. |
| Displayed total | 73,083,578 | Uncached input plus output, matching the pasted terminal line. |

Per positive token-delta record:

| Metric | Value |
| --- | ---: |
| Average raw input | 135,783.7 |
| Average uncached input | 5,535.4 |
| Average output | 483.2 |
| Input p50 / p95 / max | 139,057 / 223,796 / 245,169 |
| Uncached p50 / p95 / max | 1,337 / 16,281 / 236,490 |
| Output p50 / p95 / max | 281 / 1,407 / 26,675 |

## Usage By Active Day

| Day | Positive token deltas | Raw input | Cached input | Uncached input | Output | Reasoning | Display total | Cached % | Display share |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2026-04-29 | 542 | 70,997,831 | 68,011,776 | 2,986,055 | 237,586 | 68,134 | 3,223,641 | 95.8% | 4.4% |
| 2026-04-30 | 2,712 | 370,736,375 | 358,091,264 | 12,645,111 | 1,168,008 | 248,461 | 13,813,119 | 96.6% | 18.9% |
| 2026-05-01 | 2,742 | 370,686,452 | 355,935,488 | 14,750,964 | 1,995,064 | 658,391 | 16,746,028 | 96.0% | 22.9% |
| 2026-05-06 | 2,102 | 278,665,151 | 268,807,936 | 9,857,215 | 769,624 | 199,927 | 10,626,839 | 96.5% | 14.5% |
| 2026-05-07 | 2,993 | 424,227,829 | 405,393,792 | 18,834,037 | 1,250,414 | 415,502 | 20,084,451 | 95.6% | 27.5% |
| 2026-05-08 | 317 | 41,248,878 | 38,643,584 | 2,605,294 | 158,162 | 62,317 | 2,763,456 | 93.7% | 3.8% |
| 2026-05-09 | 735 | 92,258,531 | 86,720,640 | 5,537,891 | 288,153 | 107,001 | 5,826,044 | 94.0% | 8.0% |

The biggest days were May 7, May 1, and April 30. They line up with many
continuation turns rather than one isolated huge answer.

## Usage By User Prompt Type

| Prompt type | Positive token deltas | Raw input | Cached input | Uncached input | Output | Reasoning | Display total | Display share |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `short_continue` | 9,859 | 1,350,496,228 | 1,297,057,920 | 53,438,308 | 4,860,973 | 1,417,081 | 58,299,281 | 79.8% |
| `short_other` | 1,119 | 144,435,683 | 138,356,352 | 6,079,331 | 460,744 | 137,375 | 6,540,075 | 8.9% |
| `substantive` | 543 | 76,269,675 | 73,695,872 | 2,573,803 | 265,244 | 94,881 | 2,839,047 | 3.9% |
| `commit_push` | 262 | 35,346,099 | 32,798,464 | 2,547,635 | 131,435 | 60,264 | 2,679,070 | 3.7% |
| `verify_or_test` | 240 | 24,574,607 | 23,277,568 | 1,297,039 | 90,247 | 24,341 | 1,387,286 | 1.9% |
| `review_or_feedback` | 120 | 17,698,755 | 16,418,304 | 1,280,451 | 58,368 | 25,791 | 1,338,819 | 1.8% |

Message counts by prompt type:

| Prompt type | User messages |
| --- | ---: |
| `short_continue` | 213 |
| `review_or_feedback` | 55 |
| `substantive` | 45 |
| `commit_push` | 44 |
| `short_other` | 37 |
| `verify_or_test` | 28 |
| `instructions` | 26 |

The single largest lever is not the wording `go on` by itself. The issue is
that these short continuation turns kept one long, high-context session alive
for days. Each continuation had access to a large cached prefix, but it still
added uncached deltas and output while repeatedly resending a very large raw
input context.

## Most Expensive User Turns

| Turn idx | Day | Prompt type | Positive token deltas | Raw input | Cached input | Uncached input | Output | Reasoning | Display total | Prompt summary |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 186 | 2026-05-01 | `short_continue` | 333 | 44,463,908 | 42,494,336 | 1,969,572 | 231,142 | 131,821 | 2,200,714 | `go on` |
| 322 | 2026-05-07 | `short_continue` | 132 | 17,536,819 | 16,153,600 | 1,383,219 | 59,143 | 19,270 | 1,442,362 | `go on` |
| 321 | 2026-05-07 | `short_continue` | 105 | 14,228,803 | 12,989,312 | 1,239,491 | 42,225 | 15,730 | 1,281,716 | `go on` |
| 277 | 2026-05-06 | `short_continue` | 193 | 25,716,949 | 24,696,704 | 1,020,245 | 77,288 | 22,609 | 1,097,533 | `go on` |
| 377 | 2026-05-07 | `short_continue` | 111 | 14,462,685 | 13,483,136 | 979,549 | 61,186 | 23,666 | 1,040,735 | `go on` |
| 179 | 2026-05-01 | `short_continue` | 234 | 35,007,542 | 34,103,040 | 904,502 | 133,272 | 40,040 | 1,037,774 | `go on` |
| 269 | 2026-05-06 | `short_continue` | 154 | 21,515,078 | 20,556,032 | 959,046 | 71,070 | 27,809 | 1,030,116 | `go on` |
| 280 | 2026-05-07 | `short_continue` | 310 | 48,631,511 | 47,721,728 | 909,783 | 116,629 | 30,734 | 1,026,412 | `go on` |
| 299 | 2026-05-07 | `substantive` | 199 | 29,425,977 | 28,562,048 | 863,929 | 91,975 | 36,128 | 955,904 | `can we rather decrease Python and CPython usage all together...` |
| 159 | 2026-05-01 | `short_continue` | 152 | 21,742,169 | 20,930,560 | 811,609 | 115,034 | 28,098 | 926,643 | `go on` |
| 185 | 2026-05-01 | `short_continue` | 91 | 11,705,630 | 10,879,104 | 826,526 | 50,245 | 31,025 | 876,771 | `go on` |
| 327 | 2026-05-07 | `short_continue` | 63 | 7,796,081 | 6,975,104 | 820,977 | 27,156 | 9,777 | 848,133 | `go on` |
| 267 | 2026-05-06 | `short_continue` | 216 | 29,297,128 | 28,578,816 | 718,312 | 73,221 | 27,241 | 791,533 | `go on` |
| 15 | 2026-04-29 | `short_continue` | 147 | 20,310,108 | 19,647,104 | 663,004 | 62,194 | 23,012 | 725,198 | `after that go on with the conversion` |
| 190 | 2026-05-01 | `short_continue` | 124 | 17,039,175 | 16,423,424 | 615,751 | 91,258 | 29,961 | 707,009 | `go on` |

## Largest Per-Call Spikes

Largest uncached-input spikes:

| Timestamp | Day | Prompt type | Raw input | Cached input | Uncached input | Output | Reasoning | Prompt summary |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 2026-05-01T16:51:31.116Z | 2026-05-01 | `short_continue` | 238,922 | 2,432 | 236,490 | 378 | 27 | `go on` |
| 2026-04-29T22:31:11.578Z | 2026-04-30 | `short_continue` | 217,058 | 6,528 | 210,530 | 639 | 212 | `go on` |
| 2026-04-29T20:06:38.465Z | 2026-04-29 | `short_continue` | 216,014 | 6,528 | 209,486 | 1,035 | 626 | `go on` |
| 2026-05-01T14:09:22.374Z | 2026-05-01 | `short_continue` | 216,629 | 11,136 | 205,493 | 781 | 516 | `go on` |
| 2026-05-07T00:01:15.209Z | 2026-05-07 | `short_continue` | 210,839 | 6,528 | 204,311 | 756 | 444 | `go on` |
| 2026-04-29T19:06:07.024Z | 2026-04-29 | `short_other` | 206,644 | 6,528 | 200,116 | 521 | 170 | `optimize iterations of 500+ lines to take less...` |
| 2026-05-01T15:40:03.680Z | 2026-05-01 | `short_continue` | 201,447 | 2,432 | 199,015 | 6,685 | 1,086 | `go on` |
| 2026-04-29T21:43:30.467Z | 2026-04-30 | `short_continue` | 203,357 | 6,528 | 196,829 | 772 | 0 | `go on` |
| 2026-04-30T21:07:03.284Z | 2026-05-01 | `short_other` | 205,896 | 11,136 | 194,760 | 842 | 516 | `plan carefully how to proceed with whole conversion to C++ - and do` |
| 2026-05-07T15:31:07.733Z | 2026-05-07 | `short_continue` | 222,743 | 28,032 | 194,711 | 913 | 516 | `go on` |

Largest output spikes:

| Timestamp | Day | Prompt type | Raw input | Cached input | Uncached input | Output | Reasoning | Prompt summary |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 2026-05-01T18:19:03.279Z | 2026-05-01 | `short_continue` | 159,984 | 158,592 | 1,392 | 26,675 | 2,426 | `go on` |
| 2026-04-30T22:01:18.378Z | 2026-05-01 | `substantive` | 164,621 | 164,224 | 397 | 22,615 | 0 | `"The generic slicing planner is not present in this repo...` |
| 2026-05-01T02:08:05.347Z | 2026-05-01 | `short_continue` | 56,417 | 54,656 | 1,761 | 21,901 | 6,568 | `go on` |
| 2026-05-01T12:34:07.788Z | 2026-05-01 | `short_continue` | 113,929 | 112,000 | 1,929 | 21,487 | 4,725 | `go on` |
| 2026-05-01T02:29:57.245Z | 2026-05-01 | `short_continue` | 163,292 | 162,688 | 604 | 19,957 | 2,416 | `go on` |
| 2026-05-01T12:52:30.947Z | 2026-05-01 | `short_continue` | 205,531 | 204,672 | 859 | 19,614 | 62 | `go on` |
| 2026-05-01T01:28:26.750Z | 2026-05-01 | `short_continue` | 123,312 | 120,192 | 3,120 | 15,691 | 0 | `go on` |
| 2026-04-30T22:33:31.148Z | 2026-05-01 | `short_continue` | 96,606 | 91,520 | 5,086 | 14,771 | 0 | `go on` |
| 2026-05-06T11:01:45.246Z | 2026-05-06 | `short_continue` | 122,427 | 120,192 | 2,235 | 14,755 | 239 | `go on` |
| 2026-05-01T08:00:37.264Z | 2026-05-01 | `short_continue` | 171,505 | 159,104 | 12,401 | 13,928 | 1,270 | `go on` |

## Transcript Storage Pressure

| Record type | Records | Bytes |
| --- | ---: | ---: |
| `response_item` | 57,115 | 132,808,473 |
| `event_msg` | 29,728 | 57,931,036 |
| `compacted` | 133 | 11,630,407 |
| `turn_context` | 358 | 6,469,598 |
| `session_meta` | 1 | 22,029 |

`response_item` dominates transcript storage because it includes assistant
messages, reasoning records, function calls, and function-call outputs.

Response item counts:

| Response item type | Count |
| --- | ---: |
| `function_call` | 18,659 |
| `function_call_output` | 18,650 |
| `reasoning` | 9,654 |
| `message` | 5,546 |
| `custom_tool_call` | 2,300 |
| `custom_tool_call_output` | 2,298 |
| `tool_search_call` | 4 |
| `tool_search_output` | 4 |

Visible message text bytes:

| Role / phase | Bytes |
| --- | ---: |
| Assistant messages | 1,367,361 |
| Developer messages | 1,076,456 |
| User messages | 528,246 |
| Assistant commentary | 1,062,150 |
| Assistant final answers | 283,407 |

Turn-context records were also material: 358 records totaling 6,469,598 bytes,
with an average of 18,071.5 bytes and a p95 of 25,541 bytes.

## Tool Output Payloads

| Tool output type | Calls | Bytes |
| --- | ---: | ---: |
| `shell_command` | 18,405 | 81,662,820 |
| `view_image` | 9 | 1,102,113 |
| `custom_tool_call_output` | 2,298 | 414,893 |
| `first_moves_predict` | 30 | 78,655 |
| `update_plan` | 163 | 19,265 |
| `automation_harness_detect` | 3 | 3,375 |
| `list_agents` | 31 | 2,932 |
| `spawn_agent` | 4 | 1,400 |
| `close_agent` | 1 | 1,398 |
| `request_user_input` | 2 | 589 |
| `dab_find_window` | 2 | 186 |

The largest transcript payload source was shell output. This does not mean
shell output alone consumed 81.7 MB of provider tokens, because prompt
construction and compaction decide what enters each future model request.
However, large command outputs are the most obvious stored conversation payload
and a high-risk source of prompt-history pressure.

## Findings

1. Prompt caching worked, but the cached prefix was too large.
   Cached input was 95.92% of raw input. That is good cache behavior, but it
   also proves Codex kept resending a huge prompt/context shape.

2. The displayed 73.1M tokens were mostly continuation-loop overhead.
   `short_continue` prompts account for 58.3M displayed tokens, or 79.8% of
   the displayed total.

3. The root cause is session shape, not one bad response.
   The transcript spans several active days, 448 user messages, 12,143 positive
   token-delta records, 18,650 function-call outputs, and 133 compaction
   records. The session kept accumulating work instead of handing off into
   smaller fresh sessions.

4. Shell output is the biggest stored payload class.
   `shell_command` outputs contributed 81,662,820 bytes to the transcript.
   Artifact-backed output would keep full logs available while preventing the
   model history from carrying the full text by default.

5. There are large uncached spikes even inside a cached session.
   Some calls had more than 190k uncached input tokens. Those look like
   cache-boundary, compaction, or large-context refresh moments and need
   prompt-layer telemetry to diagnose precisely.

6. The current TUI does not explain where tokens went.
   It correctly displays uncached input plus output, but it does not break down
   prompt layers such as instructions, history, tool outputs, images, subagent
   context, cached input, uncached input, and output.

## Recommendations
- Do not run an explicit `/compact` before every `go on`. The sampled session had enough continuation prompts that this would trade one repeated-token problem for many extra compaction model calls. Use thresholded continuation-aware semantic compaction instead: count short continuation turns separately, compact only after repeated continuations or other work/token/tool thresholds, and respect a cooldown after each compact.
- Use subagents more selectively, but make the default cheaper. Spawned agents should default to no forked history, use Slow context-budget mode, and receive a narrow context contract with exact first reads. Reuse stable helpers for related follow-up work, and compact/restart/close those helpers based on usefulness instead of spawning a full-context replacement.
- Keep full-history forks explicit. `fork_turns="all"` should remain available for rare cases where the child truly needs the parent transcript, but omitted `fork_turns` should mean no inherited conversation history.
- Streaming canary on `019dd6e7-fffc-7512-9608-00bcddbcfbd0`: 448 user turns contained 199 short continuation turns. Compacting before every continuation would add 199 compaction calls; the threshold-plus-cooldown continuation rule would fire 24 times on the same turn stream, avoiding 175 extra compaction calls.

### Immediate workflow changes

| Action | Expected impact | Reason |
| --- | --- | --- |
| Stop using one multi-day continuation session as the default. | Very high | Fresh sessions from short durable handoffs reset history pressure. |
| Add explicit checkpoint boundaries after each completed slice. | Very high | A `go on` chain should summarize and restart after commit/test/review milestones. |
| Avoid raw log/session dumps in chat. | High | Store full output on disk and inject only the digest, path, and command metadata. |
| Use bounded session discovery before opening JSONL files. | Medium | The selected session was found cheaply from the state index; broad scans are avoidable. |
| Keep continuation prompts, but make the engine guard them. | High | The user preference for `go on` is valid; Codex should checkpoint automatically when the session gets too large. |

### Codex implementation changes

| Priority | Change | What it should do |
| --- | --- | --- |
| P0 | Token ledger by prompt layer | Record instructions, AGENTS/project rules, conversation history, tool outputs, images, subagent inherited context, cached input, uncached input, output, and reasoning. |
| P0 | Artifact-backed tool output | Persist full stdout/stderr/images/session excerpts to files; put only small digests and handles in prompt history by default. |
| P0 | Continuation guardrails | After N calls, large uncached spikes, high context pressure, or a completed milestone, force a compact handoff before accepting another automatic continuation. |
| P1 | Scoped instruction loading | Keep a tiny stable root instruction prefix; load scoped AGENTS/skills only for touched domains. |
| P1 | Subagent/review inheritance limits | Default to `fork_turns=none` or a small recent window plus exact file lists and artifact handles. |
| P1 | Recoverable prompt elision | Replace repeated large tool outputs with path/hash/line-count summaries that can be expanded on demand. |
| P2 | First-moves and repo-scout discipline | Keep using first-moves prediction and repo scouts to avoid broad opening sweeps. |

### Reporting improvements

The next TUI or diagnostics view should make a table similar to this available
from the live session:

| Category | Why it matters |
| --- | --- |
| Static instructions | Shows always-on overhead from system/developer/project rules. |
| Scoped skills/AGENTS | Shows whether domain rules are loaded too broadly. |
| Conversation history | Shows the real cost of continuing a long session. |
| Tool output history | Shows whether shell/log/image output is dominating prompt pressure. |
| Subagent inherited context | Shows the cost of review/worker forks. |
| Cached input | Shows prompt-cache benefit. |
| Uncached input | Shows what is still newly charged/displayed. |
| Output/reasoning | Shows response-side cost. |

## What To Do Next For This Session

Do not resume `019dd6e7-fffc-7512-9608-00bcddbcfbd0` for new implementation
work unless the exact old state is needed. Create a compact handoff instead:

1. Current goal and repo path.
2. Latest commit/status.
3. Files changed and why.
4. Tests/builds already run.
5. Remaining concrete next step.
6. Paths to any full logs or artifacts.

Then start a fresh Codex session from that handoff. This preserves the useful
state without carrying the 209 MB transcript and the huge cached prompt prefix
forward.

## Sources Inspected

- `scripts/find-codex-sessions.ps1`
- `docs/token-usage-cache-audit-2026-05-06.md`
- `docs/token-usage-reduction-broader-audit-2026-05-07.md`
- `docs/token-saving-tools/codex-fork-token-saving-plan.md`
- `codex-rs/tui/src/token_usage.rs`
- `codex-rs/utils/output-truncation/src/lib.rs`
- `C:\Users\Oleh\.codex\sessions\2026\04\29\rollout-2026-04-29T04-43-42-019dd6e7-fffc-7512-9608-00bcddbcfbd0.jsonl`

## Verification

- Re-ran `scripts/find-codex-sessions.ps1` for
  `C:\Users\Oleh\Documents\GitHub\Serial_to_Google_Doc_topdown` and confirmed
  the selected session id, path, transcript size, title, and indexed token
  total.
- Re-ran a bounded JSONL parser against the selected transcript and confirmed:
  - `input=1,648,821,047`
  - `cached=1,581,604,480`
  - `uncached=67,216,567`
  - `output=5,867,011`
  - `reasoning=1,759,733`
  - `display_total=73,083,578`
- Confirmed the displayed total matches the TUI formula:
  `67,216,567 + 5,867,011 = 73,083,578`.
- No Rust tests were run because this is a documentation-only research
  deliverable.
