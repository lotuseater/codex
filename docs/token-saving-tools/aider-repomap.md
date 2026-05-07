# Aider Repo Map Token-Saving Research

Source:

- Official docs: https://aider.chat/docs/repomap.html
- Upstream: https://github.com/Aider-AI/aider
- Local status: web/docs reviewed; repo not cloned in this pass.

## Key Ideas

Aider's repo map is one of the clearest examples of budgeted codebase context.
It sends a compact map of the repository with important symbols, signatures,
and selected definition lines. For large repos, it ranks and truncates the map
to fit a token budget.

Important mechanisms:

- Symbol map of files, classes, methods, and functions.
- Dependency/reference graph where files are nodes.
- Graph ranking to pick the most relevant parts of the map.
- Dynamic token budget controlled by map-token settings and chat state.
- The map expands when no files are already in context and shrinks when the
  active files provide enough grounding.

## How It Works

Aider builds a repository-level symbol graph and selects relevant map entries
for the current chat. The model does not see full files by default. It sees
enough signatures and definition snippets to know which files to request next
and how current files relate to the rest of the repo.

The important token-saving move is "elided code with retrieval affordances":
the model receives precise names, signatures, and file locations, not whole
implementation bodies.

## Evidence From Docs

The official documentation describes:

- A concise map of the whole git repository.
- Key symbols and critical definition lines per file.
- Graph ranking over file dependencies.
- Selection of the most relevant portions that fit in a token budget.
- Dynamic expansion when the chat lacks added files.

I did not benchmark Aider on this Codex checkout in this pass.

## What Codex Should Take

Useful design elements:

- A native symbol map for Rust crates and TypeScript/JavaScript support files.
- Personalized ranking from the current user task, open files, changed files,
  and conversation summary.
- Prompt output as elided code views:
  file path, symbol name, signature, doc comment, and a few anchor lines.
- Strict map-token budget with visible telemetry.
- Dynamic map size:
  larger at cold start, smaller after files are selected.

## Risks And Gaps

- Symbol maps miss behavior hidden in macro expansion, config files, tests, or
  build scripts.
- Graph ranking can over-select central files and under-select rare edge cases.
- Rust-specific symbol extraction needs to respect crates/modules and public
  API boundaries.

## Codex Implementation Candidates

1. Extend native first-moves with a Rust-aware repo map.
2. Add a `repo_map` prompt layer capped by `map_tokens`.
3. Include only signatures/doc comments/anchor lines by default.
4. Personalize ranking with:
   prompt terms, changed files, AGENTS shards, previous successful first-moves,
   and optional graph/wiki communities.
5. Add tests that assert the rendered repo map stays under budget and includes
   expected symbols for known tasks.
