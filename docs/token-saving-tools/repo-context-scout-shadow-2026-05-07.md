# Repo Context Scout Shadow Prototype

Date: 2026-05-07

## Scope

This note studies a context-narrowing stage for fresh Codex starts, parallel
instances, `/clear`, resume, and post-compaction work.

The target is not a larger automatic repo dump. The target is a small
task-scored packet that tells the next agent which files, changed areas,
symbols, artifacts, and follow-up tools are worth using first.

The first prototype is implemented as `scripts/context-scout-map.ps1`. The
current Rust integration adds the same idea as a separate
`codex-repo-context-scout` crate with shadow-only fresh-turn recording by
default and an opt-in `repo_context_scout` tool mode. It still does not inject
scout packets into normal model context automatically.

## Requirements

A useful cold-start context stage should:

- stay bounded under a hard token budget;
- expose path-scored first reads, not only prose;
- surface dirty, staged, and untracked files even when the durable index is
  stale;
- include enough anchors to choose narrow reads or symbol tools;
- separate direct model-visible context from artifact handles and tool routes;
- work after compaction or resume, where the next model may not know previous
  exploration;
- remain shadow-only until first-read hit data proves that it improves quality.

The biggest design correction from the benchmark is that external tools should
not all be scored as if they were direct scout packets. Some are better as
support prompts:

- GSD2: command-output memory and artifact-backed exploration handles.
- Graphify: topology and relation hints.
- Repomix: scoped snapshot artifact plus token accounting.
- Serena: semantic lookup route for symbol overview and references.

## Sources Inspected

Local notes:

- `docs/token-saving-tools/aider-repomap.md`
- `docs/token-saving-tools/graphify.md`
- `docs/token-saving-tools/gsd2.md`
- `docs/token-saving-tools/repomix.md`
- `docs/token-saving-tools/serena.md`
- `docs/token-saving-tools/operation-replacement-study.md`
- `docs/token-usage-reduction-broader-audit-2026-05-07.md`

Prototype and benchmark artifacts:

- `scripts/context-scout-map.ps1`
- `logs/context-scout-bench-codex.json`
- `logs/context-scout-bench-donutgame.json`
- `logs/context-scout-bench-serial.json`
- `logs/context-scout-bench-synthetic-stale.json`
- `logs/context-scout-bench-codex-role-aware.json`

External tools and docs:

- Aider repo map: https://aider.chat/docs/repomap.html
- Graphify: https://github.com/safishamsi/graphify
- GSD2: https://github.com/gsd-build/gsd-2
- Repomix: https://repomix.com/ and https://github.com/yamadashy/repomix
- Serena: https://github.com/oraios/serena

## Tool Roles

### First-Moves Predictor

First-moves is the best existing native opening hint. It uses historical repo
reads, changed files, topic shards, and learned path weights. In the Codex
benchmark it found relevant changed `context_ops` files, but also included
baseline docs and unrelated files from repo guidance. In DonutGame it found key
rendering files but missed some changed tests. In Serial it outperformed the
scout for a broad native-code prompt.

The gap is representation. First-moves returns candidates and shell hints. It
does not maintain a durable repo context index, emit a single packet with
freshness warnings, or overlay files added after index generation.

Best use: keep it as a learned prior and compare scout-selected paths against
its first-read hits.

### Aider Repo Map

Aider's repo map is the closest proven direct-packet shape: ranked file and
symbol context inside a token budget. The useful idea for Codex is not a
whole-repo summary. It is path plus symbol plus line anchor plus reason.

Best use: borrow the representation style for `repo_context_scout` packets.

### GSD2

GSD2 is exposed on this machine through `gsd_exec`, `gsd_exec_search`, and
`gsd_resume`. The initial benchmark output looked tiny: around 155 to 165
estimated tokens on Codex, DonutGame, and Serial. That number was misleading
when treated as a direct context selector, because the visible output is mostly
a handle plus a small digest.

The better representation is `gsd2_artifact_exploration_prompt`: include the
`gsd_exec` id, stdout artifact path, a short visible digest, parsed path hints,
and explicit instructions to inspect or search the artifact before rerunning
broad exploration.

Observed Codex role-aware result:

| Record | Tokens | Changed paths represented | Verdict |
| --- | ---: | ---: | --- |
| `gsd_exec_raw_exploration_digest` | 157 | 0 / 17 | `needs_more_data` |
| `gsd2_artifact_exploration_prompt` | 630 | 15 / 17 | `pass_support_prompt` |

Best use: artifact-backed exploration memory and resume handles, not standalone
file ranking.

### Graphify

Graphify produced path-bearing topology output, but the old harness failed to
parse `src=... loc=...` path hints and therefore counted it as pathless.
After parsing those paths, it becomes useful as a topology aid.

