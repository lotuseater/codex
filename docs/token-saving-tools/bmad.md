# BMAD Method Token-Saving Research

Source:

- Local clone: `C:\Users\Oleh\Documents\GitHub\agent-context-tools-lab\BMAD-METHOD`
- Upstream: https://github.com/bmad-code-org/BMAD-METHOD
- Local status: cloned and docs/skills inspected; not run as a workflow in this
  pass.

## Key Ideas

BMAD saves tokens less through code indexing and more through workflow
discipline. It turns vague long-running work into explicit artifacts that
become focused context for later phases.

Important mechanisms:

- Structured phases: analysis, planning, solutioning, implementation.
- Each phase produces documents consumed by the next phase.
- `project-context.md` captures stable technical preferences and project rules.
- Story files carry focused implementation context for one unit.
- Distillator skill compresses source documents into dense, LLM-oriented
  distillates.
- Distillation is framed as compression rather than lossy summarization.
- Optional round-trip validation checks whether a distillate preserved source
  facts.

## How It Works

BMAD's workflow map makes context progressive. A PRD informs architecture;
architecture informs stories; a story informs implementation. Instead of
carrying all brainstorming, requirements, architecture, and implementation
history in one chat, each phase emits a smaller artifact with a clear consumer.

The distillator skill is especially relevant. It analyzes source documents,
routes compression into single or fan-out mode, creates dense bullet-form
distillates, verifies completeness using headings/entities, and optionally
round-trips the distillate by reconstructing source content from the distillate
alone.

## Evidence From Source Review

Inspected files include:

- `docs/reference/workflow-map.md`
- `docs/how-to/project-context.md`
- `src/core-skills/bmad-distillator/SKILL.md`

The docs emphasize lean `project-context.md` guidance, using artifacts to carry
decisions, and keeping story context focused on what an implementation agent
needs. The distillator doc provides a concrete multi-stage compression and
validation workflow.

## What Codex Should Take

Useful design elements:

- Explicit "task artifact" creation when a conversation becomes a multi-turn
  research or implementation chain.
- Distillate format for large docs, logs, and sessions: dense bullets,
  provenance, token estimate, and section manifest.
- Consumer-aware compression. A review agent, implementation agent, and resume
  agent need different retained facts.
- Optional validation for high-risk distillates: compare reconstructed claims
  or required facts against originals.
- Lean project context: keep global instructions short and move narrow rules to
  scoped shards.

## Risks And Gaps

- Too much workflow ceremony can cost more than it saves for small fixes.
- LLM-created distillates can omit details without validation.
- Round-trip validation is expensive and should be reserved for high-stakes
  docs or reusable artifacts.
- Planning artifacts must be updated when implementation diverges.

## Codex Implementation Candidates

1. Add a native "conversation distillate" artifact type for completed research,
   log analysis, and codebase exploration chains.
2. Add a prompt policy that replaces old raw research turns with a distillate
   handle plus a short manifest.
3. Add optional consumer profiles:
   `review`, `implementation`, `resume`, `debug`, `planning`.
4. Add completeness metadata:
   source files/logs, headings/entities, changed files, open risks, tests run.
5. Add a cheap validation pass for critical distillates:
   verify that required file paths, commands, findings, and decisions still
   appear in the artifact.
