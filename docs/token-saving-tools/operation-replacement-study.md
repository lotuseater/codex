# Operation Replacement Study

Date: 2026-05-07

Scope: study token-saving opportunities beyond cache and first-moves by
replacing common Codex operations with better primitive operations.

## Sources Inspected

- Recent local Codex session artifacts under `C:\Users\Oleh\.codex\sessions`.
- `scripts/find-codex-sessions.ps1`.
- Current Codex tool code:
  - `codex-rs/core/src/tools/handlers/shell.rs`
  - `codex-rs/core/src/tools/handlers/unified_exec.rs`
  - `codex-rs/core/src/tools/context.rs`
  - `codex-rs/core/src/tools/operation_cache.rs`
  - `codex-rs/tools/src/local_tool.rs`
  - `codex-rs/app-server-protocol/src/protocol/common.rs`
  - `codex-rs/app-server-protocol/src/protocol/v2/fs.rs`
- Existing tool research docs in this folder.
- Local Serena clone:
  `C:\Users\Oleh\Documents\GitHub\agent-context-tools-lab\serena`.
- Other local tool clones under
  `C:\Users\Oleh\Documents\GitHub\agent-context-tools-lab`:
  Graphify, GSD2, SR2, Aspens, Codesight, and BMAD.
- Official docs for Aider repo map, Repomix, and Sourcegraph Cody context.
- Survey-only docs or announcements for Continue codebase retrieval,
  RepoPrompt, CodeGraphContext, and SymDex. These need local validation before
  they should influence Codex defaults.

## Recent Operation Mix

I analyzed 40 recent session JSONL files under 5 MB each. This intentionally
excluded very large sessions such as an 80 MB April 30 session so the scan would
stay bounded. The sample still covered 550 tool calls.

Observed operation classes:

| Operation class | Calls | Output chars | Output lines |
|---|---:|---:|---:|
| shell file reads | 220 | 1,652,452 | 31,586 |
| shell git inspect | 71 | 984,180 | 18,191 |
| shell text search | 69 | 1,244,638 | 11,755 |
| shell list/glob | 56 | 165,398 | 2,483 |
| shell build/test | 19 | 126,700 | 981 |
| first-moves calls | 18 | 62,349 | 52 |
| shell process/runtime | 12 | 93,377 | 1,396 |

Several outputs hit about 40,000 characters, which is consistent with a
truncation cap still being large enough to pollute context. The hot path is not
just "too many commands"; it is "too many broad shell commands whose result is
still text poured into the model".

Repeated command families included:

- `find-codex-sessions.ps1` for session discovery.
- `git status --short`, `git diff --stat`, and `git diff --name-only`.
- whole-file `Get-Content`.
- broad `rg` over large repo regions.
- PowerShell process inspection.

## Current Codex Operation Surface

The model-facing operation surface is still heavily shell-centered.

Facts from current code:

- `ShellCommandHandler` runs `shell_command`, then sends formatted shell output
  back to the model.
- `format_exec_output_str` and related helpers truncate output, but still return
  text in the conversation.
- `ExecCommandHandler` has `max_output_tokens`, chunk IDs, and structured output
  support. That helps, but it is still an exec primitive, not a semantic code or
  repo operation.
- `operation_cache` stores pre/post tool payloads by tool input/output, but it
  does not by itself replace old conversation context with small handles.
- The app-server protocol already has useful APIs such as `fuzzyFileSearch/*`,
  `fs/readFile`, and `fs/readDirectory`, but these are client/app APIs rather
  than the default model-facing code navigation tools.
- `ToolSearchHandler` searches deferred tool metadata, not project code.
- I did not find an active model-facing first-class code `read_file` or
  `grep_files` handler registered in `handlers/mod.rs`; recent sessions confirm
  the model falls back to shell for file reads and search.

## Tool Principles Beyond Serena

