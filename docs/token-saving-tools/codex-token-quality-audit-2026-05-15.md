# Codex Token And Quality Audit - 2026-05-15

## Scope

- Window: local Codex sessions updated from 2026-05-08 through 2026-05-15.
- Source of truth for session selection: `C:\Users\Oleh\.codex\state_5.sqlite`.
- Long-talk threshold: sessions with `threads.tokens_used >= 10M`, plus linked child/helper sessions `>= 5M`.
- Deliverable: evidence-backed feature assessment and ranked patch plan. No runtime code changes were made for this audit.
- Important caveat: `context_budget_mode` is not stored in `threads`, so slow-mode quality is estimated from current config, code behavior, and observed token bands, not from an A/B control group.

## Sources Inspected

- Session DB: `C:\Users\Oleh\.codex\state_5.sqlite`, including `threads` and `thread_spawn_edges`.
- Selected transcripts: JSONL files under `C:\Users\Oleh\.codex\sessions\2026\05\...`.
- Cache DB: `C:\Users\Oleh\.claude\cache\tool_cache.sqlite`.
- First-moves DBs: `C:\Users\Oleh\.codex\cache\first-moves\*\first_moves.sqlite`.
- Active config: `C:\Users\Oleh\.codex\config.toml`.
- Repo docs: `docs/token-usage-cache-audit-2026-05-06.md`, `docs/operation-cache-status.md`, `docs/token-saving-tools/context-reducer-lab-results-2026-05-08.md`, `docs/token-saving-tools/graphify.md`.
- Runtime code: `codex-rs/core/src/session/turn.rs`, `codex-rs/core/src/session/checkpoint_policy.rs`, `codex-rs/core/src/session/first_moves.rs`, `codex-rs/context-pack/src/lib.rs`, `codex-rs/tui/src/chatwidget/tests/plan_mode.rs`.
- Lab reports: `C:\Users\Oleh\Documents\GitHub\context-reducer-lab\reports\context-reducer-suite-2026-05-08.md`, `graphify-scout-dist-refresh-2026-05-09.md`, `dab-apps-deep-20260508-101147.json`.

## Executive Findings

1. Recent token burn is concentrated in a small number of very long root sessions. The 21-session audit set captured 1.223B of 1.505B recorded state tokens, or 81.2% of the 7-day total.
2. Provider prompt caching is working well: selected transcript token events showed about 1.714B input tokens with about 1.593B cached input tokens, roughly 92.9% cached. This reduces provider recomputation, but it does not stop the context from becoming huge or slow.
3. Local operation-cache savings are real but small compared with prompt-cache and context size. From 2026-05-08 through 2026-05-15, the shared operation cache recorded 1,128 hits, 8,692 misses, and about 3.06 MB text saved.
4. Automatic self-review is a major aggregate cost. In the same 7-day window, 91 review subagent sessions consumed 264.4M recorded tokens; 17 sessions above 5M accounted for 139.6M.
5. Slow mode is useful but incomplete. It keeps most turns below the 75% context-window band, tightens first-moves injection, and disables first-moves prewarm. Still, 438 selected token-count events crossed 75% of the 258,400-token context window, and four sessions crossed it dozens of times.
6. Graphify/context-pack functionality has the best current routing upside. The latest lab report promotes `conservative_graphify_candidate` with 82.1% positive coverage, 98.2% savings, zero negative injections, and max 21 ms latency, while the current Codex graphify strategy had negative-case regressions.
7. DAB is operationally solid in canaries. The deep DAB app canary passed 27/27 cases, including find-window, visual scan, send keys, element map, smart click, drag, terminal focus, navigation, and screenshot+click.

## Session Metrics

### 7-Day State DB Totals

| Class | Sessions | Recorded tokens |
| --- | ---: | ---: |
| All sessions | 151 | 1,505,329,262 |
| Main/root-like | 117 | 1,430,463,063 |
| Helper | 13 | 46,714,377 |
| Explorer | 16 | 18,261,752 |
| Worker | 5 | 9,890,070 |

