# GSD2 Token-Saving Research

Source:

- Local clone: `C:\Users\Oleh\Documents\GitHub\agent-context-tools-lab\gsd-2`
- Upstream: https://github.com/gsd-build/gsd-2
- Local status: cloned and source/docs inspected; not fully installed or run
  in this pass.

## Key Ideas

GSD2 treats an agent workflow as units of work with explicit state, cost, and
context boundaries. Its biggest token-saving idea is that each unit starts with
a clean context and receives only the files/artifacts it needs.

Important mechanisms:

- Fresh session per unit, research phase, planning phase, or task.
- Context Mode, enabled by default, that injects task-ready context.
- `gsd_exec` for noisy commands; full stdout/stderr is saved on disk while only
  a short digest enters the conversation.
- `gsd_exec_search` to reuse previous command results instead of rerunning.
- `gsd_resume` to load a prior compaction snapshot.
- Worktree-scoped database for state, cost, tokens, sessions, and recovery.
- Token profiles: budget, balanced, and quality.
- Budget ceilings and cost projections.
- Headless query command for instant JSON state without starting an LLM.

## How It Works

GSD2's workflow engine decomposes larger work into units and persists state in a
project-local database plus artifact directories such as `.gsd/exec/`. The
agent is steered to run expensive shell searches, builds, and tests through GSD
tools. Those tools keep the full result outside the LLM context and inject a
digest with metadata and a path handle.

This is the important distinction for Codex: single-action caches do not save
tokens if the cached action output is still pasted back into every future
prompt. GSD2 saves tokens because it changes the representation of command
history from "large text replay" to "short digest plus durable pointer".

## Evidence From Source Review

The README and docs describe:

- Context Mode guidance and persisted exec outputs.
- Tunable stdout caps, digest length, timeouts, and environment allowlist.
- Token/cost fields in headless output types.
- Claimed 40-60 percent reduction for coordinated token profiles.
- Claimed 65 percent or better reduction for tiered context injection.

I did not run GSD2 end to end on the Codex repo in this pass, so these numbers
remain vendor/project claims until measured locally.

## What Codex Should Take

Codex should borrow the artifact-backed chain cache pattern directly.

Useful design elements:

- Store large command outputs, file reads, session-log excerpts, and build logs
  as durable artifacts.
- Inject short digests and stable handles into the conversation, not the full
  output.
- Let future turns request an artifact by handle, byte range, line range, or
  search query.
- Reuse prior command artifacts when command, cwd, environment fingerprint, and
  relevant file mtimes/hashes match.
- Track per-turn and per-phase token/cost in local telemetry.
- Use explicit task/unit boundaries to avoid carrying stale exploratory text
  into implementation and verification.

## Risks And Gaps

- Bad digests can hide failures. Codex needs "open full artifact" escape hatches.
- Command-result reuse must account for changed files and environment.
- A fresh session per unit can lose useful nuance unless the handoff artifact is
  structured and reviewed.
- Too many handles can create navigation friction if the UI does not surface
  them well.

## Codex Implementation Candidates

1. Add a `tool_artifacts` store for large shell/file/tool outputs.
2. Replace large repeated tool outputs in history with
   `{artifact_id, digest, token_estimate, source, replay_hint}`.
3. Add internal tools for `artifact_read`, `artifact_search`, and
   `artifact_list_recent`.
4. Extend prompt elision so older large outputs are replaced with digest
   handles automatically, not only when exact duplicates occur.
5. Add a per-session token ledger grouped by user turn, tool name, and output
   artifact class.
6. Add a benchmark lane where the same review/debug task runs raw versus
   artifact-backed and compares total prompt tokens.
