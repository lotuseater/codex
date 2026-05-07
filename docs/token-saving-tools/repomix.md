# Repomix Token-Saving Research

Source:

- Official docs: https://repomix.com/
- FAQ: https://repomix.com/guide/faq
- Upstream: https://github.com/yamadashy/repomix
- Local status: web/docs reviewed; repo not cloned or run in this pass.

## Key Ideas

Repomix packs a repository into an AI-friendly output file with token counts,
ignore rules, security checks, and optional compression. It is useful for
one-shot context packaging, snapshots, and offline analysis.

Important mechanisms:

- Output formats: XML, Markdown, JSON, plain text.
- Respects ignore patterns.
- Token count reporting.
- Secret scanning through Secretlint.
- Tree-sitter-based compression that preserves high-level code structure such
  as imports, exports, classes, functions, interfaces, and signatures while
  dropping implementation detail.
- Remote repository packing.
- MCP/automation integrations in newer versions.

## How It Works

Repomix walks a repository, filters files, optionally compresses code with
Tree-sitter, and emits a single structured artifact. Unlike Graphify or
Codesight, it does not primarily answer targeted graph/wiki queries. Its main
value is producing a bounded, auditable snapshot that a model or another tool
can consume.

For Codex, Repomix is most relevant as an artifact format and benchmark
baseline, not as a default in-conversation retrieval layer.

## Evidence From Docs

The official docs describe packing a codebase into an AI-friendly file, token
counting, `--compress`, and Tree-sitter based compression. The FAQ explains
that compression keeps important structure while removing implementation
details.

I did not run Repomix on Codex in this pass.

## What Codex Should Take

Useful design elements:

- Token-count tree for selected context, so the agent sees where context budget
  is spent.
- Secret scanning before exporting large context snapshots.
- Tree-sitter compression as a cheap preview mode for files not yet selected
  for full read.
- Snapshot artifacts for handoff/resume:
  "this was the exact selected context for task X".
- Include/ignore profiles by task type.

## Risks And Gaps

- A whole-repo packed file can be huge; if Codex reads it all, token usage gets
  worse.
- Compression may remove implementation details needed for bug fixes.
- Single-file snapshots can go stale quickly.
- The one-way packed artifact is less interactive than graph/wiki/symbol query
  systems.

## Codex Implementation Candidates

1. Add a `context_snapshot` artifact that records selected files, compressed
   previews, token counts, and secret-scan status.
2. Add a Tree-sitter/Rust-syn preview renderer:
   imports, public types, function signatures, doc comments, and test names.
3. Use snapshots for resume and review baselines, not as mandatory prompt
   content.
4. Add a "token tree" view to prompt telemetry, grouped by layer and file.
5. Benchmark snapshot compression against raw selected-file reads.