| Tool family | Mechanism | Codex operation it can replace | Token win hypothesis | Quality risk | Required test before default |
|---|---|---|---|---|---|
| Serena | LSP/IDE symbol tools: overview, find symbol, references, implementations, diagnostics | Whole-file reads, identifier `rg`, manual reference chasing | Return symbol slices instead of file bodies | Index/backend stale or unavailable | Compare against raw `rg` on known symbol/reference tasks |
| Graphify | Persistent graph, communities, wiki, graph query, content-hash cache | Architecture sweeps, cross-module `rg`, repeated orientation reads | Query a bounded graph packet instead of rediscovering topology | Graph may miss recent files or infer weak edges | Measure required-file recall on Codex architecture/debug tasks |
| Codesight | Deterministic map/wiki, framework detectors, blast-radius query, MCP wiki tools | Cold-start repo exploration, route/schema/config discovery, review blast-radius checks | Read tiny wiki index plus one article instead of full map or file sweep | Detectors may not match Codex's CLI/TUI shape | Generate on Codex sample and verify affected-file/test-lane hints |
| Aider repo map | Budgeted symbol map, signatures, dependency graph ranking, dynamic map size | Whole-repo orientation and "which file do I need?" steps | Show signatures and anchors under a strict map-token budget | Misses config/tests/macros and rare edge cases | Check map includes expected Rust symbols for golden tasks |
| RepoPrompt-style codemap | Selected files plus codemap for unselected files | Manual file selection and broad source packing | Keep exact source only for selected files, codemap elsewhere | UX idea, not yet a Codex-native operation | Prototype as context snapshot and compare selected-file recall |
| Repomix | Include/ignore profiles, token tree, Tree-sitter compression, secret checks | One-shot context snapshots, resume/review packs, token accounting | Compressed structural preview before full file reads | Whole-repo pack can be worse if read blindly | Use only scoped snapshots; verify token tree and secret-scan behavior |
| GSD2 | Fresh unit contexts, `gsd_exec` digest artifacts, `exec_search`, phase anchors, observation masking | Build/test logs, noisy process output, repeated diagnostics, long phase carryover | Store large output once and replay digest plus handle | Bad digest hides first actionable failure | Compare digest with raw output on failing build/test fixtures |
| SR2 | Context compiler, three-zone history, compaction recovery hints, cost gate, tool masking | Prompt assembly, old tool-output history, always-visible tool catalogs | Keep stable prefix cached; compact middle history; expose fewer tools | Over-compaction or tool masking can block valid work | Trace per-layer tokens and verify re-open-by-handle recovery |
| Aspens | Scoped skills from import graph, activation hooks, freshness/impact checks | Monolithic AGENTS/skill reads | Load one small shard instead of all instructions | Stale shard can encode wrong local rules | Path/task activation tests plus shard freshness lint |
| BMAD | Consumer-specific distillates, story artifacts, project context, optional round-trip validation | Long research/planning chains carried as raw chat | Replace old raw chain with dense artifact for the next phase | Distillate can omit evidence unless validated | Required-fact coverage checks for review/debug distillates |
| Sourcegraph Cody | Keyword search, Sourcegraph search, code graph, explicit context mentions | Broad search and code-relationship discovery | Hybrid retrieval can choose smaller relevant snippets | More context is sometimes needed for quality | Borrow retrieval mix; validate context-size vs answer correctness |
| Continue | Local embeddings, keyword search, rerank, ignore rules | Natural-language codebase questions and docs lookup | Retrieve and rerank only final snippets | Embeddings can miss exact identifiers | Use as optional lane in benchmark, not default |
| CodeGraphContext/SymDex | Persistent symbol graph, call graph, semantic search via MCP/CLI | Function/class lookup, call-chain discovery, impact analysis | Return symbol/call context in tens or hundreds of tokens | MCP tool overhead and incomplete graph can erase savings | Local Codex benchmark before adopting graph MCP patterns |

The strongest shared principle is not "add another search tool". It is to
change the default representation of context:

- file bodies become outlines, symbols, or slices,
- command logs become digests plus artifact handles,
- repo structure becomes a bounded map/wiki/query result,
- long conversation chains become consumer-specific distillates,
- global instructions become scoped shards,
- broad tool catalogs become task-state tool sets.

## Replacement Candidates

