# Codex Fork Token-Saving Plan

Date: 2026-05-07

Related docs:

- [Broader audit](../token-usage-reduction-broader-audit-2026-05-07.md)
- [Graphify](graphify.md)
- [GSD2](gsd2.md)
- [SR2](sr2.md)
- [Aspens](aspens.md)
- [Codesight](codesight.md)
- [BMAD Method](bmad.md)
- [Aider Repo Map](aider-repomap.md)
- [Serena](serena.md)
- [Repomix](repomix.md)

Additional sources used for replacement strategy:

- Aider repo map: https://aider.chat/docs/repomap.html
- Repomix FAQ and configuration docs: https://repomix.com/guide/faq and
  https://repomix.com/guide/configuration
- Sourcegraph Cody context docs:
  https://sourcegraph.com/docs/cody/core-concepts/context
- Survey-only candidates that still need local Codex validation:
  Continue retrieval/rerank, RepoPrompt codemaps, CodeGraphContext, and SymDex.

## Problem

Weekly token usage is growing faster than elapsed time to reset. The main reason
is not that Codex lacks caches. It is that too much repeated and low-value text
still enters the model:

- broad repo exploration,
- repeated large command outputs,
- repeated session/log discovery,
- monolithic instructions,
- raw history carried after the useful conclusion is already known,
- external tools whose setup or response text costs more than the context they
  save,
- first-moves paths that can hang or run expensive live logic before the model
  starts useful work.

Single-action cached results do not solve this. If a cached shell output is
still pasted back into the conversation, it saves time but not prompt tokens.
The win comes from replacing chains of work with compact artifacts and handles.

## Principles To Borrow

1. Graphify/Codesight: build persistent repo maps and topic indexes so cold
   sessions start with a small map, not a raw exploration sweep.
2. Aider: keep a strict token-budgeted repo map with signatures and selected
   anchors, not full file bodies.
3. Serena: use symbol/reference retrieval for narrow code edits after a
   semantic index is warm.
4. GSD2: store noisy command results outside context; inject digest plus handle.
5. SR2: compile prompt layers deliberately, preserve stable prefixes, and manage
   raw/compacted/summarized history zones.
6. Aspens: split global instructions into small scoped shards loaded by path and
   task.
7. BMAD: turn long research/planning chains into durable distillates consumed by
   later phases.
8. Repomix: expose token trees and compressed snapshots, but avoid making a full
   packed repo file the default prompt.
9. Sourcegraph Cody/Continue: combine keyword search, code graph, embeddings,
   and reranking instead of relying on one retrieval signal.
10. RepoPrompt/CodeGraphContext/SymDex: use codemaps or symbol graphs to answer
   "where is this?" and "what calls this?" without reading whole files. Treat
   these as design inputs until local Codex benchmarks prove value.

## Priority 0: Stop Known Waste

### 0.1 Prefer Native First-Moves Over External Wizard First-Moves

Observed issue: an external Wizard MCP call to `first_moves_predict` ran for
more than 11 minutes during review startup. The native Codex first-moves path is
already internalized and bounded; the external Wizard path can do embeddings,
topic shards, live logic, lexical fallback, and broad scans without a hard outer
timeout.

Change:

- Do not directly bootstrap external `first_moves_predict` and
  `first_moves_stats` when native first-moves is enabled.
- Keep `first_moves_logic_advice` discoverable only for explicit deep-analysis
  cases, not as default startup.
- Add a hard wall-clock timeout around any pre-LLM context scout. If it misses
  the budget, fail open with no context rather than blocking the turn.
- Review prompt default should use cheap mode:
  no live logic, no embedding auto-spawn, no topic shards unless explicitly
  requested.

Expected token/time effect:

- Removes long idle startup and prevents the agent from spending a large share
  of the session before useful reasoning starts.

### 0.2 Internalize Fast Session And Log Discovery

Prototype already added:

- `scripts/find-codex-sessions.ps1`

Current prototype status:

- Handles current `session_meta.payload` JSONL records.
- Reads `state_5.sqlite` through `sqlite3` first, so recent threads can be
  found by indexed `cwd`/`updated_at` metadata before scanning JSONL files.
- Returns `tokens_used` from indexed sessions, so session discovery can also
  identify the conversations spending the most tokens without reading full logs.
- Adds repo skill `.codex/skills/codex-session-discovery/SKILL.md` so future
  Codex sessions start from the indexed session path and DAB/live-terminal
  checks instead of broad JSONL scans.