By source, review subagents deserve separate attention even though most are below the long-talk threshold:

| Source | Sessions | Recorded tokens |
| --- | ---: | ---: |
| CLI/root | 18 | 1,164,607,797 |
| Automatic review subagent | 91 | 264,405,047 |
| Explicit spawned agents | 42 | 76,316,418 |

Model mix was dominated by `gpt-5.5` and `xhigh`:

| Field | Sessions | Recorded tokens |
| --- | ---: | ---: |
| `model = gpt-5.5` | 132 | 1,483,814,845 |
| `reasoning_effort = xhigh` | 121 | 1,453,568,391 |

### Top Projects

| Project | Sessions | Recorded tokens |
| --- | ---: | ---: |
| `open_ai/codex` | 51 | 658,243,540 |
| `context-reducer-lab` | 25 | 234,316,534 |
| `DonutGame` | 12 | 179,248,649 |
| `Serial_to_Google_Doc_topdown` | 29 | 146,730,717 |
| `AppleHedgehog` | 11 | 134,582,808 |
| `Cognos` | 4 | 73,461,128 |
| `Wizard_Erasmus` | 4 | 53,049,605 |

### Included Long Sessions

| Session | Project | Recorded tokens | Final token-count total | Notes |
| --- | --- | ---: | ---: | --- |
| `019e135e` | `codex` | 216,651,943 | 216,651,943 | dependency/build-size work |
| `019e047d` | `context-reducer-lab` | 137,375,488 | 137,375,488 | git push/failure investigation |
| `019e08c4` | `codex` | 133,667,211 | 133,667,211 | `/slow` mode/token policy |
| `019e232d` | `codex` | 128,467,083 | 266,089,202 | Prolog/ProbLog research; state/event mismatch |
| `019e1dc6` | `DonutGame` | 123,027,207 | 268,036,108 | game/pixel-art work; state/event mismatch |
| `019e1dcb` | `AppleHedgehog` | 113,429,853 | 113,429,853 | resume latest Claude task |
| `019e22c8` | `Serial_to_Google_Doc_topdown` | 68,889,355 | 284,862,782 | continuation work; state/event mismatch |
| `019e0721` | `context-reducer-lab` | 65,891,160 | 65,891,160 | Serena/context tools exploration |
| `019e20e1` | `Cognos` | 60,677,396 | 60,677,396 | resume latest Claude conversation |
| `019e239f` | `Wizard_Erasmus` | 44,571,567 | 44,571,567 | DAB improvement task |

The selected transcripts contained 18,814 token-count events. Three selected sessions had large `threads.tokens_used` vs final `token_count` mismatches:

| Session | Recorded tokens | Final token-count total | Difference |
| --- | ---: | ---: | ---: |
| `019e232d` | 128,467,083 | 266,089,202 | +137,622,119 |
| `019e1dc6` | 123,027,207 | 268,036,108 | +145,008,901 |
| `019e22c8` | 68,889,355 | 284,862,782 | +215,973,427 |

This should be fixed before using `threads.tokens_used` for exact budgeting or billing-style decisions.

## Slow Mode Assessment

### Current Behavior

Current global config has:

```toml
model = "gpt-5.5"
model_reasoning_effort = "xhigh"
context_budget_mode = "slow"
```

Slow mode is also the default when config does not specify a mode. The relevant behavior is:

- `ContextBudgetMode` supports `standard` and `slow`, with `slow` as the default.
- `auto_compact_token_limit_for_mode(...)` uses the model auto-compact limit in standard mode, but in slow mode caps it at `min(model_limit, model_context_window * 3 / 4)`.
- With the observed 258,400-token context window, the slow-mode hard cap is about 193,800 tokens.
- `effective_first_moves_config(...)` tightens first moves in slow mode: `max_context_moves <= 4`, `max_prewarm_files = 0`, and `min_context_score >= 0.70`.

### Observed Quality

Slow mode is good as a default guardrail, but not enough for the longest work.