| Current operation | Better primitive | Use when | Keep raw shell when |
|---|---|---|---|
| `Get-Content file` | `read_file_slice` | User needs exact lines or a bounded excerpt | File is tiny or non-text handling is unusual |
| whole code file read | `file_outline` then `read_symbol` | Code file is large and task names a type/function/module | Bug likely depends on hidden implementation details |
| `rg -n identifier` | `find_symbol` or `find_references` | Query is an identifier, function, type, method, trait, enum | Search is natural language, comments, docs, config, or generated text |
| broad `rg` | `search_text` with grouping/caps | Search is still text-based but should be bounded | User explicitly asks for all matches |
| `rg --files` / recursive list | `find_file` / project file index | Looking for a path by name, extension, or subsystem | Filesystem state outside the project index matters |
| `git status --short` | `git_worktree_summary` | Need dirty/staged/untracked overview | User asks for exact native output |
| `git diff --stat` | `diff_summary` | Need changed files and blast radius | User asks for raw diffstat |
| `git diff` | `diff_hunks` / `changed_symbols` | Review or scoped implementation | Exact patch text is needed |
| build/test shell command | `run_check` | Need verification result | Command is interactive or custom beyond parser support |
| process PowerShell query | `process_find` | Need live PIDs/parents/commands | Need arbitrary OS inspection |
| session JSONL scan | `session_find` / `session_tail` / `session_stats` | Need recent/live Codex sessions | Deep forensic read of one known session |
| full skill doc read | `skill_select` / `skill_summary` | Need applicable guidance for current task/path | Creating or auditing the skill itself |

## Replacement Evaluation Gate

No replacement should become default just because it is more compact. Each
candidate must prove both token savings and quality against the current/raw
operation.

Rollout modes:

- `off`: current behavior only.
- `recommend`: detect a better operation and log the suggestion, but do not run
  it.
- `shadow`: run the replacement beside the current operation and compare
  outputs without changing model-visible behavior.
- `canary`: use the replacement for low-risk read-only cases and fall back on
  mismatch, stale index, low confidence, or explicit raw-output requests.
- `default`: use the replacement after it passes golden tasks.

Each `replacement_bench` record should include:

- baseline command or tool call,
- replacement operation and config,
- baseline model-visible tokens,
- replacement model-visible tokens,
- full-output artifact bytes,
- wall time,
- required facts found or missed,
- fallback reason if any,
- verdict: `pass`, `fail_quality`, `fail_tokens`, `fallback_required`, or
  `needs_human_review`.

Promotion threshold:

- no missed critical files, sessions, staged/unstaged states, failing tests, or
  review findings in golden tasks,
- exact detail recoverable through artifact handles or raw fallback,
- at least 30 percent fewer model-visible tokens for discovery/output-heavy
  operations, or a clear latency win for live session/process discovery,
- deterministic fallback when indexes are stale, confidence is low, output is
  truncated, or the user asks for exact native output.

## Prototype Implemented

Added `scripts/measure-operation-replacements.ps1` as a read-only benchmark
harness for candidate operation replacements. It emits `replacement_bench`
records for:

- `git_worktree_summary` versus raw `git status --short`, `git diff --stat`,
  and `git diff --name-only`.
- `session_find` versus broad recursive session scans.
- `search_text` versus raw `rg`.
- `file_outline` versus raw whole-file reads.
- `run_check_digest` versus raw noisy command/check output.

Also improved `scripts/find-codex-sessions.ps1` so session discovery handles
current Codex `session_meta.payload` records, includes recently modified older
session files, reads `state_5.sqlite` first when `sqlite3` is available,
returns `tokens_used` for indexed sessions, and uses a safe bounded tail read
for fallback matching.

Sample run on this dirty Codex checkout:

| Candidate | Verdict | Baseline tokens | Candidate tokens | Result |
|---|---|---:|---:|---|
| `git_worktree_summary` | `pass` | 92,308 | 193 | Saved about 99.8 percent while preserving the 1,469 changed-file set. |
| `search_text` for `first_moves` | `pass` | 2,820 | 1,445 | Saved about 48.8 percent with all 26 baseline files still represented. |
| `session_find` | `pass` | 212 | 210 | Quality passed after metadata/timestamp/state-db fixes; script wall time improved versus broad scan but still includes PowerShell/sqlite startup. |
| `file_outline` for `shell.rs` | `pass` | 6,952 | 969 | Saved about 86.1 percent while preserving all 58 detected definitions. |
| `file_outline` for `chatwidget.rs` | `pass` | 113,277 | 5,957 | Saved about 94.7 percent when the outline cap was raised to cover all 599 detected definitions. |
| `run_check_digest` | `pass` | 4,965 | 95 | Saved about 98.1 percent by storing full output under `logs/operation-replacement-artifacts` and returning diagnostics plus an artifact path. |