The direct query can still be larger than the raw compact baseline on Codex.
Its value is relation structure: which symbols are connected, which source
files contain them, and whether `graphify explain` or `graphify path` should be
used before another broad `rg`.

Observed Codex role-aware result:

| Record | Tokens | Changed paths represented | Verdict |
| --- | ---: | ---: | --- |
| `graphify_query_sample` | 1,565 | 8 / 17 | `needs_more_data` |
| `graphify_topology_prompt` | 1,335 | 8 / 17 | `pass_support_prompt` |

Best use: graph topology prompt for architecture and cross-file flow questions.
Do not make it the default cold-start selector until update cost and stale
graph handling are measured.

### Repomix

Repomix is useful for auditable snapshots and token accounting. Injecting the
compressed artifact directly is often too expensive:

- Codex selected-path artifact: 37,436 tokens in the earlier benchmark and
  43,153 tokens in the role-aware benchmark.
- DonutGame selected-path artifact: 1,596 tokens.
- Serial selected-path artifact: 20,144 tokens.

The better representation is `repomix_artifact_context_prompt`: include the
artifact path, estimated artifact tokens, selected paths, and instructions to
read only specific sections or source files.

Observed Codex role-aware result:

| Record | Tokens | Changed paths represented | Verdict |
| --- | ---: | ---: | --- |
| `repomix_compressed_selected` | 43,153 | 12 / 17 | `fail_tokens` |
| `repomix_artifact_context_prompt` | 348 | 12 / 17 | `pass_support_prompt` |

Best use: handoff artifact for reviews, parallel agents, and reproducible
context packs. It should not be default prompt content except on small projects
or tightly scoped file sets.

### Serena

The benchmark only exercised Serena's CLI tool catalog, which is generic and
not a fair semantic retrieval test. It proved availability of useful tool
names, not task-specific quality.

The better representation is `serena_semantic_lookup_prompt`: include the top
scout paths, prompt-derived symbol query terms, and an explicit route:
activate project, run `get_symbols_overview` on candidate files, then use
`find_symbol` and `find_referencing_symbols` before broad whole-file reads.

Observed Codex role-aware result:

| Record | Tokens | Changed paths represented | Verdict |
| --- | ---: | ---: | --- |
| `serena_tool_catalog` | 1,296 | 0 / 17 | `needs_more_data` |
| `serena_semantic_lookup_prompt` | 345 | 12 / 17 | `pass_support_prompt` |

Best use: semantic lookup route after candidate paths exist. A separate MCP
symbol benchmark is still needed before claiming quality.

## Prototype Implemented

`scripts/context-scout-map.ps1` supports:

- `Build`: create a repo context index under `logs/context-scout-shadow/<repo>`.
- `Scout`: emit task-scored context packets.
- `Bench`: compare scout variants, raw exploration, and external tool outputs.
- `Status`: report index and changed-path state.

Index contents:

- repo key, git HEAD, schema, timestamp, and tool availability;
- file inventory from `rg --files`, excluding `.git`, `target`,
  `node_modules`, logs, caches, `.gsd`, and generated graph outputs;
- language, size, line count, mtime, and lightweight anchors;
- directory and language rollups.

Direct scout variants:

- `inventory_anchor_scout`
- `changed_area_scout`
- `topic_catalog_scout`
- `symbol_graph_lite_scout`
- `session_memory_scout`
- `hybrid_ranked_scout`

Support-prompt variants:

- `gsd2_artifact_exploration_prompt`
- `graphify_topology_prompt`
- `repomix_artifact_context_prompt`
- `serena_semantic_lookup_prompt`

Important behavior:

- Current changed files are overlaid into the index at scout time, so stale
  indexes still surface new or modified files.
- Output defaults to a 2,000 token budget; `-MaxOutputTokens 0` disables the
  cap for research.
- Null JSON anchors, missing candidate paths, `.ps1` tool shims, and failed
  external tools are handled explicitly.
- Support prompts receive a separate `usage_role` and are not treated as direct
  scout-packet winners.

## Benchmark Results

All token counts are approximate, using the script's current `chars / 4`
estimator.

Earlier cross-repo direct-scout benchmark:

| Repo | Raw baseline | Best direct scout | Scout tokens | Savings | Changed paths represented |
| --- | ---: | --- | ---: | ---: | ---: |
| Codex | 1,430 | `changed_area_scout` | 997 | 30.3% | 12 / 16 |
| DonutGame | 1,342 | `topic_catalog_scout` | 734 | 45.3% | 12 / 15 |
| Serial_to_Google_Doc_topdown | 895 | `hybrid_ranked_scout` | 887 | 0.9% | 0 / 0 |
| Synthetic stale index | 56 | `hybrid_ranked_scout` | 343 | 0.0% | 2 / 2 |

