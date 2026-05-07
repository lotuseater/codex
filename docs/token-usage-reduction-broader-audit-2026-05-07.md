# Token Usage Reduction: Broader Audit

Date: 2026-05-07

## Scope

This note answers the broader question behind the recent high weekly usage:

- Why token usage keeps growing faster than wall-clock reset progress even though prompt cache, operation cache, first-moves prediction, and smart compaction already exist.
- How to reduce tokens across all Codex conversations, not only this one session.
- How to improve live session and log discovery so finding the right PowerShell/session/log does not itself consume a large share of the conversation.
- What ideas are worth borrowing from Graphify, GSD2, SR2, Aspens, Codesight, and BMAD.

The active same-repo Codex build was still running while this note was written, so this file intentionally avoids touching shared Rust implementation files that another Codex instance is compiling.

## Local Findings

Observed live sessions:

- `open_ai/codex`: PowerShell PID `28516`, Codex PID `23672`, running `scripts\build-local-codex.ps1 -Mode FastRelease` with Cargo PID `24912`. The visible session showed roughly `67%` current-context usage and weekly usage around `38% used / 28% reset`.
- `Serial_to_Google_Doc_topdow`: PowerShell PID `16836` in earlier inspection, with a high current-context percentage and weekly usage around the same pressure band.
- This research session also grew quickly because broad session/log discovery and repo scans put large command outputs into the conversation.

Key cause:

Prompt caching is not enough. It can reduce latency and billable uncached input, but the active prompt can still be very large and repeated. Local weekly/rate-limit counters and context pressure are driven by the total request shape, including cached input, repeated system/project rules, conversation history, tool outputs, subagent prompts, and repeated exploration outputs.

The largest avoidable sources are:

- Broad `git status`, `rg`, log, and JSONL scans whose raw output enters history.
- Session discovery that searches too much before using known Codex log/session roots and live window metadata.
- Subagents and review sessions that start with large inherited context and then repeat repo exploration.
- Operation-cache hits that return useful command results but still place the result text into the model context.
- Late compaction, which helps after the conversation is already expensive rather than before the next task starts.

## Current Fork State

Relevant existing work already present in this checkout:

- `codex-rs/core/src/context_manager/prompt_elision.rs` elides repeated identical large tool outputs before prompt construction.
- `codex-rs/core/src/session/checkpoint_policy.rs` contains semantic compact triggers for continuations, work checkpoints, git commits, tool-call count, and early pressure.
- `codex-rs/operation-cache/src/lib.rs` provides a Wizard-backed per-project/system operation cache.
- `codex-rs/features/src/tests.rs` includes coverage showing deferred MCP tool search defaults are enabled for local token savings.
- `codex-rs/rollout/src/session_index.rs`, `codex-rs/rollout/src/list.rs`, and `codex-rs/thread-store/src/local/list_threads.rs` already provide pieces for fast thread/session listing.

The gap is that these pieces mostly reduce repeated work or repeated exact outputs. They do not yet create a small, task-scoped context pack before the next model request.

## External Tool Study

### Graphify

Source: <https://github.com/safishamsi/graphify>

Graphify turns a folder into `graphify-out/GRAPH_REPORT.md`, `graph.json`, and `graph.html`, then answers targeted graph queries instead of making the agent grep/read widely. Its README says code is extracted locally with tree-sitter and no API calls; docs and media can use model APIs when enabled.

Local test:

- Installed from the local clone with `uv tool install --force ...\graphify`.
- Built a bounded Codex context sample of 10 relevant Rust files.
- `graphify update ... --force` finished in about 11 seconds.
- Report: 10 files, about 16,026 words, 241 nodes, 449 edges, 10 communities, 98% extracted relationships.
- `graphify benchmark` on the sample reported about 16,066 naive tokens versus about 1,851 average query tokens: roughly `8.7x` fewer tokens per query.
- A query for pre-task narrowing/compaction found `core/src/session/checkpoint_policy.rs` and the relevant decision types with no need to read the whole sample.

Useful principle: maintain a durable graph/index, query it first, and cap the returned answer. This saves prompt tokens only when Codex reads the graph digest/query result instead of the source tree.

### GSD2

Source: <https://github.com/gsd-build/gsd-2>

GSD2's Context Mode is the closest match to the missing "chain action cache" idea. It steers agents toward `gsd_exec` for noisy scans, builds, tests, and diagnostics; stores full stdout/stderr and metadata under `.gsd/exec/`; and puts only a short digest into the conversation. It also has `gsd_exec_search` to reuse prior runs and `gsd_resume` to load a previous snapshot.

Useful principle: command chains must become persisted artifacts with handles, not pasted transcript text. The model sees a digest and can ask to expand a handle if needed.

### SR2

Source: <https://github.com/terminus-labs-ai/sr2>

SR2 treats context as a managed pipeline:

- Stable layers first for KV-cache friendliness.
- Raw recent turns, compacted middle turns, summarized older turns.
- Tool outputs replaced with line counts, samples, and recovery hints.
- File contents replaced with path references.
- Code execution results reduced to exit code plus a few lines.
- Pre-emptive rotation before emergency truncation.

Useful principle: compaction should be structural and continuous, not only a late emergency. The replacement must be recoverable by path, hash, command, and artifact id.

### Aspens

Source: <https://github.com/aspenkit/aspens>

Aspens replaces a large always-loaded instruction file with small scoped skills generated from an import graph. Its README positions the target shape as roughly 35 focused lines per domain, activated only when the agent touches that part of the codebase.

Useful principle: shrink always-on AGENTS/project rules. Load a tiny root index plus scoped rules selected by predicted files/domains.

