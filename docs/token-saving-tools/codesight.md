# Codesight Token-Saving Research

Source:

- Local clone: `C:\Users\Oleh\Documents\GitHub\agent-context-tools-lab\codesight`
- Upstream: https://github.com/Houseofmvps/codesight
- Local status: cloned and README inspected; not executed in this pass.

## Key Ideas

Codesight builds a deterministic codebase map and optional wiki using AST and
framework detectors. It aims to replace repeated manual exploration with small,
stable context files.

Important mechanisms:

- Zero-dependency Node CLI.
- Multi-language framework, route, ORM, model, component, and environment
  detectors.
- TypeScript gets full AST precision; other languages use detector rules.
- `--wiki` generates `.codesight/wiki/` with index and topic articles.
- `--init` can generate AI guidance files including `AGENTS.md`.
- `--mcp` exposes tools for wiki access and other queries.
- `--blast` shows blast radius for a file.
- `--benchmark` estimates token savings.
- Knowledge mode maps Markdown decision notes into a compact primer.

## How It Works

Codesight parses source and framework-specific shapes into a compact
human-readable map. The wiki layer turns that map into topic articles such as
auth, database, payments, and UI. The intended session pattern is:

1. Read a small wiki index.
2. Pick one article relevant to the task.
3. Read source files only after the index/article narrows the target.

This is close to Graphify's wiki mode, but Codesight emphasizes deterministic
framework extraction and extremely small startup context.

## Evidence From Source Review

The README reports:

- Wiki index around 200 tokens at session start.
- Targeted articles around a few hundred tokens.
- Benchmarks claiming order-of-magnitude reduction versus manual exploration on
  route/model/component questions.
- MCP tools for wiki index/article/lint.
- Knowledge mode for docs, ADRs, meetings, retrospectives, specs, and research.

I did not run Codesight on Codex in this pass. Its Rust support appears oriented
to web frameworks such as Actix and Axum, so Codex's CLI/TUI architecture would
need validation.

## What Codex Should Take

Useful design elements:

- Create a committed or cacheable wiki index as the default cold-start context.
- Keep the index tiny and use topic articles for depth.
- Add deterministic detectors for Codex-specific subsystems:
  CLI commands, TUI widgets, MCP tools, config schema, protocol types, build
  scripts, and tests.
- Add a blast-radius query for changed files during review.
- Add knowledge-mode equivalents for local research docs and incident notes.

## Risks And Gaps

- Framework detectors are only useful if they match the project shape.
- A wiki can become stale unless regenerated after relevant changes.
- Marketing benchmarks are not a substitute for Codex-specific comparisons.
- If Codex reads full `CODESIGHT.md` every session instead of one article, the
  wiki benefit is lost.

## Codex Implementation Candidates

1. Add `.codex/wiki/index.md` generated from a deterministic Codex indexer.
2. Generate subsystem articles with a hard token budget:
   `core-session.md`, `tui.md`, `mcp.md`, `app-server.md`, `build.md`,
   `config.md`, `tests.md`.
3. Add `codex context wiki get <article>` internal tool or MCP resource.
4. Add stale-wiki lint to show source file changes after article generation.
5. Add review support: given a diff, return affected articles and likely test
   lanes.