The session result is intentionally conservative: the script now proves the
indexed discovery shape and includes `tokens_used` from `state_5.sqlite`, but a
native Codex implementation should query the thread store directly or use
DAB/live-session lookup so the common path is milliseconds rather than paying
PowerShell/sqlite process startup.

The `chatwidget.rs` run also shows why replacement gates matter: with a cap of
500 outline items, the candidate saved tokens but omitted 99 definitions and was
correctly marked `fallback_required`. Raising the cap to 700 preserved all
detected definitions and still saved more than 94 percent of model-visible
tokens. Native `file_outline` should therefore record omitted counts and fall
back or ask for a narrower slice when a cap hides definitions.

The `run_check_digest` result is the concrete chain-cache pattern: full noisy
output is stored once as a durable artifact, while the model sees only status,
diagnostics, token counts, and a path/handle for reopening details. This is a
token win in a way that single-action output caching is not.

## Highest-Value New Codex Operations

### 1. `git_worktree_summary`

Returns structured fields:

- branch,
- changed file counts,
- staged/unstaged/untracked groups,
- generated/vendor/schema file grouping,
- top changed directories,
- commands to get exact raw output if needed.

Why first: recent sessions show `git status` and `git diff --stat` repeatedly
hit large output caps because this checkout is very dirty. A summary tool would
usually replace hundreds of lines with tens.

### 2. `diff_hunks`

Inputs:

- paths,
- staged/unstaged/both,
- context lines,
- max files,
- max hunks,
- optional pattern filter.

Output:

- changed symbols or hunk headers where available,
- short patch snippets,
- omitted counts,
- artifact handle for full diff.

This replaces repeated raw `git diff` calls during review.

### 3. `project_file_index`

Operations:

- `find_file(query, roots, limit)`,
- `list_dir_compact(path)`,
- `recent_files(root, limit)`,
- `files_by_extension(root, ext, limit)`.

Use the existing app-server fuzzy search design as a starting point, but expose
it as a model-facing internal tool with token caps.

### 4. `search_text`

A safer replacement for broad `rg`:

- grouped by file,
- max matches per file,
- max files,
- context lines disabled by default,
- common ignored dirs enforced,
- summary of omitted matches,
- optional artifact handle for full raw result.

This keeps exact text search available while preventing 40k-character result
blocks from entering the model.

### 5. `file_outline` And `read_file_slice`

`file_outline` returns language-aware structure:

- imports/use statements,
- public types/functions,
- test names,
- comments/doc headings,
- line ranges.

`read_file_slice` returns exact bounded lines by path/range with token estimate.

This should be implemented before full semantic LSP integration because it is
lower risk and useful for Rust, Markdown, PowerShell, JSON, and TOML.

### 6. `symbol_ops`

Serena-style operations for code:

- `symbol_overview(path)`,
- `find_symbol(name, roots, kind, limit)`,
- `read_symbol(path, symbol)`,
- `find_references(path, symbol, limit)`,
- `changed_symbols(diff)`.

Implementation choices:

- Start with a deterministic Rust parser/indexer or lightweight LSP bridge.
- Keep it in a new crate, for example `codex-rs/context-ops`, rather than
  growing `codex-core`.
- Return compact Markdown or typed structs, not large JSON.
- Fall back to `search_text` when the index is unavailable or stale.

### 7. `run_check`

Wrapper for tests/builds:

- runs command,
- stores full output artifact,
- extracts status, failing tests, compiler errors, warnings summary,
- returns short digest.

This is GSD2's `gsd_exec` idea as a native Codex operation.

### 8. `session_find`, `session_tail`, `session_stats`

Native replacement for repeated PowerShell session discovery:

- use known session root and date directories,
- sort by mtime,
- parse bounded `session_meta` and recent user/tool clues,
- return paths and handles,
- read full JSONL only on demand.

The existing `scripts/find-codex-sessions.ps1` is a good prototype.

### 9. `process_find`

