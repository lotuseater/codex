---
name: prototype-first-automation
description: Use when a small script, canary, fixture runner, or external prototype would make risky tool/runtime work, repeated checks, GUI/DAB testing, parsers/reducers, migrations, or expensive build/test loops faster and safer before changing the main repo path.
---

# Prototype-First Automation

Use this skill before work that is likely to benefit from a tiny separate program or script.

## Trigger

Consider a prototype first when any of these are true:

- You are changing tool/runtime behavior: DAB, hooks, skills, MCP, first-moves, repo-context-scout, cache, shadow/reducer operations, MultiAgentV2 loop or supervision.
- You need GUI/live verification: windows, screenshots, OCR, visible terminals, app smoke tests, or real-data UI checks.
- Verification is expensive: Codex release builds, CMake/Ninja builds, wrapper deployment, broad Cargo tests, or repeated build-status checks.
- The work is systematic: multi-file migration, repeated mechanical refactor, parser/reducer logic, or current-vs-alternative comparisons.
- Token or context use is the point: exploration agents, first_moves, cache hits, output truncation, context reducers, or command-output compaction.
- The same shell command family has been run several times, or the same command family failed twice.

Skip this for one-off exact reads/searches, single `git status`/`git diff`/`git log`, commit/push finalization, tiny docs edits, or final verification already using a working canary.

## Workflow

1. Estimate ROI: `risk + repetition + verification_cost + reuse_potential + token_savings - prototype_cost - noise_penalty`.
2. If ROI is positive, build the smallest separate proof first:
   - Codex tool experiments default to `C:\Users\Oleh\Documents\GitHub\context-reducer-lab`.
   - Repo-local apps can use a tiny fixture script or smoke harness beside the app tests.
   - Keep the prototype deterministic, CLI-rerunnable, and narrow.
3. Run the prototype, capture the exact command and observed result, and fix it there before copying logic into the main repo.
4. Port only the proven minimal logic into Codex or the target app.
5. Report whether the prototype should be promoted into a durable script, skill, hook, or Codex code change.

## Handoff

Include:

- prototype path;
- rerun command;
- observed output or artifact path;
- logic copied into the main change;
- what was discarded;
- promotion candidate, if any.
