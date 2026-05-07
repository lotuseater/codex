# Graphify Token-Saving Research

Source:

- Local clone: `C:\Users\Oleh\Documents\GitHub\agent-context-tools-lab\graphify`
- Upstream: https://github.com/safishamsi/graphify
- Local status: installed with `uv tool install`; tested on a Codex source sample.

## Key Ideas

Graphify turns a repo or document set into a persistent knowledge graph. The
agent should ask graph questions before grepping and reading raw files.

Important mechanisms:

- File and document extraction creates nodes, edges, and communities.
- `graphify-out/GRAPH_REPORT.md` gives a small orientation document with
  high-centrality nodes and communities.
- `graphify-out/graph.json` stores the graph for later query without rereading
  the full source corpus.
- Optional `graphify-out/wiki/` provides targeted Markdown articles.
- `graphify query`, `graphify path`, and `graphify explain` return bounded
  graph-derived answers.
- Per-file extraction cache skips unchanged files during rebuild.
- Agent integration installs small instructions and hooks that tell the agent
  to consult the graph before broad search.

## How It Works

Graphify has two extraction modes:

- Deterministic structural extraction for code, using parsers/AST-style
  signals where supported.
- Optional semantic extraction for non-code artifacts, split into chunks and
  cached by file hash.

After extraction, Graphify builds a NetworkX graph, deduplicates entities,
clusters communities, computes central nodes/bridges, and emits report, JSON,
and optional HTML/wiki artifacts. The core token-saving move is not the graph
itself; it is that later sessions read a small graph report or query result
instead of rediscovering the same topology from source.

## Evidence From Local Test

I ran Graphify against a small Codex sample containing selected core/session,
TUI, and MCP files.

Observed result:

- 10 files processed.
- About 16,026 input words represented.
- 241 graph nodes and 449 edges.
- 10 communities.
- 98 percent extraction coverage.
- 0 reported model input/output tokens for this AST-heavy sample.
- `graphify query --budget 900` correctly pointed at
  `core/src/session/checkpoint_policy.rs` for compaction/pre-task context
  narrowing questions.
- `graphify benchmark` reported about 16,066 naive tokens versus about 1,851
  average query tokens, or roughly 8.7x fewer tokens per targeted query.

This is a partial benchmark, not proof for all Codex tasks. It does show that
graph-first lookup can find relevant files with much less prompt text than a
raw explorer sweep.

## What Codex Should Take

Codex should borrow the persistent graph-or-wiki layer, but not necessarily
Graphify's exact Python implementation.

Useful design elements:

- A repo-local context index under a predictable directory, for example
  `.codex/context-index/`.
- A compact report with high-centrality modules, communities, common entry
  points, and "read this first" hints.
- Query API with strict token budgets, returning paths, symbols, and short
  rationale rather than full file text.
- Incremental rebuild keyed by content hash, not mtime.
- Optional wiki articles for stable subsystems such as TUI, core session loop,
  MCP tools, app-server protocol, and release scripts.
- Agent policy: use graph/wiki for architecture questions before `rg --files`
  sweeps.

## Risks And Gaps

- A stale graph is worse than no graph if Codex trusts it blindly.
- Inferred semantic edges need provenance and confidence labels.
- The first build can be expensive on large repos if it uses model extraction.
- Graph output can become another large artifact if the agent reads the whole
  JSON instead of querying it.

## Codex Implementation Candidates

1. Add a native `codex context index` command that builds a deterministic
   symbol/file graph for supported languages and emits a small Markdown report.
2. Add a `context_query` internal tool that returns a token-bounded packet:
   ranked files, symbols, why they matter, and suggested next reads.
3. Teach native first-moves to read the context index first when present.
4. Add freshness metadata and force a fallback to live repo search when indexed
   files changed after the index.
5. Benchmark `context_query` against raw explorer and Graphify on the same
   Codex tasks before making it default.