- Scans known recent date folders and recently modified older session files, so
  active older sessions are not missed.
- Uses bounded head/tail reads and returns only recent summaries unless full
  logs are explicitly requested.
- Current `C:\Users\Oleh\.codex\config.toml` exposes the Wizard Codex MCP
  bridge, but the allowed tool list does not expose DAB/window-navigation tools
  to this session. Native Codex DAB should be preferred when the next build
  internalizes it; until then, the launcher config should expose only the small
  DAB discovery/read tools needed for live terminal lookup.

Change:

- Port the script's strategy into Codex native session tooling:
  known sessions root, per-date directories, recent timestamp sort, bounded
  metadata parse, optional live process/window lookup.
- Prefer DAB/native session APIs for live PowerShell/window navigation when
  available.
- Never full-scan session JSONL unless the user asks for deep forensics.
- For each discovered session, return project root, session id, modified time,
  first/last user clues, and artifact handles for full logs.
- Add a native recent-session index or DAB/live-session registry so this path is
  milliseconds in the common case; keep the script-style timestamp scan as a
  correctness fallback.
- Add a startup self-check that reports whether native DAB or Wizard DAB session
  lookup is available, so Codex does not silently fall back to slow filesystem
  discovery for live PowerShells.

Expected token/time effect:

- Session/log discovery should be milliseconds to seconds and should not spend
  15 percent or more of a conversation on locating files.

## Priority 1: Artifact-Backed Chain Cache

This is the highest-leverage cache change.

Build a durable artifact store for large tool chains:

- shell outputs,
- build logs,
- test logs,
- session JSONL excerpts,
- large file reads,
- repeated `rg` result sets,
- graph/wiki/repo-map outputs,
- research distillates.

Prompt representation:

```text
Artifact a1b2c3: cargo release build log
status: failed
digest: linker OOM after compiling codex-tui; full log stored
path: logs/fast-release-build-20260507-013000.log
read: artifact_read(a1b2c3, lines=...)
search: artifact_search(a1b2c3, "error")
```

Rules:

- Store full output once.
- Inject only digest, metadata, and a stable handle.
- Reopen by handle only when the model needs detail.
- Reuse previous artifacts when command, cwd, env fingerprint, inputs, and
  relevant file hashes match.
- Let users inspect artifacts on disk.

Implementation targets:

- `codex-rs/core/src/session/` for history compaction and artifact references.
- `codex-rs/core/src/tools/` for artifact read/search/list handlers.
- TUI rendering for compact artifact cards.
- Telemetry for original tokens, injected tokens, and saved tokens.

## Priority 2: Prompt Layer Compiler

Add an explicit context compilation step inspired by SR2.

Layers:

1. Stable system/developer instructions.
2. Project root identity and active config.
3. Selected scoped AGENTS shards.
4. Durable memory and repo-context index summaries.
5. Current task packet.
6. Current diff/changed files summary.
7. Recent raw conversation.
8. Compacted older history with artifact handles.
9. Latest volatile tool results.

For each layer record:

- token estimate,
- cache key/fingerprint,
- whether it changed this turn,
- truncation decision,
- compaction decision,
- artifact handles produced.

Add a `context_plan` debug view or log file. The goal is to make token burn
visible. A weekly usage increase should be traceable to layers, tools, and
tasks.

## Priority 3: Repo Context Scout Before First Model Work

Build a fast, bounded context scout that runs before the LLM starts broad
exploration.

Inputs:

- user prompt,
- cwd/project root,
- changed files,
- native first-moves DB,
- repo map / wiki / graph index if present,
- scoped instruction shards,
- recent successful first reads for similar prompts.

Output budget:

- default: 300-900 tokens,
- review/debug: up to 1,200 tokens if it includes changed-file/test hints,
- hard timeout: 1-2 seconds for default path.

Output shape:

```text
Context Scout:
- likely files: path, reason, confidence
- relevant symbols/articles: name, path/article, reason
- suggested first reads: exact commands or internal reads
- avoid: broad scans, stale external first_moves, known expensive lanes
```

Do not let the scout call expensive live logic by default. It is a narrowing
step, not a second agent.

## Priority 4: Repo Map, Wiki, And Symbol Retrieval

Implement in phases:

1. Rust-aware repo map:
   crate/module tree, public types, functions, tests, config structs, MCP
   tools, TUI widgets, app-server protocol types.