The synthetic stale-index case is intentionally tiny, so token savings are not
the metric. The important result is correctness: after building the index, the
test added `src/gamma_context.rs` and modified `src/beta.rs`; the scout overlaid
both files and ranked them first.

Role-aware Codex benchmark:

| Record | Tokens | Savings vs raw | Changed paths represented | Verdict |
| --- | ---: | ---: | ---: | --- |
| `symbol_graph_lite_scout` | 1,007 | 31.1% | 12 / 17 | `needs_more_data` |
| `hybrid_ranked_scout` | 1,012 | 30.8% | 12 / 17 | `needs_more_data` |
| `changed_area_scout` | 1,019 | 30.3% | 12 / 17 | `needs_more_data` |
| `gsd2_artifact_exploration_prompt` | 630 | 56.9% | 15 / 17 | `pass_support_prompt` |
| `graphify_topology_prompt` | 1,335 | 8.7% | 8 / 17 | `pass_support_prompt` |
| `repomix_artifact_context_prompt` | 348 | 76.2% | 12 / 17 | `pass_support_prompt` |
| `serena_semantic_lookup_prompt` | 345 | 76.4% | 12 / 17 | `pass_support_prompt` |

Interpretation:

- Direct scouts are useful but do not yet meet a 50% savings threshold on
  Codex. They are still the best candidate for automatic model-visible context
  because they provide path and anchor data directly.
- Support prompts can be much cheaper and better tailored than raw external
  output. They should be offered as instructions and handles, not as automatic
  replacement for direct scouting.
- `session_memory_scout` is too sparse on this repo state: it saved tokens but
  represented zero changed paths, so it failed quality.

## Selection

The best current implementation shape is:

1. Run the direct `repo_context_scout` packet with stale-index changed overlay.
2. Bias toward changed-area scoring when the repo is dirty and the prompt is
   review, build, test, or fix oriented.
3. Include optional support prompts only when the corresponding tool output
   exists and stays under budget:
   - GSD2 for prior exploration artifacts and broad command memory.
   - Graphify for relation and topology questions.
   - Repomix for auditable handoff snapshots.
   - Serena for semantic symbol lookup.
4. Keep native first-moves as a learned prior and benchmark both against actual
   subsequent reads.

Do not inject a Repomix pack or Graphify query directly by default. Treat them
as artifact-backed routes unless a task explicitly asks for a full snapshot or
graph reasoning.

## Rust Integration

The implemented native shape is intentionally split from `first_moves`:

- `first_moves` remains the fast learned prior for initial file reads.
- `codex-repo-context-scout` owns the durable repo index, changed overlay,
  prompt ranking, bounded packet formatting, support-route hints, and shadow
  records.
- `codex-core` only wires the fresh-turn shadow call and optional tool handler.
- Tool mode is opt-in through `[repo_context_scout].mode = "tool"`; the default
  mode is `shadow`.
- Shadow records are written under Codex home in `context-scout/<repo-key>/`.

The scout skips remote, restricted-filesystem, and external-sandbox turns so
local indexing cannot bypass the turn environment. It includes hidden config
files, excludes generated/cache directories, and expands untracked directories
through Git's untracked file list before ranking changed areas.

## Shadow Integration Plan

The next telemetry step is to extend the native shadow-only stage:

- run after `/clear`, resume, and compaction in addition to fresh task setup;
- record prompt hash, repo key, index age, changed-file overlay count,
  selected paths, support-prompt roles, approximate tokens, and first-read hits;
- compare direct scout paths, first-moves paths, and actual subsequent reads;
- record external artifact handles without injecting large artifacts;
- require path recall and real first-read hits before promotion.

Recommended record fields:

- `prompt_hash`
- `repo_key`
- `index_state`: `cold`, `warm`, or `stale`
- `direct_scout_tokens`
- `direct_scout_paths`
- `support_prompts`: list of role, tokens, artifact path, and path hints
- `first_moves_paths`
- `actual_read_paths`
- `changed_paths_represented`
- `fallback_reason`
- `verdict`

## Open Work

- Add real MCP-level Serena symbol benchmarks instead of only CLI catalog
  availability.
- Measure Graphify update cost and stale-graph behavior on large repos.
- Add a Repomix metadata-only or token-tree benchmark so artifact sizing is
  available without generating a huge prompt candidate.
- Compare fresh-agent behavior with and without the scout packet by inspecting
  subsequent first reads and token use.
- Compare scout shadow records with later first reads and refine the merge rule
  before any model-visible automatic injection.