| Band of last-turn input vs 258,400 window | Token-count events |
| --- | ---: |
| >= 60% | 3,618 |
| >= 70% | 924 |
| >= 75% | 438 |
| >= 80% | 278 |
| >= 90% | 27 |

The strongest evidence that slow mode helps is that most selected turns stay below the 75% line: only 438 of 18,814 token-count events crossed it. The strongest evidence that it is incomplete is that the same few sessions crossed the line repeatedly:

| Session | Project | Events >= 75% | Max input percent |
| --- | --- | ---: | ---: |
| `019e047d` | `context-reducer-lab` | 132 | 90.9% |
| `019e08c4` | `codex` | 115 | 91.4% |
| `019e232d` | `codex` | 96 | 93.8% |
| `019e0721` | `context-reducer-lab` | 93 | 93.6% |
| `019e22c8` | `Serial_to_Google_Doc_topdown` | 2 | 75.8% |

Estimated grade: B- for safety, C+ for token reduction. It prevents many catastrophic context-limit turns and makes first-moves cheaper, but it does not yet adapt to repeated high-context loops, review overhead, Plan mode, or oversized graph/context packets.

### Slow Mode Improvements

1. Add an adaptive slow-mode tier.
   - Behavior: if a session has repeated turns above 70% or one turn above 85%, temporarily lower the semantic checkpoint target to 60-65% and require a compact work note before more broad exploration.
   - Likely files: `codex-rs/core/src/session/turn.rs`, `codex-rs/core/src/session/checkpoint_policy.rs`.
   - Test: release test for auto-compact limit and policy transitions.

2. Store `context_budget_mode` in session telemetry.
   - Behavior: write mode into thread/session state or token events so future audits can compare `standard` vs `slow`.
   - Likely files: session state schema, OTEL/session telemetry, app-server turn payloads if needed.
   - Test: state/rollout reconstruction test plus telemetry snapshot.

3. Apply slow-mode budgets to graph/context packs.
   - Current `ContextPackRequest::new(...)` always uses a default 16-path budget. Slow mode should use fewer candidates unless the prompt explicitly asks for repo mapping.
   - Likely files: `codex-rs/core/src/session/first_moves.rs`, `codex-rs/context-pack/src/lib.rs`.
   - Test first in `context-reducer-lab` with the graphify scout use-case suite, then run `cargo test -p codex-context-pack --release`.

4. Permit safe semantic checkpoints in Plan mode.
   - Current `semantic_auto_compact_enabled(...)` disables semantic auto-compact in Plan mode. Plan review loops can therefore grow until hard compaction.
   - Behavior: allow plan-mode semantic checkpoint only after preserving the proposed plan artifact and pending implementation prompt state.
   - Likely files: `codex-rs/core/src/session/turn.rs`, `codex-rs/core/src/tasks/mod.rs`, `codex-rs/tui/src/chatwidget/tests/plan_mode.rs`.
   - Test: plan-mode snapshot/state tests plus a small lab replay of proposed-plan -> self-review -> implementation prompt.

## Feature Assessment

### Provider Prompt Cache

What works:

- Selected token-count events show about 92.9% cached input.
- This is the largest current token-saving mechanism.

Limits:

- Cached input still means the conversation is large. It can still slow turns, pressure context windows, and make compaction/review expensive.
- Prompt-cache metrics should not be confused with local operation-cache savings.

Patch direction:

- Optimize for smaller active context, not only higher cached ratio.
- Add per-session summaries that report input, cached input, uncached input, and local cache hits separately.

### Local Operation Cache

What works:

- Active wrapper reports operation cache enabled and points at the Wizard bridge.
- Codex-tagged shared-cache rows exist: 2,916 rows, 912 hits, 12.97 MB stored output.
- Codex all-project tool rows are mostly `Read`: 1,657 rows, 891 hits. `Bash` has 908 rows but only 20 hits; `Grep` has 342 rows and 1 hit.
- Current Codex checkout rows are present: 996 rows, 131 hits.

Limits:

- 7-day daily cache telemetry had 1,128 hits and 8,692 misses, about an 11.5% hit rate.
- Saved text was about 3.06 MB for the window, much smaller than prompt-cache/context effects.
- Miss reasons are dominated by `Bash` git operations, `Read` limit/offset variants, and disabled `rg`/operator patterns.

Patch direction:

- Do not broadly whitelist Bash. Add only narrow stable patterns with explicit invalidation, such as `git status --short`, `git diff --name-only`, and `rg --files` where repo state is part of the key.
- Add artifact handles for large command outputs so a compact digest can be shown while the raw output remains recoverable.
- Keep `scripts/test-operation-cache-runtime.ps1` as the canary for any bridge/interceptor change.

### First Moves And Scout

What works:

- Native system-wide first-moves storage is healthy in key repos.
- Codex first-moves DB: 216 paths, 152 prefetch logs, 2,064 path logs, 1,010 path hits.
- Serial repo DB: 258 paths, 832 path hits. DonutGame DB: 185 paths, 187 hits.
- Included transcripts had 109 `first_moves_predict` calls.
- Slow mode correctly disables prewarm and tightens injected context.

Limits:

- The repo-local `.first_moves.db` is not the real health signal for native first moves; the system-wide DB is.
- The audit cannot tie first-moves predictions to downstream token savings without better per-turn telemetry.
- Slow mode can reduce first-moves context too aggressively for broad unfamiliar repos if graph/context pack is not good enough.

Patch direction:

- Unify first-moves stats across repo-local and system-wide storage in one diagnostics command.
- Log prediction ID, selected paths, paths actually read, and whether a broad `rg` sweep followed anyway.
- Use graph/context pack first for architecture questions; use first-moves for exact next reads and warm-start paths.

### Auto-Compact

What works:

- Selected sessions contained 558 compact records/events, so compaction is active.
- Semantic checkpoint policy has useful triggers: continuation turns, work-token checkpoint, commit observed, tool-call churn, and early pressure.

Limits:

- Selected main sessions still had 993 events with last-turn input >= 180,000 tokens.
- Hard context-limit warnings were rare in transcript text compared with compact records, making compaction reason accounting hard.
- Plan mode disables semantic auto-compact, which can be expensive when plan review loops carry large context.

Patch direction:

- Add compaction reason to structured telemetry, not only warning text.
- Add adaptive thresholds after repeated high-context turns.
- Add plan-mode semantic checkpoint canary before changing runtime behavior.

### Self-Review And Plan Review

What works:

- Self-review and plan-review improve quality on risky changes, and the TUI has tests ensuring plan self-review occurs before implementation prompts.
- Review subagents often use cached input heavily, so they are less expensive than uncached from a provider-compute perspective.

Limits:

- Aggregate token cost is high: 91 review sessions used 264.4M recorded tokens in 7 days.
- 17 review sessions above 5M tokens consumed 139.6M.
- Plan-review cost is not cleanly separable from normal turns; included transcripts had 1,678 plan-review markers, but no reliable per-review cost field.

Patch direction:

- Add a review budget gate based on changed files, diff size, risk tags, and time since last review.
- Fingerprint the reviewed work slice. Skip or shorten review when a new automatic review would inspect the same fingerprint.
- Use a compact review packet by default: `git status`, `diff --stat`, exact changed paths, commands run, and relevant test output handles.
- Consider lower reasoning/model tier for low-risk review packets, but only after a lab replay verifies that quality does not regress on historical caught-bug examples.

### Graphify And Context Reducer Lab

What works:

- `codex-rs/context-pack/src/lib.rs` implements Graphify/Aider-style repo-map narrowing.
- The 2026-05-09 lab report promotes `conservative_graphify_candidate`: 82.1% positive coverage, 98.2% savings, zero negative injections, five fixed current misses, max 21 ms.
- The earlier Graphify note showed 10 Codex files represented as 241 nodes and 449 edges, with 98% coverage and about 8.7x fewer tokens for targeted queries in that sample.

Limits:

- The current Codex graphify strategy in the 2026-05-09 report had 59.0% coverage and six negative injections.
- The 2026-05-08 reducer suite explicitly says not to promote `first_moves` ranking changes and to keep many reducer replacements observe-only.
- Search, diff, run-check, and distillate reducers need `artifact_read`/`artifact_search` before replacing raw outputs.

Patch direction:

- Promote only the `conservative_graphify_candidate` behavior from the lab, not every high-saving candidate.
- Keep `rg_file_set_digest` observe-only until artifact continuation exists.
- Start any reducer/runtime change with a context-reducer-lab rerun before changing Codex.

### DAB

What works:

- The deep DAB canary passed 27/27 cases across window discovery, visual scan, send keys, element mapping, smart click, drag, terminal tabs/focus, app navigation, and screenshot+click.
- Included transcripts show live DAB use: 61 `dab_find_window`, 22 `dab_visual_scan`, 8 `dab_screenshot`, 7 `dab_ocr`, and several click/send-key/navigation calls.
- The reducer lab notes confirm the native DAB bridge rejects foreground click/send-keys when the requested target window is missing.

Limits:

- Session telemetry does not yet summarize DAB latency, target-confidence, or "wrong window avoided" outcomes.
- Visual evidence can be large unless screenshot artifacts are handled by path/handle instead of prompt text.

Patch direction:

- Add DAB operation telemetry: window match confidence, action latency, screenshot artifact path, OCR byte count, and rejected-action reason.
- Cache element maps per window generation with invalidation on title/process/rect change.
- Keep DAB verification screenshot-backed, but pass artifact handles into reviews instead of raw screenshot descriptions.

## Ranked Patch Plan

### 1. Add Token/Feature Telemetry Needed For Decisions

- Expected impact: high quality and medium token impact; unlocks better gates.
- Behavior: record `context_budget_mode`, compaction reason/phase, review trigger/fingerprint, first-moves prediction ID, local cache hit/miss, and DAB action summaries per turn/session.
- Likely files: `codex-rs/core/src/state`, `codex-rs/otel/src/events/session_telemetry.rs`, `codex-rs/otel/src/metrics/runtime_metrics.rs`, rollout reconstruction tests.
- Lab/canary: small local transcript fixture in `context-reducer-lab` that validates fields can reproduce this audit's key tables.
- Validation: focused release tests for state reconstruction and telemetry serialization.
- Risk: schema churn; keep fields additive and nullable.

### 2. Budget Automatic Review

- Expected impact: very high token impact; target 50M-100M fewer review tokens per busy week without disabling review.
- Behavior: fingerprint changed work slices, skip duplicate automatic review, use compact review packets, and downshift only low-risk reviews after fixture validation.
- Likely files: `codex-rs/tui/src/chatwidget/tests/plan_mode.rs`, `codex-rs/self-review/src/lib.rs`, `codex-rs/models-manager/self_review_instructions.md`, guardian/review session code.
- Lab/canary: replay historical review prompts where a bug was found vs no-finding reviews; compare findings under compact packet and lower tier.
- Validation: TUI plan/self-review tests plus self-review unit tests.
- Risk: quality regression if review is skipped too aggressively; use fingerprint plus risk tags, not a global throttle.

### 3. Make Slow Mode Adaptive

- Expected impact: high token impact for long talks; medium risk.
- Behavior: after repeated high-context turns, lower soft compact thresholds to 60-65%, require a compact work note, and budget context-pack paths more tightly.
- Likely files: `codex-rs/core/src/session/turn.rs`, `codex-rs/core/src/session/checkpoint_policy.rs`, `codex-rs/core/src/session/first_moves.rs`, `codex-rs/context-pack/src/lib.rs`.
- Lab/canary: replay selected long-session token bands in `context-reducer-lab` and assert fewer turns exceed 75% without losing required source paths.
- Validation: release tests around `auto_compact_token_limit_for_mode`, checkpoint policy, first-moves slow config, and context-pack budget.
- Risk: over-compaction can lose useful continuity; preserve scratchpad/work-note artifacts.

