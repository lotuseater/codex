# TODO Triage

This note groups TODO-heavy areas by risk so cleanup work can be chosen by
ownership and impact instead of by raw TODO count.

## Compatibility And Removal Criteria

These TODOs usually protect older clients, legacy config, or protocol fallback
paths. Do not remove them until there is a named compatibility cutoff.

- App-server fallback and capability checks.
- Protocol optional fields that are planned to become required.
- Legacy exec, shell, and sandbox behavior.
- Client-version gates in TUI and app-server adapters.

Removal criteria should name the client version, schema version, or migration
state that makes the fallback unnecessary.

## Telemetry And Accounting

These TODOs affect observability, token cost, cache behavior, and long-running
automation quality.

- Token usage attribution and compaction accounting.
- Tool-call output sizing and truncation metrics.
- Deferred MCP tool loading and connector cache hit/miss reporting.
- Memory phase-one and phase-two job reporting.
- Multi-agent lifecycle and mailbox wait accounting.

Prefer small counters and status surfaces before policy changes. Observability
should make the next behavior change measurable.

## Flaky Or Ignored Tests

Ignored tests and flaky paths need reproduction notes, not silent cleanup.

- Record the command, platform, expected behavior, and observed failure.
- Note whether the failure is timing, external dependency, filesystem, or UI
  rendering related.
- Convert broad ignored buckets into narrow tests when a stable repro exists.

## Protocol And Schema Cleanup

Protocol TODOs have higher blast radius because they affect generated schema,
SDK clients, app-server docs, and backward compatibility.

- Keep new app-server API work in v2.
- Preserve wire names unless the task is explicitly a breaking API change.
- Regenerate stable and experimental schema fixtures together when experimental
  surface area changes.
- Add tests around behavior, not only schema shape.

## Refactor Debt

Large-module TODOs should be handled as small extraction slices.

- Keep orchestration modules as orchestrators and move cohesive leaf behavior to
  focused modules.
- Move or add tests next to the extracted behavior.
- Avoid mixing behavior changes with mechanical moves.
- Prioritize files called out in `docs/repo-review-improvement-notes.md`:
  `chatwidget.rs`, `chat_composer.rs`, `codex_message_processor.rs`, and
  app-server protocol v2 definitions.

## Current Next Actions

1. Use `docs/generated-artifact-lifecycle.md` for any schema or snapshot change.
2. Tag new TODO cleanup issues with one of the groups above.
3. Start refactor work only when it can be verified as a behavior-preserving
   slice.
