# Compaction Max Output Plan

Status: accepted workflow plan, normalized on 2026-05-21.

## Purpose

Capture the durable implementation direction for two related failures:

- Normal turns can end with `response.incomplete` when the API reaches
  `max_output_tokens`; this should be surfaced as a typed incomplete response,
  not as a generic stream disconnect or frozen turn.
- Auto-compaction can also end with `response.incomplete/max_output_tokens`;
  this must fail cleanly, preserve the existing history, and avoid immediate
  retry loops that repeatedly start post-turn cleanup.

## Patch Shape

1. Preserve incomplete response details through the SSE/API layer.
   Add a typed event or error shape that carries at least the incomplete reason,
   response id when available, and token usage when available.

2. Separate normal turn output budgets from compaction output budgets.
   Normal user turns should have enough output budget to finish expected model
   responses, while compaction can keep a narrower budget and must handle
   truncation explicitly.

3. Treat incomplete compaction as failed compaction, not successful history
   replacement.
   Only replace session history when a valid compacted history was produced.
   On `max_output_tokens`, record a guarded failure state or otherwise suppress
   immediate repeated cleanup attempts for the same over-limit state.

4. Keep hard-limit pressure behavior explicit.
   The hard-limit/post-turn path should remain independent from semantic
   compaction enablement, and any cooldown or retry suppression should cover
   both the hard-limit trigger and semantic compaction path.

## Verification Targets

- `codex-api` SSE coverage:
  `response.incomplete` with `max_output_tokens` is parsed and surfaced with
  structured reason data instead of a stream-disconnect-only error.

- Core compaction coverage:
  a post-turn hard-limit compaction stream ending in
  `response.incomplete/max_output_tokens` does not replace history, does not
  retry indefinitely during the same cleanup cycle, and still permits a later
  user turn.

- Context reduction policy coverage:
  early pressure compaction still fires when semantic compaction is disabled,
  and cooldown/retry behavior is applied consistently.

After Rust edits, run formatting and focused release checks such as:

```powershell
cd codex-rs
just fmt
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-api sse
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-context-reduction
```

If implementation touches `codex-core`, add a focused core release test lane:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core <focused-test-filter>
```

## Non-Goals

- Do not expand the in-progress `session`/`turn` refactor unless compile fallout
  makes it necessary.
- Do not treat truncated compaction output as a valid compact.
- Do not rely only on the semantic compaction cooldown, because hard-limit
  post-turn compaction can bypass that decision path.