2. Topic wiki:
   tiny index plus subsystem articles with hard token budgets.
3. Graph edges:
   imports, calls where cheap, config/schema relations, test-to-code links,
   command-to-script links.
4. Optional LSP-backed symbol retrieval for exact symbol/reference tasks.

Context policy:

- Cold start reads tiny index or scout output.
- Task reads one article or map slice.
- Full file reads happen after the map narrows targets.

## Priority 5: Scoped Instructions

Split large always-on guidance into shards.

Initial shard candidates:

- `core-session`
- `tui`
- `mcp-tools`
- `app-server`
- `release-build-windows`
- `tests-snapshots`
- `docs-research`
- `launcher-integration`

Each shard should include:

- activation path globs,
- key files,
- local conventions,
- hard prohibitions,
- verification lane,
- max token budget,
- freshness fingerprint.

Codex should load shards by deterministic path/task matching, then expose "why
loaded" in prompt telemetry.

## Priority 6: Distillates For Long Conversations

When a chain exceeds a threshold or produces reusable knowledge, create a
distillate artifact:

- sources inspected,
- decisions,
- facts,
- open questions,
- changed files,
- tests/builds run,
- failures and causes,
- artifact handles,
- next verified action.

Replace older raw chain context with the distillate handle and keep only recent
raw turns. This directly addresses the user's point: caching chain actions is
valuable only if the chain result replaces bulky prior context.

Use consumer profiles:

- review distillate,
- implementation distillate,
- resume distillate,
- debugging distillate,
- research distillate.

## Priority 7: Token And Quality Benchmark Harness

Create a benchmark harness before making retrieval defaults permanent.

Tasks:

1. Review current Codex changes.
2. Find why first-moves startup hung.
3. Locate prompt elision/autocompaction code.
4. Discover latest sessions/logs for two repo roots.
5. Find files affected by a TUI snapshot change.

Lanes:

- raw explorer (`rg`, direct reads),
- native first-moves,
- native first-moves plus repo map,
- Graphify query,
- Codesight/wiki if generated,
- LSP/symbol retrieval when available,
- artifact-backed command reuse.

Metrics:

- wall time,
- number of tool calls,
- bytes returned,
- estimated prompt tokens,
- files/symbols found,
- missed critical files,
- final answer quality,
- cache read/write tokens,
- artifact tokens saved.

Quality gates:

- A token-saving lane must not reduce correctness on review/debug tasks.
- A context tool that saves tokens but hides evidence should stay opt-in.
- Tool output must be compact Markdown or typed structs, not verbose JSON dumps.
- Every replacement candidate must be tested against the current/raw behavior
  before it can be used by default.

## Priority 7.1: Shadow, Benchmark, Promote

This is the migration gate for replacing common Codex operations. It must land
before automatic shell substitution.

Rollout modes:

- `off`: current behavior only.
- `recommend`: classify the command and log a better operation, but do not run
  it.
- `shadow`: run the replacement beside the current operation and compare
  results while keeping the current output model-visible.
- `canary`: use the replacement for low-risk read-only cases and fall back on
  mismatch, stale index, low confidence, or explicit raw-output requests.
- `default`: use the replacement after golden tasks pass.

Add a `replacement_bench` report for every candidate run:

- baseline command/tool,
- replacement operation and version,
- baseline model-visible tokens,
- replacement model-visible tokens,
- full-output artifact bytes,
- wall time,
- required facts found or missed,
- fallback reason,
- verdict: `pass`, `fail_quality`, `fail_tokens`, `fallback_required`, or
  `needs_human_review`.

Promotion rule:

- no quality regression on review/debug golden tasks,
- exact detail recoverable by artifact handle or raw fallback,
- at least 30 percent fewer model-visible tokens for discovery/output-heavy
  operations, or a clear latency win for live session/process discovery,
- deterministic fallback when the candidate cannot prove equivalence.

Initial prototype:

- `scripts/measure-operation-replacements.ps1` benchmarks `GitSummary`,
  `SessionFind`, `SearchText`, `FileOutline`, and `RunCheck`.
- Current sample results: `git_worktree_summary` passed with about 99.8 percent
  token savings; capped `search_text` passed with about 48.8 percent savings on
  `first_moves`; `session_find` passed quality after metadata/timestamp fixes
  and now uses `state_5.sqlite` before filesystem fallback; `file_outline`
  passed on `shell.rs` with about 86.1 percent savings and on huge
  `chatwidget.rs` with about 94.7 percent savings when the cap covered all
  detected definitions; `run_check_digest` passed with about 98.1 percent
  savings by storing full output as an artifact and returning diagnostics plus a
  path.
