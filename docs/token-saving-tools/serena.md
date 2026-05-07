# Serena Token-Saving Research

Source:

- Upstream: https://github.com/oraios/serena
- MCP registry mirror: https://github.com/mcp/oraios/serena
- Local status: cloned after the first pass and source/docs inspected; not run
  against this Codex checkout yet.

## Key Ideas

Serena is a semantic code retrieval and editing toolkit exposed through MCP. It
uses language-server style symbol operations so an agent can navigate by
definitions, references, and symbols instead of reading entire files.

Important mechanisms:

- Symbol-level retrieval.
- Reference and relation queries.
- IDE-like editing/refactoring operations.
- MCP integration so multiple agents can call it.
- Project memories for longer workflows.
- Language-server backend for semantic understanding.

## How It Works

Serena starts a project-aware server, indexes code through language-server
capabilities, and exposes tools for finding symbols, reading symbol bodies,
finding references, and applying edits around symbols. The agent can ask for a
specific symbol or reference set rather than scanning files with grep.

The token-saving principle is targeted semantic retrieval. When the task is
"change this method and all callers", symbol/reference APIs can return a much
smaller and more precise packet than repeated file reads.

## Evidence From Docs

The public docs describe semantic code retrieval, editing, and refactoring
tools, with symbol-level extraction and relational structure. The local clone
shows concrete tool classes for `get_symbols_overview`, `find_symbol`,
`find_referencing_symbols`, `find_implementations`, `find_declaration`,
diagnostics by file/symbol, symbolic edits, `search_for_pattern`, `find_file`,
`list_dir`, and `read_file`.

I did not run Serena against Codex in this pass, and user reports on the web
are mixed. It should be benchmarked against Codex's normal `rg` and
first-moves behavior before adoption.

## What Codex Should Take

Useful design elements:

- Internal semantic tools for:
  find symbol, read symbol, find references, list module symbols, and rename or
  insert near symbol.
- Prefer semantic retrieval for narrow code-edit tasks after the repo/language
  server is warm.
- Return compact Markdown packets, not verbose JSON.
- Include exact file path, symbol range, signature, and reference count.
- Fall back to `rg` when language-server data is stale or unavailable.

## Risks And Gaps

- Starting language servers can be slow and memory-heavy.
- MCP tool-call metadata can add token overhead if responses are verbose.
- Semantic retrieval can underperform for docs, config, generated code, shell
  scripts, and cross-language glue.
- A broken language-server index must not block basic filesystem search.

## Codex Implementation Candidates

1. Add optional LSP-backed internal retrieval for supported languages.
2. Make symbol tools return minimal packets with line ranges and short
   signatures.
3. Integrate symbol references into first-moves ranking.
4. Cache LSP indexes per project and invalidate by file content hash.
5. Benchmark LSP retrieval against `rg` on review, refactor, and bug-fix tasks.
