# Codex Fork Feature Improvement Plan

Date: 2026-06-11
Branch: `claude-automation-toolkit`
Primary sources: `FORK_FEATURES_REVIEW.md`, `docs/fork-feature-inventory.md`, latest Claude session `dfb70c66-afc5-40c0-a675-af646f3e6340.jsonl`, and read-only worker handoffs from 2026-06-11.

## Goals

This plan resumes the fork-improvement work after the upstream merge and turns it into a measurable, staged implementation program.

The first implementation slice is:

1. Fix the terminal-blocking Responses API error:
   `Invalid Value: 'tools.namespace'. User-defined namespace 'web' collides with an existing tool namespace.`
2. Build an extensive prompt-economics research program in `context-reducer-lab`.
3. Use that lab to evaluate future-action optimization and batch/mini-programming prompt variants before promoting any default runtime prompt.
4. Keep improving all fork-specific feature families with the same pattern: variants, representative tasks, explicit metrics, targeted tests, release build, deploy, smoke prompt, then promote the verified binary to the Codex UI app path.

The plan optimizes for long-term fork health: high-value behavior preserved, merge conflicts reduced, prompt/runtime changes measured before promotion, and every major improvement ending in a usable local Codex binary.

## Current State

The 2026-06-10 review found 11 existing feature families. Ten are `needs-work`; operation cache is `at-risk`. The architecture is usually sound: owner crates and thin adapters are the right direction. The main failures are operational:

- fork wiring silently severed by upstream merges;
- prompt/runtime experiments promoted or discarded from too-narrow evidence;
- test safety net broken by deferred `cfg(test)` debt;
- merge-surface reduction unfinished;
- blocking I/O left on async or UI hot paths;
- docs and inventory stale enough to mislead future merge repair.

This plan adds a 12th family, `prompt-economics`, covering:

- future-action optimization instructions;
- batch/mini-programming guidance;
- prompt-simulated decision quality;
- work-per-token and work-per-second measurement;
- lab-first promotion gates for any prompt injection.

## Feature Families And Improvement Strategy

| # | Feature family | Current state | Improvement strategy |
|---|---|---|---|
| 1 | Context budget and token-saving routing | needs-work | Move blocking context-pack work off async paths; finish or remove `ForkFeaturesState`; collapse duplicated compaction domain types; add direct compaction-decision tests. |
| 2 | Operation cache | at-risk | Restore runtime integration behind a tiny seam; add wiring-guard tests; make test script fail on zero matched tests; refresh stale status docs. |
| 3 | TUI fork UX | needs-work | Fix missing snapshots/scripts; reduce placeholder label noise; normalize session-limit keys; coalesce inactive-agent deltas. |
| 4 | `tui-render` extraction | needs-work | Port missing upstream markdown/table behavior; revive tests/dev-deps; delete stale snapshots; script port passes. |
| 5 | Self-review and task-memory | needs-work | Restore or retire plan checkpoint; reorder token-pressure gates; extract repeated compaction preservation helper; move git capture off UI path. |
| 6 | Multi-agent v2 and blackboard | needs-work | Delete duplicate specs; add behavioral lifecycle tests; move collab event deltas to fork-owned sibling modules; fix blocking blackboard I/O. |
| 7 | Experimental lanes | needs-work | Restore first-moves hit hook; decide replacement-shadow by measurement; fix/remove scout tool handler; move first-moves prediction to blocking pool. |
| 8 | Desktop Automation Bridge | needs-work | Kill timed-out bridge children; validate click coordinates; delete dead spec duplicate; decide mutating-tool approval model. |
| 9 | Owner crates and config family | needs-work | Finish dependency-boundary reductions; fix turn-diff drift; remove speculative unused port crates. |
| 10 | Server-side analytics/app-server/protocol | needs-work | Fix non-compiling tests; guard analytics sync hazards; reduce merge pressure through upstreamable pure code motion. |
| 11 | Build/test automation and relocated tests | needs-work | Repair final Phase E test estate; make vacuous tests fail; refresh merge metrics and taxonomy. |
| 12 | Prompt-economics | new | Build deterministic and live benchmark harnesses; evaluate prompt/action/batch variants on real task families; promote only measured winners. |

Every feature family must be evaluated with the same lens:

- What variants exist?
- What task suite actually represents the feature?
- Which metrics decide whether a variant improves the fork?
- Which wiring guard catches silent merge loss?
- Which verification lane is release-safe on this Windows machine?
- Does the feature deserve runtime promotion, shadow-only use, explicit-tool use, or removal?

## Wave 0: Terminal Unblock And Planning Base

### Namespace Collision Fix

Problem: the Responses API rejects a request containing hosted `web_search` plus a user-defined namespace named `web`. Local source showed dynamic and MCP namespaces were serialized as `ToolSpec::Namespace` without a hosted-tool reserved namespace policy.

Implementation:

- Add a central model-visible namespace alias policy in the tool-spec assembly path.
- Reserve `web` whenever hosted `web_search` is present.
- If an external MCP/dynamic namespace collides, expose it to the model as a deterministic alias such as `codex_ext_web`.
- Dispatch the aliased model call back through the original source namespace so dynamic tool providers and MCP handlers do not need to rename themselves.
- Apply aliasing before direct specs are merged and before deferred `tool_search` entries are exposed.

Acceptance:

- A dynamic tool with namespace `web` and hosted `web_search` produces model-visible `codex_ext_web`, not `web`.
- Deferred search output also uses `codex_ext_web`.
- Runtime registry dispatches by the model-visible alias, while source handlers call their original backend names.
- Non-conflicting namespaces are unchanged.
- `namespace_tools = false` behavior is unchanged.

### Long Plan Doc

This document is the Wave 0 planning artifact. It must be kept current as research and implementation waves complete.

### Baseline And Wave 42 Update

Status as of the Wave 42 fan-out:

- Codex commit `9aa9371b4a` (`Fix hosted web namespace collisions`) is the Wave 0 namespace/doc baseline. Hosted `web_search` reserves the model-visible `web` namespace, external `web` namespaces are exposed through deterministic aliases such as `codex_ext_web`, alias calls dispatch back to the source namespace, deferred `tool_search` descriptions use the alias, and this long plan exists.
- Lab commit `083d87c` in `context-reducer-lab` (`Add prompt economics comparison harness`) is the Wave 1 deterministic prompt-economics baseline. It adds the comparison harness, 39 prompt variants, 13 representative tasks, and `reports/prompt-economics-compare-2026-06-11.{json,md}`. The deterministic report nominates `action_route_selection` for live-model extension, but it does not promote a runtime default.
- Wave 41 `codex exec` audit workers were useful as an orchestration canary, not as full audit workers: nested exec sessions started and repeatedly logged hosted-web aliasing from `web` to `codex_ext_web`, but they lacked usable local shell/file tools for source inspection. Until that worker surface is repaired, use visible worktrees or normal spawned workers for mutating and source-review lanes.
- Wave 42 uses four mutating worktrees plus review gates: namespace hardening in `C:\w\c42n`, action-optimization config canary work in `C:\w\c42a`, lab live-manifest research in `C:\w\l42b`, and this plan-doc sync in `C:\w\c42p`; a separate read-only critic reviews overlap and self-review evidence, and one verifier owns expensive checks.

The research-first rule still holds: no prompt becomes a runtime default until the deterministic candidate survives live/sandbox gates, verifier-owned checks, and the normal release/deploy/smoke promotion cadence.

## Wave 1: Prompt-Economics Lab

Prompt-economics must be research-first. No prompt injection becomes a default because it sounds good.

### Deterministic Harness

Add a deterministic harness in `context-reducer-lab` that evaluates prompt variants against frozen task descriptions without requiring live model calls. This is the cheap canary layer.

The deterministic layer must record:

- prompt variant id and family;
- prompt token estimate;
- expected action signals covered;
- forbidden action signals triggered;
- schema regression risk;
- overbatch risk;
- overplanning risk;
- correctness, efficiency, safety, ergonomics, and composite scores;
- verified milestones per 1k prompt tokens.

The deterministic layer does not replace live evaluation. It filters obviously bad prompt shapes and produces a manifest for live Codex runs.

### Live-Model Extension

After the deterministic harness is stable, add a live run driver that renders each `(variant, task)` pair into a frozen prompt, runs Codex in a disposable fixture workspace, captures JSONL/tool traces, and scores actual behavior.

Live metrics:

- correctness and rubric quality;
- verified milestones completed;
- wall time;
- input/output/total tokens;
- tool call count;
- failed tool calls;
- invalid `workflow_batch` schemas;
- repeated file reads;
- user-visible planning paragraphs;
- unnecessary broad-scan rate;
- recovery cost after a bad first action;
- destructive-action avoidance;
- permission-boundary recognition;
- overbatch opacity;
- underbatch repetitive-loop rate;
- whether simple tasks stay simple.

### Batch-Programming Variants

The batch/mini-programming matrix must include at least 20 variants:

- current runtime text;
- no guidance;
- terse local work;
- balanced diagnosable batching;
- work-per-step batching;
- aggressive batching;
- anti-overbatching;
- shell-first;
- Python-first;
- workflow-batch-first;
- full schema;
- schema with compact example;
- no schema;
- positive-only guidance;
- positive plus negative guidance;
- long-task planner;
- minimal one-liner;
- reducer-first;
- assertion-first;
- wait-utilization;
- delegation-aware;
- permission-safe;
- report-artifact;
- one-recovery-only after bad schema.

Prompt ablations:

- exact schema present vs omitted;
- examples vs no examples;
- short vs medium vs long wording;
- positive-only vs positive plus anti-patterns;
- batch-only vs action-only vs combined prompts;
- root-confined wording vs generic local-work wording;
- explicit `never include response_length` vs no schema warning.

Promotion gates:

- no correctness drop against current guidance;
- zero invalid `workflow_batch` schema calls on schema canaries;
- no direct-answer regression;
- no destructive or permission-boundary overbatching;
- at least 15% lower cost per verified milestone or 25% better verified milestones per tool call;
- if tokens increase, verified milestone efficiency must improve by at least 20%.

### Future-Action Optimization Variants

The action-optimization matrix must include at least 12 variants:

- balanced verified-progress instruction;
- route-selection instruction;
- no-loop instruction;
- long-task strategy;
- minimal action optimizer;
- conservative anti-overthinking;
- verification-first;
- delegation split;
- budget-aware probe;
- wait-utilization;
- prototype-first;
- simple-task guard.

Promotion gates:

- simple tasks remain direct and short;
- concrete bugs still reproduce first;
- long tasks reduce repeated reads/tool loops;
- output does not become visible planning noise;
- no recursive delegation or broad scans when a focused probe is enough.

### Representative Task Suite

The prompt-economics task suite must cover:

- namespace bug triage;
- multi-file Rust refactor;
- failing test repair;
- repeated log/file reduction;
- feature-plan synthesis;
- GUI/live-session triage;
- cross-repo prototype promotion;
- audit findings with line refs;
- simple direct-answer control;
- one-off symbol search;
- `workflow_batch` schema canary;
- destructive-action boundary;
- long build wait utilization.

Later live suites should add full fixture workspaces for:

- real Rust source edits with targeted tests;
- docs-only planning/reporting;
- no-environment prompt-only tasks;
- dynamic/MCP tool availability tasks;
- parallel-worker orchestration tasks.

## Wave 2: Prompt Promotion

Only after Wave 1 reports are reviewed:

- update `BatchMiniProgrammingInstructions::body()` if a batch variant wins;
- add `ActionOptimizationInstructions` behind a disabled-by-default config canary;
- keep action optimization default off until the live model run passes;
- if promoted, choose the narrowest trigger, likely Plan mode or first-turn only;
- do not duplicate `workflow_batch` schema details in the action-optimization prompt.

Config shape:

```toml
action_optimization_instructions = {
  mode = "off", # off | plan | first_turn | tool_turn | always
  variant = "balanced",
  max_tokens = 120
}
```

Implementation seams:

- TOML config in `codex-rs/config/src/config_toml.rs`;
- resolved config in `codex-rs/core/src/config`;
- feature flag in `codex-rs/features`;
- context fragment in `codex-rs/core/src/context/action_optimization_instructions.rs`;
- injection in `build_initial_context`, after collaboration-mode instructions and before batch mini-programming instructions.

## Waves 3-8: Feature Recovery And Improvement

### Wave 3: P0 Wiring Guards

Restore or deliberately remove features that are currently severed or dead:

- operation-cache runtime integration;
- first-moves hit-recording hook;
- plan self-review checkpoint;
- repo-context-scout tool handler;
- replacement-shadow feature flags and runtime interception.

For each feature, choose one of:

- live runtime;
- shadow-only;
- explicit tool only;
- remove from workspace/config.

No feature should remain as dead code with green scripts.