Replacement for broad CIM/process dumps:

- filters by process name, cwd clue, command substring, window/session id,
- returns minimal fields,
- optionally uses DAB or Wizard session APIs for visible terminals.

## Routing Rules

Start with recommendations and telemetry before auto-rewriting commands.

Do not auto-route any replacement until it has passed the evaluation gate above.
Before that point, the classifier may only recommend or shadow-run candidates.

Suggested read-only routing:

- If the model asks for `git status --short`, route to `git_worktree_summary`.
- If it asks for `git diff --stat` or `git diff --name-only`, route to
  `diff_summary`.
- If it asks for `Get-Content` on a code file over a size threshold, route to
  `file_outline` and require a follow-up slice or symbol read.
- If it asks for `rg -n` with an identifier-like query in code roots, offer
  `find_symbol`/`find_references` first.
- If it asks for `rg --files` or recursive `Get-ChildItem`, route to
  `project_file_index`.
- If it asks for process/session discovery, route to `process_find` or
  `session_find`.
- If it asks for build/test, route to `run_check` unless the command is
  interactive or explicitly user-provided.

Do not auto-route mutating shell commands. The replacement work here is for
read-only discovery, inspection, and verification output shaping; mutating
commands remain explicit shell operations unless a separate safety design is
written and tested.

## Implementation Plan

Phase 1: telemetry-only classifier and benchmark schema.

- Add a read-only shell command classifier in the tool layer.
- Record operation class, output chars, output tokens, and replacement candidate.
- Add `replacement_bench` records and golden-task fixtures.
- Do not change behavior or model-visible output.

Phase 2: shadow replacements.

- Implement candidates behind `recommend` and `shadow` modes.
- Run candidate operations beside baseline shell/current behavior.
- Compare required facts, token counts, output size, and wall time.
- Keep baseline behavior model-visible.

Current native stage:

- `features.context_ops_shadow` is default-enabled in this fork.
- It shadows exact safe shell discovery patterns only, leaving the real shell
  output unchanged.
- It writes `replacement_bench` JSONL records and full baseline/replacement
  artifacts under `<codex_log_dir>/replacement-shadow/`.
- Initial candidates are `git_worktree_summary`, `search_text`, and
  `file_outline`.

Phase 3: low-risk typed tools.

- Implement `git_worktree_summary`, `diff_summary`, `project_file_index`,
  `search_text`, `read_file_slice`, and `session_find`.
- Keep outputs capped by design.
- Add tests using synthetic dirty repos and fixture session logs.

Phase 4: artifact-backed outputs.

- Pair `search_text`, `diff_hunks`, and `run_check` with artifact handles for
  full output.
- Older raw shell outputs should compact into artifact references.

Phase 5: semantic code operations.

- Add `file_outline` for Rust/Markdown/TOML/JSON/PowerShell first.
- Add `symbol_overview`, `find_symbol`, `read_symbol`, and `find_references`.
- Benchmark against raw `rg` and Serena on the same tasks.

Phase 6: guarded auto-routing.

- Auto-route only exact read-only command patterns with tested equivalence.
- Keep raw-shell escape hatches.
- Emit a trace explaining the substitution.

## Benchmarks To Run

Use these tasks:

- Review a dirty Codex worktree.
- Locate code handling tool output truncation.
- Find all call sites of a renamed Rust type.
- Find latest session for a project and summarize its token burn.
- Run a failing release check and identify the first actionable failure.

Compare lanes:

- current raw shell,
- typed operations only,
- typed operations plus artifact handles,
- Serena MCP if installed,
- Graphify/Codesight-style index query where applicable.

Metrics:

- total tool calls,
- output chars sent to model,
- estimated prompt tokens,
- elapsed time,
- relevant files found,
- missed findings,
- final answer quality.

## Recommendation

Do not start by adding a full Serena dependency to Codex. Borrow the operation
shapes first. The fastest local win is to replace high-volume shell patterns
with native typed operations:

1. `git_worktree_summary`
2. `diff_summary` / `diff_hunks`
3. `project_file_index`
4. `search_text`
5. `read_file_slice` / `file_outline`
6. `session_find`
7. `run_check`

Then add semantic symbol operations after the lower-risk primitives and
benchmark harness are in place.