### Codesight

Source: <https://github.com/Houseofmvps/codesight>

Codesight generates a persistent `.codesight/wiki/` from AST/framework detection. Its README describes a small `index.md` plus targeted articles such as `auth.md`, `database.md`, and `ui.md`; the intended pattern is reading the index at session start and one article for the task, not the full map.

Useful principle: a repo should expose a tiny context catalog, then task-specific articles with measured token counts and freshness checks.

### BMAD

Source: <https://github.com/bmad-code-org/BMAD-METHOD>

BMAD is not mainly a code graph tool; its token lesson is workflow shaping:

- Fresh chat per workflow/story to avoid context garbage.
- Story-centric implementation with focused context.
- `project-context.md` kept lean and loaded consistently.
- Large docs sharded by section.
- A distillator skill that turns source documents into dense, token-efficient distillates, with optional round-trip validation.

Useful principle: split work into durable story/context artifacts. The next agent should start from the story/context artifact, not from the entire planning conversation.

## Design Direction For This Fork

The high-probability fix is a three-layer context gate before every model request:

1. **Task Context Scout**
   - Runs locally before the model call when a fresh user task arrives.
   - Inputs: cwd, prompt, git state, known project roots, first-moves predictor, optional graph/wiki indexes.
   - Output: a small context pack with likely files, scoped rules, recent related sessions, and reusable artifacts.
   - Hard cap: target hundreds of tokens, not thousands.

2. **Artifact-Backed Tool Output**
   - Store full outputs for shell, search, log reads, session scans, builds, and tests in repo/system artifact files.
   - Put only a digest into conversation: command, cwd, exit code, elapsed time, bytes/lines, first/last lines, artifact id, sha.
   - Reuse by artifact id or query search instead of rerunning and repasting.
   - This is the chain-action cache the current single-action cache does not provide.

3. **Continuous Prompt Elision**
   - Extend prompt elision from "duplicate large outputs" to "large or noisy outputs after first useful view".
   - Replace old outputs with recoverable references even if they are not exact duplicates.
   - Preserve the current turn's most relevant output, but compact older scans/build logs/session listings aggressively.

## Session And Log Discovery Plan

Discovery should stop being a broad search problem.

Required fast path:

- Known Codex roots:
  - `$CODEX_HOME\sessions`
  - `$HOME\.codex\sessions`
  - repo-local `logs\`
  - rollout/thread-store indexes when present
- Build an index keyed by project root, session id, JSONL path, last write time, process id, window handle/title when available, and last visible cwd clue.
- For "latest in project X", check project-keyed index first, then recently modified JSONL/log files, then fall back to reading only the start/end slices for cwd clues.
- For live PowerShells, use DAB/window metadata when available: hwnd, process tree, title, cwd, visible text tail. The future internal DAB should be preferred over WizardErasmus, but the interface should keep the same high-level operations.
- Return a short table and handles; never paste full logs unless explicitly requested.

Suggested user-facing/internal commands:

- `codex sessions find --project <path> --latest --limit 3`
- `codex sessions tail --session <id> --bytes 4096`
- `codex sessions live --project <path>`
- `codex artifacts show <id> --lines 80`

## Implementation Slices

Safe order:

1. Add artifact-backed prompt output references for shell/search/log outputs.
   - New artifact writer module.
   - Extend prompt elision to replace large historical outputs with artifact refs.
   - Tests should assert digest text and recoverable metadata.

2. Add fast session/log discovery.
   - Reuse `rollout::session_index` and thread-store listing.
   - Add known-root scan and project-root matching.
   - Add optional DAB/live-window provider behind a trait so WizardErasmus and future internal DAB can share the same contract.

3. Add pre-task context scout.
   - Combine first-moves predictor with graph/wiki/context catalogs when present.
   - Produce a capped context pack before the first model call for a new task.
   - Include scoped AGENTS/skill snippets instead of the whole project instruction monolith where possible.

4. Add token ledger categories.
   - Track prompt sections separately: system/developer/project rules, history, tool outputs, file contents, subagents, cached input, uncached input.
   - This makes regressions visible when one category grows faster than useful work.

5. Add guardrails for subagents and reviews.
   - Do not fork full context by default when only a digest and file list are needed.
   - Pass artifact handles and targeted files, not the entire exploration transcript.

## Implemented In This Pass

- Added this audit note as a durable repo artifact.
- Added `scripts/find-codex-sessions.ps1` as an immediate non-Rust fast path for session/log discovery while another Codex instance was compiling the Rust tree.
- The script scans recent date-partitioned `$CODEX_HOME\sessions\yyyy\MM\dd` files, parses only bounded `session_meta` fields, optionally includes repo `logs\`, and can emit full JSON when exact paths are needed.
- Verified it finds recent `open_ai\codex` and `Serial_to_Google_Doc_topdown` sessions without dumping full JSONL contents into the prompt.

## Expected Impact

Most savings should come from reducing prompt context before it reaches the model:

- Graph/wiki/context scout: likely `5x-10x` fewer exploration tokens on repo-study turns, based on the local Graphify sample and Codesight-style targeted articles.
- Artifact-backed shell/session/log outputs: likely large savings in this workflow, because session discovery and logs were a visible high-cost source.
- Scoped instructions: medium recurring savings in every turn if large always-loaded AGENTS/project rules are split or summarized.
- Operation cache alone: useful for time and repeat execution, but low prompt-token savings unless the returned result is also digested or referenced.

The important rule: cache full data, but prompt only the handle plus a small, task-relevant digest.