### Wave 4: TUI And Render Correctness

- Port missing upstream markdown/table rendering into `tui-render`.
- Fix `tui-render` dev-deps and snapshot test estate.
- Repair TUI session-limit footer script/snapshots.
- Normalize rate-limit ids.
- Coalesce inactive-agent output deltas.

### Wave 5: Async And Hot-Path Performance

- Move context-pack render to `spawn_blocking`.
- Move first-moves prediction scans/prewarm/shadow append to blocking pool.
- Move blackboard git/fs work off async hot paths.
- Move `GitReviewAnchor::capture` off the TUI thread.
- Reorder task-memory gates to avoid O(history) serialization below pressure.
- Fix turn-diff multi-environment drift.

### Wave 6: Structural Cleanup

- Finish or remove `ForkFeaturesState`.
- Retire fork-resurrected `Op::UserTurn`.
- Collapse compaction domain type duplication.
- Delete duplicate `agent_tool.rs`, DAB spec duplicate, and Cognos duplicate.
- Remove dead speculative port crates or wire real consumers.

### Wave 7: Merge Automation And Upstream PRs

- Refresh `docs/fork-feature-inventory.md`.
- Refresh merge metrics and taxonomy.
- Add scripted port passes for extracted/relocated code.
- Open upstream PRs only for pure code-motion or interface-boundary splits.
- Keep upstream PRs free of fork behavior and AI co-author trailers.

### Wave 8: Final Phase E Test Repair

- Repair `cfg(test)` and `--tests` compile estate after feature recovery is stable.
- Work per crate with release-safe lanes.
- End with final release build, deploy, smoke prompt, and UI-app promotion.

## Verification And Promotion Cadence

Before every major wave:

```powershell
git status --short --branch
git fetch origin upstream
powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode Status
```

Avoid broad debug Cargo lanes. Prefer:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package <crate> -Filter <filter>
powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode LowMemRelease -Jobs 1
```

After each major improvement:

1. Run targeted release tests for the touched surface.
2. Run a LowMemRelease or FastRelease build, depending on current memory/disk state.
3. Deploy via the local build script.
4. Smoke-test the deployed `codex.exe` with at least one prompt relevant to the changed behavior.
5. Promote the verified binary to the Codex UI app usage path.
6. Commit only the coherent verified slice.
7. Push only if origin is not ahead and the user requested or the branch cadence requires it.

The UI promotion step must preserve the copy-first / validate-before-promotion rule: patch/build a copy, test it, then update wrapper/UI app routing to use the verified binary.

## Worker Split

Use spawned Codex workers for bounded, non-overlapping ownership:

- namespace/tool-registry worker;
- prompt-economics lab worker;
- operation-cache/first-moves wiring worker;
- TUI/render worker;
- DAB worker;
- async hot-path worker;
- structural cleanup worker;
- docs/merge automation worker;
- Phase E per-crate test workers.

Root session owns:

- integration decisions;
- applying cross-worker patches;
- final tests;
- release build/deploy;
- UI promotion;
- commit boundaries.

Worker prompts must include:

- exact allowed files/modules;
- exact verification command;
- no reverting unrelated changes;
- concise handoff with changed files, risks, and test result;
- stop condition when ownership boundary is reached.

## Dirty Worktree Rules

- Do not revert unrelated user changes.
- Stage only explicit pathspecs for the current verified slice.
- In `context-reducer-lab`, preserve the existing report/artifact noise; own only new prompt-economics source/report files.
- If an unrelated dirty file overlaps a target file, read and integrate around it.
- Commit doc/lab/runtime slices separately unless they are part of one verified wave.

## Rollback

If a deployed binary is bad:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode Rollback
```

If source is bad:

- revert only the bad wave commit;
- rebuild and redeploy the previous known-good state;
- preserve user/unrelated changes;
- document the failed verification and replacement approach.

## Acceptance For This Plan

Wave 0 is complete when:

- `tools.namespace = web` no longer blocks model requests with hosted web search;
- focused namespace tests pass;
- prompt-economics lab harness runs and emits JSON/Markdown reports;
- this plan exists in docs;
- a release build is deployed, smoke-tested, and promoted to Codex UI usage.

The full objective is complete when all waves have shipped through the same verify/build/deploy/promote cadence and every fork feature is either live with tests, shadow/tool-only with documented gates, or deliberately removed.