### 4. Promote Conservative Graphify Context Pack

- Expected impact: medium-high token impact and quality gain on opening turns.
- Behavior: replace current graphify/context-pack emission rules with the lab-promoted conservative candidate; avoid negative injections.
- Likely files: `codex-rs/context-pack/src/lib.rs`, `codex-rs/core/src/session/first_moves.rs`.
- Lab/canary: rerun `graphify-scout-dist-refresh-2026-05-09` suite before and after.
- Validation: `cargo test -p codex-context-pack --release` plus the lab report delta.
- Risk: stale or overconfident routing; require exact source reads before edits.

### 5. Add Artifact-Backed Command Output Flow

- Expected impact: high token impact for search/diff/test loops; larger implementation.
- Behavior: keep raw `rg`, diff, and run-check outputs in artifacts; pass compact digests with `artifact_read`/`artifact_search` handles into the model.
- Likely files: tool handlers for shell/search/diff, operation-cache integration, transcript serialization.
- Lab/canary: use the context-reducer suite rows for `rg_file_set_digest`, `run_check_digest`, and diff summaries.
- Validation: operation-cache runtime canary, focused shell output tests, transcript reconstruction tests.
- Risk: missing raw-output recovery would hurt correctness; artifact recovery must ship first.

### 6. Improve Operation Cache Narrowly

- Expected impact: medium token/time impact; low-medium risk if narrow.
- Behavior: cache stable read-only command families with repo-state keys: `git status --short`, `git diff --name-only`, `rg --files`, and maybe selected `git show --stat`.
- Likely files: `codex-rs/core/src/tools/operation_cache.rs`, Wizard bridge canonicalizer, `scripts/test-operation-cache-runtime.ps1`.
- Lab/canary: add one canary per command family; verify invalidation after file edits.
- Validation: existing operation-cache script plus Wizard bridge pytest.
- Risk: stale command output; avoid content `rg` replacement until artifact handles exist.

### 7. Add First-Moves Outcome Accounting

- Expected impact: medium quality and token impact.
- Behavior: report whether first-moves suggested paths were actually read and whether the agent still did a broad sweep.
- Likely files: `codex-rs/core/src/tools/handlers/first_moves.rs`, first-moves DB/stats code, telemetry.
- Lab/canary: small session fixture with predicted path, read hit, and broad-search fallback.
- Validation: first-moves stats tests.
- Risk: telemetry volume; store compact counters, not full prompts.

### 8. Add DAB Latency And Artifact Handles

- Expected impact: medium quality and time impact; small direct token impact.
- Behavior: record target confidence, action latency, screenshot/OCR artifact path, and rejected-action reason; cache element maps by window generation.
- Likely files: Wizard DAB bridge and Codex DAB tool handlers.
- Lab/canary: extend `dab-apps-deep` with latency and rejection assertions.
- Validation: DAB app suite and screenshot artifact inspection.
- Risk: GUI flakiness; keep canaries target-window explicit.

## Validation Lanes For Future Patches

- Session telemetry: state reconstruction tests and transcript fixture replay.
- Slow mode: `auto_compact_token_limit_for_mode` tests, checkpoint policy tests, first-moves slow-config tests, plan-mode compaction replay.
- Review gates: TUI plan-mode tests and self-review compact-packet fixture tests.
- Context pack: `context-reducer-lab` graphify scout suite, then `cargo test -p codex-context-pack --release`.
- Operation cache: `scripts/check-operation-cache.ps1` and `scripts/test-operation-cache-runtime.ps1`.
- DAB: `context-reducer-lab` DAB app suite with screenshot artifacts.
- Codex binary verification, only after runtime code changes: `scripts/build-local-codex.ps1 -Mode FastRelease`.

## Bottom Line

The best near-term token reduction is not a bigger cache. It is budgeting automatic review, making slow mode adaptive, and promoting the conservative graph/context-pack routing that already passed lab gates. The local operation cache should still improve, but only through narrow, telemetry-backed command families and artifact-backed raw-output recovery.
