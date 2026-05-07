# Aspens Token-Saving Research

Source:

- Local clone: `C:\Users\Oleh\Documents\GitHub\agent-context-tools-lab\aspens`
- Upstream: https://github.com/aspenkit/aspens
- Local status: cloned and README inspected; not executed in this pass.

## Key Ideas

Aspens attacks instruction bloat. Instead of one large `CLAUDE.md` or
`AGENTS.md`, it generates short scoped skill files from the repo import graph.
Each skill activates only for a relevant domain.

Important mechanisms:

- Generate skills of roughly 35 lines each.
- Discover feature domains from import graph and repo structure.
- Emit target-specific files for Claude, Codex, or both.
- `doc impact` checks freshness, domain coverage, and whether important hub
  files are surfaced.
- `doc sync` maps recent git changes to affected skills and updates only those.
- Optional post-commit hook keeps generated context in sync.

## How It Works

Aspens scans the codebase, detects architecture and feature domains, and writes
small markdown skill files with activation patterns, key files, concepts, and
critical rules. Instead of loading a full monolithic instruction file on every
task, the agent loads the base guidance plus a domain-specific slice.

The token-saving principle is scoped instruction activation: most tasks need a
few local conventions, not every convention in the repository.

## Evidence From Source Review

The README gives a Codex target that emits `AGENTS.md`, `.agents/skills`, and
directory `AGENTS.md` files. It describes:

- Import graph analysis.
- Feature-domain generation.
- Stale context checks.
- Commit-driven sync for affected skills.
- Before/after behavior where agents avoid 10-20 exploratory tool calls because
  key local files and rules are already surfaced.

I did not run `aspens doc init` on Codex in this pass because the repo already
has significant manual AGENTS and skill structure, and this turn is limited to
research docs.

## What Codex Should Take

Useful design elements:

- Split project instructions into scoped shards:
  `tui`, `core-session`, `mcp-tools`, `app-server`, `release-build`,
  `windows-launcher`, `docs`.
- Load only shards matching changed/read paths plus the user task.
- Keep a short root `AGENTS.md` that points to shards instead of duplicating all
  details.
- Add freshness checks for instruction shards: source files changed after shard
  generation, missing key files, too-long shard.
- Add a "why this shard loaded" trace to prevent hidden context bloat.

## Risks And Gaps

- Generated skills can encode wrong conventions if the import graph is shallow.
- Too many tiny shards can increase activation overhead.
- A stale shard can cause worse behavior than no shard.
- Codex needs deterministic path-to-shard matching; relying only on LLM skill
  activation may miss critical rules.

## Codex Implementation Candidates

1. Add native support for `AGENTS.d/*.md` or `.agents/context-shards/*.md`.
2. Add an instruction-shard selector before prompt assembly.
3. Enforce shard budgets and warn when a shard grows beyond a configured limit.
4. Add a `codex context impact` command that reports stale shards, missing
   coverage, and high-touch files without local guidance.
5. Make first-moves use shard metadata as another routing signal.
