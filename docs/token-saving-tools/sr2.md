# SR2 Token-Saving Research

Source:

- Local clone: `C:\Users\Oleh\Documents\GitHub\agent-context-tools-lab\sr2`
- Upstream: https://github.com/terminus-labs-ai/sr2
- Local status: cloned and source/docs inspected; not run end to end in this
  pass.

## Key Ideas

SR2 is a context-window compiler. It treats the prompt as a layered resource
with budgets, cache policies, compaction, summaries, and degradation rules.

Important mechanisms:

- Layers ordered from most stable to most volatile to preserve provider
  KV-cache prefixes.
- Three-zone conversation history:
  - raw recent turns,
  - compacted older turns,
  - summarized oldest turns.
- Config-driven pipeline with per-layer budgets and cache policies.
- Rule-based compaction for tool outputs and redundant fetches.
- Structured summaries that preserve decisions, unresolved issues, facts,
  preferences, and errors.
- Pre-emptive rotation before context pressure becomes emergency truncation.
- Metrics for token counts, cache hit rates, prefix stability, and degradation.
- Circuit breakers so optional context layers can fail open.

## How It Works

SR2 resolves each layer, applies cache policy, compacts or summarizes where
configured, enforces a total token budget, and emits a compiled context string
plus metrics. The key principle is that prompt layout is not incidental.
Stable content should stay at the top so provider prompt caching can reuse it,
while volatile conversation/tool data should be late and compactable.

For Codex, this maps directly onto system instructions, AGENTS rules, selected
repo context, durable memory, conversation history, and tool results. Codex
already has some compaction/autocompact behavior, but SR2's useful framing is a
formal context compiler with visible layer metrics and explicit degradation.

## Evidence From Source Review

The README and docs include:

- Pipeline engine resolving layers, checking cache, and enforcing budgets.
- A configuration example with immutable and append-only layers.
- A compaction guide with strategies like replacing code execution outputs with
  exit-code plus first lines and a result file pointer.
- Claimed managed-session reduction of about 52 percent input tokens versus a
  naive baseline.
- Claimed 100 percent KV-cache prefix hit rate in one benchmark.

I did not run SR2 on Codex conversation logs in this pass. Its design is still
highly relevant because it targets the exact failure mode observed here:
conversations growing faster than useful work.

## What Codex Should Take

Useful design elements:

- Replace ad hoc prompt assembly with an explicit context compilation trace.
- Keep stable layers stable and early:
  system instructions, developer instructions, project root identity, selected
  AGENTS shard fingerprints.
- Move volatile layers late:
  latest user turn, live tool outputs, current patch summary.
- Add three-zone history in Codex:
  raw recent turns, compacted tool-history zone, durable summary zone.
- Promote large completed tool chains into artifacts with structured summaries.
- Record per-layer token counts and prefix-stability percentages.
- Trigger compaction before emergency autocompact, using a predictable threshold.

## Risks And Gaps

- Over-compaction can remove evidence needed for review tasks.
- Summaries can become stale or too confident; they need provenance.
- Prefix stability helps cost/latency only if the provider's caching behavior
  recognizes the unchanged prefix.
- The context compiler must fail open; a broken retrieval layer should not block
  the agent from answering.

## Codex Implementation Candidates

1. Add a `ContextPlan` or `PromptPlan` debug object showing every prompt layer,
   token estimate, cache key, and truncation/compaction decision.
2. Add a compacted history zone that stores digest handles for older tool
   outputs and repeated file reads.
3. Add an old-turn structured summary with explicit categories:
   decisions, facts, open questions, files changed, artifacts, and user
   preferences.
4. Add prompt-prefix metrics to local telemetry:
   stable-prefix tokens, changed-prefix tokens, cache read/write tokens.
5. Add a max-token budget per layer so a runaway tool-result layer cannot crowd
   out current task instructions.