- Promotion decision from the sample: move `git_worktree_summary` and
  `search_text` toward native shadow mode first; move `file_outline` into the
  same shadow lane with omitted-definition counting; move `run_check_digest`
  into the artifact-backed output lane; port `session_find` to the native thread
  store or DAB lookup so it avoids PowerShell/sqlite startup and only falls back
  to JSONL scans when indexed state is missing.
- Canary stage 1 is the opt-in `features.context_ops` native tool set:
  `file_outline`, `git_worktree_summary`, and `search_text`. It is read-only,
  disabled by default, and intended for measured use before any automatic shell
  replacement.
- Shadow stage 1 is `features.context_ops_shadow`, default-enabled in this
  fork. It keeps normal shell output model-visible, classifies exact read-only
  shell discovery commands, runs the compact candidate in the background, and
  writes `replacement_bench` JSONL plus artifacts under
  `<codex_log_dir>/replacement-shadow/`.

## Priority 8: Replace Common Shell Operations With Better Primitives

See [Operation Replacement Study](operation-replacement-study.md).

Recent local sessions show that many token-heavy turns are not "reasoning"
turns. They are repeated shell-mediated file reads, `rg` searches, recursive
listing, `git status`/`git diff`, process inspection, and session-log discovery.
These should become typed, budgeted Codex operations.

Initial replacements:

- `git status`, `git diff --stat`, `git diff --name-only` ->
  `git_worktree_summary` and `changed_files`.
- `Get-Content`/`cat` whole files -> `read_file_slice`, `file_outline`, and
  later `read_symbol`.
- broad `rg` -> capped `search_text`, symbol search, reference search, or
  graph/wiki query depending on task.
- recursive `Get-ChildItem`/`rg --files` -> indexed `find_file`/fuzzy file
  query.
- process/session PowerShell scans -> native `process_find`, `session_find`,
  and `session_tail`.
- build/test commands -> `run_check` that stores full logs as artifacts and
  returns failure summaries.

Adopt in four stages:

1. Record and recommend replacements, without changing behavior.
2. Shadow-run candidates beside the current behavior and write
   `replacement_bench` records.
3. Canary only exact read-only command patterns with tested equivalence and
   artifact/raw fallback.
4. Promote to default only after golden-task quality and token gates pass.

## Patch Sequence

Recommended order after the other active build/code session is done:

1. Finish and canary the read-only `features.context_ops` native tools:
   `file_outline`, `git_worktree_summary`, and `search_text`.
2. Ship default-on observe-only `features.context_ops_shadow` for exact safe
   shell patterns (`git status`/`git diff --stat`, capped `rg`, and whole-file
   reads) so the next build collects replacement benchmarks without changing
   model-visible behavior.
3. Patch MCP tool exposure so external Wizard first-moves is not direct-default
   when native first-moves is available.
4. Add hard timeout/fail-open around pre-LLM scouts and any external context
   provider.
5. Add replacement classifier telemetry and the `replacement_bench` report
   schema. This only observes and recommends; it does not change behavior.
6. Add benchmark fixtures for raw shell/current behavior versus candidate
   replacements on the five golden tasks.
7. Add artifact store types and artifact read/search/list internal tools.
8. Extend prompt elision to replace large older tool outputs with artifact
   handles.
9. Add context-plan telemetry with per-layer token estimates.
10. Add native fast session/log discovery command based on
   `scripts/find-codex-sessions.ps1`.
11. Add repo-map prototype for Rust symbols and changed-file review hints.
12. Add scoped instruction shard loader.
13. Add typed replacement operations for git summary, file index/search,
    session discovery, and capped file slices in `recommend` or `shadow` mode.
14. Canary only candidates whose `replacement_bench` records pass the quality
    and token gates.

## Success Criteria

- Fresh review/debug tasks spend less time and fewer tokens on discovery.
- Session/log discovery no longer burns more than a few seconds or a small
  prompt packet.
- Large command outputs appear once as artifacts, then as compact handles.
- Prompt telemetry shows which layers dominate token usage.
- Replacement telemetry proves which operations saved tokens and which were
  rejected for quality.
- Weekly token growth tracks actual productive work more closely than elapsed
  context accumulation.
- All defaults fail open and preserve correctness over token savings.
