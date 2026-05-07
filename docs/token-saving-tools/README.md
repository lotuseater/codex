# Token-Saving Tool Research

This folder contains per-tool research notes for external agent-context and
token-reduction systems, plus a combined implementation plan for this Codex
fork.

Docs:

- [Graphify](graphify.md)
- [GSD2](gsd2.md)
- [SR2](sr2.md)
- [Aspens](aspens.md)
- [Codesight](codesight.md)
- [BMAD Method](bmad.md)
- [Aider Repo Map](aider-repomap.md)
- [Serena](serena.md)
- [Repomix](repomix.md)
- [Operation Replacement Study](operation-replacement-study.md)
- [Codex Fork Plan](codex-fork-token-saving-plan.md)

The main design conclusion is that prompt-cache hits and single-action caches
are not enough. Token usage falls only when Codex sends less repeated text to
the model: compact chain artifacts, targeted code maps, scoped instructions,
short handles for large tool outputs, and a fast pre-LLM context scout.
